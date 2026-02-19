use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph, Tabs},
    Frame, Terminal,
};
use std::io::{self, BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

struct AppState {
    is_running: bool,
    is_updating: bool,
    output_lines: Vec<String>,
    viruses_found: Vec<String>,
    current_status: String,
    progress: f64,
    file_count: usize,
    scanned_count: usize,
    current_view: View,
    exit_code: Option<i32>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            is_running: false,
            is_updating: false,
            output_lines: Vec::new(),
            viruses_found: Vec::new(),
            current_status: "Ready".to_string(),
            progress: 0.0,
            file_count: 0,
            scanned_count: 0,
            current_view: View::Dashboard,
            exit_code: None,
        }
    }
}

#[derive(Clone, PartialEq)]
enum View {
    Dashboard,
    Output,
    Viruses,
    Summary,
}

struct App {
    state: Arc<Mutex<AppState>>,
}

impl App {
    fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(AppState::default())),
        }
    }

    fn run_scan(&self) {
        let state = self.state.clone();

        thread::spawn(move || {
            // Reset state
            {
                let mut s = state.lock().unwrap();
                s.is_running = true;
                s.output_lines.clear();
                s.viruses_found.clear();
                s.current_status = "Starting scan...".to_string();
                s.progress = 0.0;
                s.exit_code = None;
                s.file_count = 0;
                s.scanned_count = 0;
            }

            // Run the script
            let mut cmd = Command::new("sudo");
            cmd.arg("./clam.sh");

            cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

            match cmd.spawn() {
                Ok(mut child) => {
                    // Read stdout
                    if let Some(stdout) = child.stdout.take() {
                        let reader = BufReader::new(stdout);
                        let state_clone = state.clone();

                        thread::spawn(move || {
                            for line in reader.lines() {
                                if let Ok(line) = line {
                                    let mut s = state_clone.lock().unwrap();
                                    s.output_lines.push(line.clone());

                                    if line.contains("Total files to scan:") {
                                        if let Some(count) = line.split(':').nth(1) {
                                            if let Ok(num) = count.trim().parse::<usize>() {
                                                s.file_count = num;
                                            }
                                        }
                                    } else if line.contains("Scanning") {
                                        s.scanned_count += 1;
                                        if s.file_count > 0 {
                                            s.progress = (s.scanned_count as f64
                                                / s.file_count as f64)
                                                * 100.0;
                                        }
                                    } else if line.contains("FOUND") {
                                        s.viruses_found.push(line.clone());
                                    }

                                    if s.output_lines.len() > 1000 {
                                        s.output_lines.remove(0);
                                    }
                                }
                            }
                        });
                    }

                    // Read stderr
                    if let Some(stderr) = child.stderr.take() {
                        let reader = BufReader::new(stderr);
                        let state_clone = state.clone();

                        thread::spawn(move || {
                            for line in reader.lines() {
                                if let Ok(line) = line {
                                    let mut s = state_clone.lock().unwrap();
                                    s.output_lines.push(format!("[ERROR] {}", line));

                                    if s.output_lines.len() > 1000 {
                                        s.output_lines.remove(0);
                                    }
                                }
                            }
                        });
                    }

                    // Wait for process
                    match child.wait() {
                        Ok(status) => {
                            let mut s = state.lock().unwrap();
                            s.is_running = false;
                            s.exit_code = status.code();
                            s.current_status = match s.exit_code {
                                Some(0) => "Scan completed: No viruses found".to_string(),
                                Some(1) => "Scan completed: Viruses found".to_string(),
                                _ => format!("Scan completed (exit: {:?})", s.exit_code),
                            };
                            s.progress = 100.0;
                        }
                        Err(e) => {
                            let mut s = state.lock().unwrap();
                            s.is_running = false;
                            s.current_status = format!("Process error: {}", e);
                        }
                    }
                }
                Err(e) => {
                    let mut s = state.lock().unwrap();
                    s.is_running = false;
                    s.current_status = format!("Failed to start: {}", e);
                    s.output_lines.push(format!("Failed to start: {}", e));
                }
            }
        });
    }

    fn update_virus_db(&self) {
        let state = self.state.clone();

        thread::spawn(move || {
            {
                let mut s = state.lock().unwrap();
                s.is_updating = true;
                s.current_status = "Updating virus database...".to_string();
            }

            let output = Command::new("sudo").arg("freshclam").output();

            {
                let mut s = state.lock().unwrap();
                s.is_updating = false;

                match output {
                    Ok(output) => {
                        if output.status.success() {
                            s.current_status = "Database updated successfully".to_string();
                        } else {
                            s.current_status = "Database update failed".to_string();
                        }
                    }
                    Err(e) => {
                        s.current_status = format!("Update error: {}", e);
                    }
                }
            }
        });
    }
}

fn main() -> io::Result<()> {
    // Quick check if clam.sh exists
    if !std::path::Path::new("./clam.sh").exists() {
        println!("Error: clam.sh not found!");
        println!("Please make sure clam.sh is in the current directory.");
        return Ok(());
    }

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create and run app
    let app = App::new();
    let result = run_app(&mut terminal, app);

    // Cleanup
    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;

    result
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, &app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    KeyCode::Char('s') => {
                        let state = app.state.lock().unwrap();
                        if !state.is_running && !state.is_updating {
                            drop(state);
                            app.run_scan();
                        }
                    }
                    KeyCode::Char('u') => {
                        let state = app.state.lock().unwrap();
                        if !state.is_running && !state.is_updating {
                            drop(state);
                            app.update_virus_db();
                        }
                    }
                    KeyCode::Char('1') => app.state.lock().unwrap().current_view = View::Dashboard,
                    KeyCode::Char('2') => app.state.lock().unwrap().current_view = View::Output,
                    KeyCode::Char('3') => app.state.lock().unwrap().current_view = View::Viruses,
                    KeyCode::Char('4') => app.state.lock().unwrap().current_view = View::Summary,
                    KeyCode::Char('c') => app.state.lock().unwrap().output_lines.clear(),
                    KeyCode::Tab => {
                        let mut state = app.state.lock().unwrap();
                        state.current_view = match state.current_view {
                            View::Dashboard => View::Output,
                            View::Output => View::Viruses,
                            View::Viruses => View::Summary,
                            View::Summary => View::Dashboard,
                        };
                    }
                    _ => {}
                }
            }
        }

        thread::sleep(Duration::from_millis(50));
    }
}

fn ui(f: &mut Frame, app: &App) {
    let state = app.state.lock().unwrap();
    let area = f.size();

    // Title
    let title = Paragraph::new("🛡️ ClamAV Scanner")
        .style(
            Style::default()
                .fg(Color::LightCyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(title, Rect::new(0, 0, area.width, 1));

    // Tabs
    let tabs = Tabs::new(vec!["Dashboard", "Output", "Viruses", "Summary"])
        .select(match state.current_view {
            View::Dashboard => 0,
            View::Output => 1,
            View::Viruses => 2,
            View::Summary => 3,
        })
        .block(Block::default().borders(Borders::BOTTOM))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );
    f.render_widget(tabs, Rect::new(0, 1, area.width, 1));

    // Main content
    let main_area = Rect::new(0, 2, area.width, area.height - 4);
    match state.current_view {
        View::Dashboard => render_dashboard(f, main_area, &state),
        View::Output => render_output(f, main_area, &state),
        View::Viruses => render_viruses(f, main_area, &state),
        View::Summary => render_summary(f, main_area, &state),
    }

    // Status bar
    let status_style = if state.is_running {
        Style::default().fg(Color::Red)
    } else if state.is_updating {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Green)
    };

    let status_text = format!(
        "Status: {} | Files: {}/{} | Progress: {:.1}%",
        state.current_status, state.scanned_count, state.file_count, state.progress
    );

    let status_bar = Paragraph::new(status_text)
        .style(status_style)
        .block(Block::default().borders(Borders::TOP));
    f.render_widget(status_bar, Rect::new(0, area.height - 2, area.width, 1));

    // Help bar
    let help_bar = Paragraph::new("Keys: [S]can [U]pdate [1-4] Tabs [C]lear [Q]uit")
        .style(Style::default().fg(Color::DarkGray))
        .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(help_bar, Rect::new(0, area.height - 1, area.width, 1));
}

fn render_dashboard(f: &mut Frame, area: Rect, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(6),
            Constraint::Length(6),
            Constraint::Min(0),
        ])
        .split(area);

    // Status panel
    let status_lines = vec![
        Line::from(vec![
            Span::styled("Status: ", Style::default().fg(Color::Yellow)),
            Span::styled(
                &state.current_status,
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Running: ", Style::default().fg(Color::Yellow)),
            Span::styled(
                if state.is_running { "YES" } else { "NO" },
                if state.is_running {
                    Color::Red
                } else {
                    Color::Green
                },
            ),
        ]),
        Line::from(vec![
            Span::styled("Exit Code: ", Style::default().fg(Color::Yellow)),
            Span::styled(
                state
                    .exit_code
                    .map(|c| c.to_string())
                    .unwrap_or("N/A".to_string()),
                match state.exit_code {
                    Some(0) => Color::Green,
                    _ => Color::Gray,
                },
            ),
        ]),
    ];

    let status_widget = Paragraph::new(status_lines)
        .block(Block::default().title(" Status ").borders(Borders::ALL));
    f.render_widget(status_widget, chunks[0]);

    // Progress gauge
    let gauge = Gauge::default()
        .block(Block::default().title(" Progress ").borders(Borders::ALL))
        .gauge_style(Style::default().fg(Color::Cyan))
        .percent(state.progress as u16);
    f.render_widget(gauge, chunks[1]);

    // Actions
    let actions = vec![
        Line::from(Span::styled(
            "Quick Actions:",
            Style::default()
                .fg(Color::LightMagenta)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("• [S] - Start scan"),
        Line::from("• [U] - Update database"),
        Line::from("• [2] - View output"),
        Line::from("• [3] - View viruses"),
        Line::from("• [C] - Clear output"),
    ];

    let actions_widget =
        Paragraph::new(actions).block(Block::default().title(" Actions ").borders(Borders::ALL));
    f.render_widget(actions_widget, chunks[2]);
}

fn render_output(f: &mut Frame, area: Rect, state: &AppState) {
    if state.output_lines.is_empty() {
        let message = Paragraph::new(vec![
            Line::from("No output yet."),
            Line::from(""),
            Line::from("Press [S] to start a scan"),
        ])
        .style(Style::default().fg(Color::DarkGray))
        .block(
            Block::default()
                .title(" Live Output ")
                .borders(Borders::ALL),
        );
        f.render_widget(message, area);
    } else {
        let text: Vec<Line> = state
            .output_lines
            .iter()
            .rev()
            .take(area.height as usize - 2)
            .map(|line| Line::from(Span::raw(line)))
            .collect();

        let output_widget = Paragraph::new(text).block(
            Block::default()
                .title(format!(" Output ({}) ", state.output_lines.len()))
                .borders(Borders::ALL),
        );
        f.render_widget(output_widget, area);
    }
}

fn render_viruses(f: &mut Frame, area: Rect, state: &AppState) {
    if state.viruses_found.is_empty() {
        let text = Paragraph::new("No viruses detected.")
            .style(Style::default().fg(Color::Green))
            .block(
                Block::default()
                    .title(" Viruses Found ")
                    .borders(Borders::ALL),
            );
        f.render_widget(text, area);
    } else {
        let text: Vec<Line> = state
            .viruses_found
            .iter()
            .map(|virus| {
                Line::from(vec![
                    Span::styled("⚠ ", Style::default().fg(Color::Red)),
                    Span::raw(virus),
                ])
            })
            .collect();

        let virus_widget = Paragraph::new(text).block(
            Block::default()
                .title(format!(" Viruses ({}) ", state.viruses_found.len()))
                .borders(Borders::ALL),
        );
        f.render_widget(virus_widget, area);
    }
}

fn render_summary(f: &mut Frame, area: Rect, state: &AppState) {
    let summary = vec![
        Line::from(Span::styled(
            "Scan Summary:",
            Style::default()
                .fg(Color::LightMagenta)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(format!("Files Scanned: {}", state.scanned_count)),
        Line::from(format!("Total Files: {}", state.file_count)),
        Line::from(format!("Viruses Found: {}", state.viruses_found.len())),
        Line::from(format!("Progress: {:.1}%", state.progress)),
        Line::from(format!("Exit Code: {:?}", state.exit_code)),
    ];

    let summary_widget =
        Paragraph::new(summary).block(Block::default().title(" Summary ").borders(Borders::ALL));
    f.render_widget(summary_widget, area);
}
