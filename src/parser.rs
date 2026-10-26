use crate::model::JournalEntry;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

/// Start journalctl and stream the parsed json and send it to the mpsc
pub async fn start_log_stream(tx: tokio::sync::mpsc::Sender<JournalEntry>) {
    // Stream the last 100 entries
    let mut child = Command::new("journalctl")
        .args(["-o", "json", "--follow", "-n", "100"])
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to start journalctl process");

    let stdout = child.stdout.take().expect("Failed to open stdout pipe");

    let mut reader = BufReader::new(stdout).lines();

    // Whenever an entry pops up, send it to the mpsc
    while let Ok(Some(line)) = reader.next_line().await {
        if let Ok(entry) = serde_json::from_str::<JournalEntry>(&line) {
            let _ = tx.send(entry).await;
        }
    }
}
