use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::model::JournalEntry;
use systemd::journal::OpenOptions;

fn to_entry(entry: &std::collections::BTreeMap<String, String>) -> JournalEntry {
    let get = |key: &str| entry.get(key).cloned();

    JournalEntry {
        timestamp: get("__REALTIME_TIMESTAMP").unwrap_or_default(),
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
                    Some(query) => entry.get("MESSAGE").is_some_and(|m| m.contains(query)),
                    None => true,
                };

                if should_send && tx.blocking_send(to_entry(&entry)).is_err() {
                    break;
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
