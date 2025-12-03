//! TUI for proxy checker with progress display

use crate::proxy::{CheckerConfig, Proxy, ProxyCheckResult, ProxyChecker};
use crate::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Gauge, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};
use std::collections::VecDeque;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::PathBuf;
use tokio::time::Duration;

/// Maximum number of recent proxies to keep for display
const MAX_RECENT_PROXIES: usize = 100;

/// Proxy checker TUI application state
pub struct ProxyCheckerApp {
    /// Proxies to check
    proxies: Vec<Proxy>,
    /// Checker configuration
    config: CheckerConfig,
    /// Output file for good proxies
    good_output: Option<PathBuf>,
    /// Output file for bad proxies
    bad_output: Option<PathBuf>,
    /// Total number of proxies
    total: usize,
    /// Number of checked proxies
    checked: usize,
    /// Number of good proxies found
    good_count: usize,
    /// Number of bad proxies found
    bad_count: usize,
    /// Recent good proxies (for display, stored as VecDeque for O(1) operations)
    recent_good: VecDeque<ProxyCheckResult>,
    /// Recent bad proxies (for display, stored as VecDeque for O(1) operations)
    recent_bad: VecDeque<ProxyCheckResult>,
    /// Selected list (0 = good, 1 = bad)
    selected_list: usize,
    /// Selected item in current list
    list_state: ListState,
    /// Status message
    status_message: String,
    /// Whether checking is complete
    is_complete: bool,
    /// Whether the user wants to quit
    should_quit: bool,
}

impl ProxyCheckerApp {
    /// Create a new proxy checker TUI application
    pub fn new(
        proxies: Vec<Proxy>,
        config: CheckerConfig,
        good_output: Option<PathBuf>,
        bad_output: Option<PathBuf>,
    ) -> Self {
        let total = proxies.len();
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Self {
            proxies,
            config,
            good_output,
            bad_output,
            total,
            checked: 0,
            good_count: 0,
            bad_count: 0,
            recent_good: VecDeque::new(),
            recent_bad: VecDeque::new(),
            selected_list: 0,
            list_state,
            status_message: "Starting proxy check... Press 'q' to quit.".to_string(),
            is_complete: false,
            should_quit: false,
        }
    }

    /// Run the TUI application
    pub async fn run(&mut self) -> Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let result = self.run_app(&mut terminal).await;

        // Restore terminal
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        result
    }

    async fn run_app<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        // Create output files if specified
        let mut good_file = self
            .good_output
            .as_ref()
            .map(|p| {
                OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(true)
                    .open(p)
            })
            .transpose()?;

        let mut bad_file = self
            .bad_output
            .as_ref()
            .map(|p| {
                OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(true)
                    .open(p)
            })
            .transpose()?;

        // Start the proxy checker
        let checker = ProxyChecker::with_config(self.config.clone());
        let mut rx = checker.check_proxies_stream(self.proxies.clone());

        loop {
            // Draw UI
            terminal.draw(|f| self.ui(f))?;

            // Handle key events with a short timeout
            if event::poll(Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        self.handle_input(key.code);
                        if self.should_quit {
                            break;
                        }
                    }
                }
            }

            // Try to receive results without blocking
            match rx.try_recv() {
                Ok(result) => {
                    self.checked += 1;

                    if result.is_working() {
                        self.good_count += 1;

                        // Write to file immediately
                        if let Some(ref mut file) = good_file {
                            writeln!(file, "{}", result.proxy.to_full_string())?;
                            file.flush()?;
                        }

                        // Keep last MAX_RECENT_PROXIES for display using VecDeque for O(1) operations
                        self.recent_good.push_back(result);
                        if self.recent_good.len() > MAX_RECENT_PROXIES {
                            self.recent_good.pop_front();
                        }
                    } else {
                        self.bad_count += 1;

                        // Write to file immediately
                        if let Some(ref mut file) = bad_file {
                            writeln!(file, "{}", result.proxy.to_full_string())?;
                            file.flush()?;
                        }

                        // Keep last MAX_RECENT_PROXIES for display using VecDeque for O(1) operations
                        self.recent_bad.push_back(result);
                        if self.recent_bad.len() > MAX_RECENT_PROXIES {
                            self.recent_bad.pop_front();
                        }
                    }

                    // Update status message
                    let percentage = (self.checked as f64 / self.total as f64 * 100.0) as u32;
                    self.status_message = format!(
                        "Checking... {}% ({}/{}) | Good: {} | Bad: {}",
                        percentage, self.checked, self.total, self.good_count, self.bad_count
                    );
                }
                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                    // No result available, continue
                }
                Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                    // Channel closed, checking complete
                    self.is_complete = true;
                    self.status_message = format!(
                        "Complete! Checked: {} | Good: {} | Bad: {} | Press 'q' to quit",
                        self.total, self.good_count, self.bad_count
                    );
                }
            }

            // If complete and not exiting, just keep drawing
            if self.is_complete && !self.should_quit {
                // Continue to allow user to view results
            }
        }

        Ok(())
    }

    fn handle_input(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.should_quit = true;
            }
            KeyCode::Tab => {
                // Switch between good and bad lists
                self.selected_list = (self.selected_list + 1) % 2;
                self.list_state.select(Some(0));
            }
            KeyCode::Down => {
                let list = if self.selected_list == 0 {
                    &self.recent_good
                } else {
                    &self.recent_bad
                };
                let i = match self.list_state.selected() {
                    Some(i) => {
                        if i >= list.len().saturating_sub(1) {
                            0
                        } else {
                            i + 1
                        }
                    }
                    None => 0,
                };
                self.list_state.select(Some(i));
            }
            KeyCode::Up => {
                let list = if self.selected_list == 0 {
                    &self.recent_good
                } else {
                    &self.recent_bad
                };
                let i = match self.list_state.selected() {
                    Some(i) => {
                        if i == 0 {
                            list.len().saturating_sub(1)
                        } else {
                            i - 1
                        }
                    }
                    None => 0,
                };
                self.list_state.select(Some(i));
            }
            _ => {}
        }
    }

    fn ui(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Length(3), // Progress bar
                Constraint::Min(0),    // Proxy lists
                Constraint::Length(3), // Status bar
            ])
            .split(f.size());

        // Title
        let title = Paragraph::new("🔍 Proxy Checker")
            .style(Style::default().fg(Color::Cyan))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);

        // Progress bar
        let progress = if self.total > 0 {
            (self.checked as f64 / self.total as f64 * 100.0) as u16
        } else {
            0
        };
        let progress_label = format!("{}/{} ({}%)", self.checked, self.total, progress);
        let gauge = Gauge::default()
            .block(Block::default().borders(Borders::ALL).title("Progress"))
            .gauge_style(Style::default().fg(Color::Green).bg(Color::Black))
            .percent(progress)
            .label(progress_label);
        f.render_widget(gauge, chunks[1]);

        // Split the main area into two columns for good and bad proxies
        let proxy_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[2]);

        // Render good proxies list
        Self::render_proxy_list_static(
            f,
            proxy_chunks[0],
            "✓ Good Proxies",
            &self.recent_good,
            self.good_count,
            self.selected_list == 0,
            Color::Green,
            if self.selected_list == 0 { Some(&mut self.list_state) } else { None },
        );

        // Render bad proxies list
        Self::render_proxy_list_static(
            f,
            proxy_chunks[1],
            "✗ Bad Proxies",
            &self.recent_bad,
            self.bad_count,
            self.selected_list == 1,
            Color::Red,
            if self.selected_list == 1 { Some(&mut self.list_state) } else { None },
        );

        // Status bar
        let status = Paragraph::new(self.status_message.clone())
            .style(if self.is_complete {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::Yellow)
            })
            .wrap(Wrap { trim: true })
            .block(Block::default().borders(Borders::ALL).title("Status"));
        f.render_widget(status, chunks[3]);
    }

    #[allow(clippy::too_many_arguments)]
    fn render_proxy_list_static(
        f: &mut Frame,
        area: Rect,
        title: &str,
        results: &VecDeque<ProxyCheckResult>,
        total_count: usize,
        is_selected: bool,
        color: Color,
        list_state: Option<&mut ListState>,
    ) {
        let items: Vec<ListItem> = results
            .iter()
            .rev() // Show newest first
            .map(|result| {
                let content = if let Some(time) = result.response_time_ms {
                    format!("{} ({}ms)", result.proxy.to_simple_string(), time)
                } else {
                    result.proxy.to_simple_string()
                };
                ListItem::new(content).style(Style::default().fg(color))
            })
            .collect();

        let block_title = format!("{} ({})", title, total_count);
        let border_style = if is_selected {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(block_title)
                    .border_style(border_style),
            )
            .highlight_style(Style::default().bg(Color::DarkGray))
            .highlight_symbol(">> ");

        if let Some(state) = list_state {
            f.render_stateful_widget(list, area, state);
        } else {
            f.render_widget(list, area);
        }
    }
}
