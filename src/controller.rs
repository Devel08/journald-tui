use crate::model::JournalEntry;

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Cell, Paragraph, Row, Scrollbar, ScrollbarOrientation, ScrollbarState,
        Table, TableState, Wrap,
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

enum LoopAction {
    Continue { should_render: bool },
    Quit,
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

    pub fn draw_table(&mut self, f: &mut Frame, area: Rect) {
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

        let table = Table::new(
            //Make rows out of the logs data
            self.logs.iter().map(|log| {
                let unit = log.systemd_unit.as_deref().unwrap_or("unknown");
                let process = log.process_name.as_deref().unwrap_or("unknown");
                let message = &log.message;

                // https://www.rfc-editor.org/info/rfc5424/
                let log_style = match log.priority.as_deref() {
                    Some("0") | Some("1") | Some("2") | Some("3") => {
                        Style::default().fg(Color::Red)
                    }
                    Some("4") => Style::default().fg(Color::LightYellow),
                    _ => Style::default().fg(Color::Gray),
                };

                Row::new(vec![
                    Cell::from(unit.to_string()),
                    Cell::from(process.to_string()),
                    Cell::from(message.to_string()).style(log_style),
                ])
            }),
            widths,
        )
        .header(header_row)
        .block(block)
        .row_highlight_style(Style::default().bg(Color::DarkGray))
        .highlight_symbol(">> ");

        let table_area = area;

        f.render_stateful_widget(table, table_area, &mut self.table_state);

        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"));

        f.render_stateful_widget(scrollbar, table_area, &mut self.scroll_state);

        // update the scrollbar according to the count of the logs
        self.scroll_state = self.scroll_state.content_length(self.logs.len());
    }

    pub fn draw_metadata(&mut self, f: &mut Frame, area: Rect) {
        let selected_index = self.table_state.selected().unwrap_or(0);
        if self.logs.is_empty() {
            return;
        }
        let log = &self.logs[selected_index];

        // TODO: Add more fields?
        let process = log.process_name.as_deref().unwrap_or("unknown");
        let message = &log.message;
        let pid = log.pid.as_deref().unwrap_or("unknown");
        let hostname = log.hostname.as_deref().unwrap_or("unknown");

        // https://www.rfc-editor.org/info/rfc5424/
        let type_str = match log.priority.as_deref() {
            Some("0") => "Emergency",
            Some("1") => "Alert",
            Some("2") => "Critical",
            Some("3") => "Error",
            Some("4") => "Warning",
            Some("5") => "Notice",
            Some("6") => "Informational",
            Some("7") => "Debug",
            _ => "",
        };

        let connecting_block =
            Block::default().borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM);

        let inner_area = connecting_block.inner(area);
        f.render_widget(connecting_block, area);

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
            .split(inner_area);

        let label_style = Style::default().fg(Color::Cyan);

        let metadata = vec![
            Line::from(vec![
                Span::styled("PID: ", label_style),
                Span::raw(pid),
                Span::raw("      "),
                Span::styled("Host: ", label_style),
                Span::raw(hostname),
            ]),
            Line::from(vec![
                Span::styled("Type: ", label_style),
                Span::raw(type_str),
                Span::raw("     "),
                Span::styled("Process: ", label_style),
                Span::raw(process),
            ]),
        ];
        f.render_widget(Paragraph::new(metadata), chunks[0]);

        let msg_header = Span::styled("Message: ", label_style);
        let msg_body = Span::raw(message);

        // Multiline message
        let message =
            Paragraph::new(Line::from(vec![msg_header, msg_body])).wrap(Wrap { trim: true });

        f.render_widget(message, chunks[1]);
    }

    pub fn draw_status_bar(&mut self, f: &mut Frame, area: Rect) {
        let status_bar = Paragraph::new(" [↑] Up  [↓] Down  [/] Local search  [q] Quit ")
            .block(Block::default())
            .style(
                Style::default()
                    .bg(Color::Gray)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            );
        f.render_widget(status_bar, area);
    }

    // Returns a `LoopAction` enum
    // Continue -> the event loop continues and passes render state
    // Quit -> user pressed `q`, breaks the event loop.
    fn handle_keystrokes(&mut self, key: KeyEvent) -> LoopAction {
        match key.code {
            KeyCode::Char('q') => LoopAction::Quit,
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
                LoopAction::Continue {
                    should_render: true,
                }
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
                LoopAction::Continue {
                    should_render: true,
                }
            }
            _ => LoopAction::Continue {
                should_render: false,
            },
        }
    }

    pub async fn run(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
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

            if should_render {
                terminal.draw(|f| {
                    let size = f.area();

                    //The table will be 33 rows (for 30 entries), so that the left out space can
                    //be used for other stuff, like metadata when clicked on
                    //some log
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Percentage(70),
                            Constraint::Percentage(5),
                            Constraint::Min(0),
                            Constraint::Length(1),
                            Constraint::Length(1),
                        ])
                        .split(size);

                    // 0 - table
                    // 1 - metadata
                    // 2 - space that we'll use up for something
                    // 3 - status bar
                    // 4 - search(?)

                    self.draw_table(f, chunks[0]);
                    self.draw_metadata(f, chunks[1]);
                    self.draw_status_bar(f, chunks[3]);
                })?;
                should_render = false;
            }

            if event::poll(Duration::from_millis(50))? {
                match event::read()? {
                    Event::Key(key) => match self.handle_keystrokes(key) {
                        LoopAction::Quit => break,
                        LoopAction::Continue {
                            should_render: render,
                        } => {
                            should_render = render;
                        }
                    },
                    Event::Resize(_, _) => {
                        terminal.autoresize()?;
                        terminal.clear()?;
                        should_render = true;
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
}
