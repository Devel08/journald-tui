use crate::model::JournalEntry;

use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Cell, Row, Table},
};
use std::io::{self, Stdout};
use std::time::Duration;
use tokio::sync::mpsc::Receiver;

pub struct Controller {
    logs: Vec<JournalEntry>,
    rx: Receiver<JournalEntry>,
}

impl Controller {
    pub fn new(rx: Receiver<JournalEntry>) -> Self {
        Self {
            logs: Vec::new(),
            rx,
        }
    }

    pub fn init(&self) -> Result<Terminal<CrosstermBackend<Stdout>>, Box<dyn std::error::Error>> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(terminal)
    }

    pub fn cleanup(
        &self,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
        terminal.show_cursor()?;
        Ok(())
    }

    pub async fn run(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            while let Ok(entry) = self.rx.try_recv() {
                self.logs.push(entry);
                // Maybe we should parse that with clap-rs
                // and make 30 default if argument is not provided
                if self.logs.len() > 30 {
                    self.logs.remove(0);
                }
            }

            terminal.draw(|f| {
                let size = f.area();

                //Make rows out of the logs data
                let mut rows = Vec::new();
                for log in &self.logs {
                    let unit = log.systemd_unit.as_deref().unwrap_or("unknown");
                    let process = log.process_name.as_deref().unwrap_or("unknown");
                    let message = &log.message;

                    rows.push(Row::new(vec![
                        Cell::from(unit.to_string()),
                        Cell::from(process.to_string()),
                        Cell::from(message.to_string()),
                    ]));
                }

                let header_row = Row::new(vec![
                    Cell::from("UNIT"),
                    Cell::from("PROCESS"),
                    Cell::from("MESSAGE"),
                ])
                .style(Style::default().bg(Color::LightBlue).fg(Color::Black));

                let widths = [
                    Constraint::Length(30),
                    Constraint::Length(30),
                    Constraint::Min(10),
                ];

                let block = Block::default()
                    .title(format!(
                        " journald-tui | Showing last {} logs ",
                        self.logs.len()
                    ))
                    .borders(Borders::ALL);

                let table = Table::new(rows, widths).header(header_row).block(block);

                //The table will be 33 rows (for 30 entries), so that the left out space can
                //be used for other stuff, like metadata when clicked on
                //some log
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(33), Constraint::Min(0)])
                    .split(size);

                f.render_widget(table, chunks[0]);
            })?;

            if event::poll(Duration::from_millis(50))?
                && let Event::Key(key) = event::read()?
                && key.code == KeyCode::Char('q')
            {
                break;
            }
        }
        Ok(())
    }
}
