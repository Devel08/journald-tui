use journald_tui::controller::Controller;
use journald_tui::model::JournalEntry;
use journald_tui::parser::start_log_stream;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // We'll currently handle only 100 logs
    let (tx, rx) = tokio::sync::mpsc::channel::<JournalEntry>(100);
    std::thread::spawn(move || {
        start_log_stream(tx);
    });

    let mut controller = Controller::new(rx);

    let mut terminal = controller.init()?;
    let res = controller.run(&mut terminal).await;
    controller.cleanup(&mut terminal)?;

    res
}
