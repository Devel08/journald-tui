use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use crate::model::JournalEntry;
use systemd::journal::OpenOptions;

use crate::controller::matches_search_query;

fn to_entry(entry: &std::collections::BTreeMap<String, String>, timestamp: String) -> JournalEntry {
    let get = |key: &str| entry.get(key).cloned();

    JournalEntry {
        timestamp,
        message: get("MESSAGE").unwrap_or_default(),
        systemd_unit: get("_SYSTEMD_UNIT"),
        process_name: get("_COMM"),
        priority: get("PRIORITY"),
        pid: get("_PID"),
        hostname: get("_HOSTNAME"),
    }
}

pub fn start_log_stream(
    tx: tokio::sync::mpsc::Sender<JournalEntry>,
    global_filter: Arc<Mutex<Option<String>>>,
) {
    let mut journal = match OpenOptions::default().open() {
        Ok(journal) => journal,
        Err(err) => {
            eprintln!("Failed to open systemd journal: {err}");
            return;
        }
    };

    let mut current_filter: Option<Option<String>> = None;

    loop {
        let target_filter = global_filter.lock().unwrap().clone();

        if current_filter != Some(target_filter.clone()) {
            current_filter = Some(target_filter.clone());

            if target_filter.is_some() {
                if let Err(err) = journal.seek_head() {
                    eprintln!("Failed to seek to journal head: {err}");
                    return;
                }
            } else {
                if let Err(err) = journal.seek_tail() {
                    eprintln!("Failed to seek to journal tail: {err}");
                    return;
                }
                for _ in 0..101 {
                    if !matches!(journal.previous(), Ok(1)) {
                        break;
                    }
                }
            }
        }

        let entry_res = match journal.next_entry() {
            Ok(Some(entry)) => Ok(Some(entry)),
            Ok(None) => journal.await_next_entry(Some(Duration::from_millis(50))),
            Err(err) => Err(err),
        };

        match entry_res {
            Ok(Some(entry)) => {
                let should_send = match &target_filter {
                    Some(query) => {
                        let msg = entry.get("MESSAGE").map(String::as_str).unwrap_or("");
                        let proc = entry
                            .get("SYSLOG_IDENTIFIER")
                            .or_else(|| entry.get("_COMM"))
                            .map(String::as_str);
                        let unit = entry.get("_SYSTEMD_UNIT").map(String::as_str);
                        let hostname = entry.get("_HOSTNAME").map(String::as_str);
                        matches_search_query(msg, proc, unit, hostname, query)
                    }

                    None => true,
                };

                if should_send {
                    let timestamp = journal
                        .timestamp()
                        .ok()
                        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                        .map(|d| d.as_micros().to_string())
                        .unwrap_or_default();

                    if tx.blocking_send(to_entry(&entry, timestamp)).is_err() {
                        break;
                    }
                }
            }
            Ok(None) => continue,
            Err(err) => {
                eprintln!("Journal error: {err}");
                break;
            }
        }
    }
}
