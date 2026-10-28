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
    widgets::{
        Block, Borders, Cell, Row, Scrollbar, ScrollbarOrientation, ScrollbarState, Table,
        TableState,
    },
};
use std::io::{self, Stdout};
use std::time::Duration;
use tokio::sync::mpsc::Receiver;

pub struct Controller {
    logs: Vec<JournalEntry>,
    rx: Receiver<JournalEntry>,
    table_state: TableState,
    scroll_state: ScrollbarState,
}

impl Controller {
    pub fn new(rx: Receiver<JournalEntry>) -> Self {
        Self {
            logs: Vec::new(),
            rx,
            table_state: TableState::default(),
            scroll_state: ScrollbarState::default(),
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
        //
        //
        // !!! TODO: Maybe we'll need more abstraction layers here ;/
        // e.g. for the table, keystrokes etc.
        //
        //

        // we don't wanna render all the time
        let mut should_render = true;

        loop {
            while let Ok(entry) = self.rx.try_recv() {
                self.logs.push(entry);
                // Maybe we should parse that with clap-rs
                // and make 100 default if argument is not provided
                if self.logs.len() > 100 {
                    self.logs.remove(0);
                }
                should_render = true;
            }

            // update the scrollbar according to the count of the logs
            self.scroll_state = self.scroll_state.content_length(self.logs.len());

            if should_render {
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

                    let table = Table::new(rows, widths)
                        .header(header_row)
                        .block(block)
                        .row_highlight_style(Style::default().bg(Color::DarkGray))
                        .highlight_symbol(">> ");

                    //The table will be 33 rows (for 30 entries), so that the left out space can
                    //be used for other stuff, like metadata when clicked on
                    //some log
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([Constraint::Length(33), Constraint::Min(0)])
                        .split(size);

                    let table_area = chunks[0];

                    f.render_stateful_widget(table, table_area, &mut self.table_state);

                    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                        .begin_symbol(Some("▲"))
                        .end_symbol(Some("▼"));

                    f.render_stateful_widget(scrollbar, table_area, &mut self.scroll_state);
                })?;
                should_render = false;
            }
            if event::poll(Duration::from_millis(50))?
                && let Event::Key(key) = event::read()?
            {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Down => {
                        let i = match self.table_state.selected() {
                            Some(i) => {
                                if i >= self.logs.len().saturating_sub(1) {
                                    i
                                } else {
                                    i + 1
                                }
                            }
                            None => 0,
                        };
                        self.table_state.select(Some(i));
                        self.scroll_state = self.scroll_state.position(i);
                        should_render = true;
                    }
                    KeyCode::Up => {
                        let i = match self.table_state.selected() {
                            Some(i) => {
                                if i == 0 {
                                    0
                                } else {
                                    i - 1
                                }
                            }
                            None => 0,
                        };
                        self.table_state.select(Some(i));
                        self.scroll_state = self.scroll_state.position(i);
                        should_render = true;
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
}
