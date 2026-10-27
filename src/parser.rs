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

pub fn start_log_stream(tx: tokio::sync::mpsc::Sender<JournalEntry>) {
    let mut journal = match OpenOptions::default().open() {
        Ok(journal) => journal,
        Err(err) => {
            //TODO: .map_err() that error to something
            // when we implement error handling
            // printing because of clippy
            eprintln!("Failed to open systemd journal: {err}");
            return;
        }
    };

    if let Err(err) = journal.seek_tail() {
        //TODO: .map_err() that error to something
        // when we implement error handling
        // printing because of clippy
        eprintln!("Failed to seek to journal tail: {err}");
        return;
    }

    // TODO: parse with clap-rs
    for _ in 0..100 {
        if !matches!(journal.previous(), Ok(1)) {
            break;
        }
    }

    loop {
        let entry_res = match journal.next_entry() {
            Ok(Some(entry)) => Ok(Some(entry)),
            Ok(None) => journal.await_next_entry(None),
            Err(err) => Err(err),
        };

        match entry_res {
            Ok(Some(entry)) => {
                if tx.blocking_send(to_entry(&entry)).is_err() {
                    //TODO: map error when error handling
                    break;
                }
            }
            Ok(None) => continue,
            Err(err) => {
                //TODO: map error when error handling
                eprintln!("Journal error: {err}");
                break;
            }
        }
    }
}
