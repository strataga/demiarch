//! Demiarch TUI - Real-time agent monitoring dashboard
//!
//! This TUI provides a live view of:
//! - Active agent executions across all projects
//! - Token usage and costs in real-time
//! - Generation progress and status
//! - Skill activations
//! - Hook executions

use std::io;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};

fn main() -> anyhow::Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run app
    let result = run_app(&mut terminal);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> anyhow::Result<()> {
    loop {
        terminal.draw(|frame| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([
                    Constraint::Length(3),  // Header
                    Constraint::Min(10),    // Main content
                    Constraint::Length(3),  // Footer
                ])
                .split(frame.area());

            // Header
            let header = Paragraph::new("Demiarch Monitor")
                .style(Style::default().fg(Color::Cyan))
                .block(Block::default().borders(Borders::ALL).title("Demiarch TUI"));
            frame.render_widget(header, chunks[0]);

            // Main content - split into panels
            let main_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(30),  // Projects
                    Constraint::Percentage(40),  // Agents
                    Constraint::Percentage(30),  // Stats
                ])
                .split(chunks[1]);

            // Projects panel
            let projects = Paragraph::new("No active projects\n\nRun: demiarch new <name>")
                .block(Block::default().borders(Borders::ALL).title("Projects"));
            frame.render_widget(projects, main_chunks[0]);

            // Agents panel
            let agents = Paragraph::new("No active agents\n\nAgents appear here when generating code")
                .block(Block::default().borders(Borders::ALL).title("Active Agents"));
            frame.render_widget(agents, main_chunks[1]);

            // Stats panel
            let stats = Paragraph::new(
                "Tokens: 0\nCost: $0.00\nSkills: 0\n\nPress 'q' to quit"
            )
                .block(Block::default().borders(Borders::ALL).title("Session Stats"));
            frame.render_widget(stats, main_chunks[2]);

            // Footer
            let footer = Paragraph::new("q: Quit | r: Refresh | p: Projects | a: Agents | s: Skills")
                .style(Style::default().fg(Color::DarkGray))
                .block(Block::default().borders(Borders::ALL));
            frame.render_widget(footer, chunks[2]);
        })?;

        // Handle input
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press && key.code == KeyCode::Char('q') {
                    return Ok(());
                }
            }
        }
    }
}
