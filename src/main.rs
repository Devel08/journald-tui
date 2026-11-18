use journald_tui::controller::Controller;
use journald_tui::model::JournalEntry;
use journald_tui::parser::start_log_stream;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (tx, rx) = tokio::sync::mpsc::channel::<JournalEntry>(100);

    let mut controller = Controller::new(rx);
    let global_filter = controller.global_filter();

    std::thread::spawn(move || {
        start_log_stream(tx, global_filter);
    });

    let mut terminal = controller.init()?;
    let res = controller.run(&mut terminal).await;
    controller.cleanup(&mut terminal)?;

    res
}
