//! Demiarch TUI - Real-time agent monitoring dashboard
//!
//! This TUI provides a live view of:
//! - Active agent executions across all projects
//! - Agent hierarchy tree with status indicators
//! - Token usage and costs in real-time
//! - Generation progress and status
//! - Skill activations
//! - Hook executions

#[cfg(test)]
mod main_tests;

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use demiarch_core::agents::events::read_current_session_events;
use demiarch_core::visualization::{
    AgentStatusBar, HierarchyTreeWidget, RenderOptions, TreeBuilder, TreeColors,
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, Tabs},
    Terminal,
};
use std::io;

/// Application state
struct App {
    /// Currently selected tab
    current_tab: usize,
    /// Tab titles
    tabs: Vec<&'static str>,
    /// Scroll offset for agent tree
    tree_scroll: usize,
    /// Whether to use ASCII mode
    ascii_mode: bool,
}

impl App {
    fn new() -> Self {
        Self {
            current_tab: 1, // Start on Agents tab
            tabs: vec!["Projects", "Agents", "Stats", "Help"],
            tree_scroll: 0,
            ascii_mode: false,
        }
    }

    fn next_tab(&mut self) {
        self.current_tab = (self.current_tab + 1) % self.tabs.len();
    }

    fn prev_tab(&mut self) {
        if self.current_tab == 0 {
            self.current_tab = self.tabs.len() - 1;
        } else {
            self.current_tab -= 1;
        }
    }

    fn scroll_up(&mut self) {
        self.tree_scroll = self.tree_scroll.saturating_sub(1);
    }

    fn scroll_down(&mut self) {
        self.tree_scroll = self.tree_scroll.saturating_add(1);
    }

    fn toggle_ascii(&mut self) {
        self.ascii_mode = !self.ascii_mode;
    }
}

fn main() -> anyhow::Result<()> {
    // Load .env file if present (silently ignore if not found)
    dotenvy::dotenv().ok();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new();

    // Run app
    let result = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> anyhow::Result<()> {
    loop {
        terminal.draw(|frame| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([
                    Constraint::Length(3), // Header with tabs
                    Constraint::Min(10),   // Main content
                    Constraint::Length(3), // Status bar
                ])
                .split(frame.area());

            // Header with tabs
            let tabs = Tabs::new(app.tabs.to_vec())
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Demiarch Monitor"),
                )
                .select(app.current_tab)
                .style(Style::default().fg(Color::White))
                .highlight_style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                );
            frame.render_widget(tabs, chunks[0]);

            // Main content based on selected tab
            match app.current_tab {
                0 => render_projects_tab(frame, chunks[1]),
                1 => render_agents_tab(frame, chunks[1], app),
                2 => render_stats_tab(frame, chunks[1]),
                3 => render_help_tab(frame, chunks[1]),
                _ => {}
            }

            // Status bar
            render_status_bar(frame, chunks[2], app);
        })?;

        // Handle input
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Tab | KeyCode::Right => app.next_tab(),
                        KeyCode::BackTab | KeyCode::Left => app.prev_tab(),
                        KeyCode::Up | KeyCode::Char('k') => app.scroll_up(),
                        KeyCode::Down | KeyCode::Char('j') => app.scroll_down(),
                        KeyCode::Char('a') => app.toggle_ascii(),
                        KeyCode::Char('1') => app.current_tab = 0,
                        KeyCode::Char('2') => app.current_tab = 1,
                        KeyCode::Char('3') => app.current_tab = 2,
                        KeyCode::Char('4') | KeyCode::Char('?') => app.current_tab = 3,
                        _ => {}
                    }
                }
            }
        }
    }
}

fn render_projects_tab(frame: &mut ratatui::Frame, area: Rect) {
    let content = Paragraph::new(
        "No active projects\n\n\
         Projects will appear here during code generation.\n\n\
         To create a project:\n\
         $ demiarch new <name> --framework <framework>\n\n\
         To start code generation:\n\
         $ demiarch generate \"description\"",
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Projects")
            .style(Style::default()),
    );
    frame.render_widget(content, area);
}

fn render_agents_tab(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    // Split into tree and details panels
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    // Build tree from live events (falls back to placeholder if no events)
    let tree = TreeBuilder::from_live_events();

    // Configure options based on app state
    let options = if app.ascii_mode {
        RenderOptions::ascii()
    } else {
        RenderOptions::default()
    };

    // Agent hierarchy tree
    let tree_widget = HierarchyTreeWidget::new(&tree)
        .options(options)
        .colors(TreeColors::default())
        .scroll(app.tree_scroll)
        .show_header(true)
        .show_footer(true)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Agent Hierarchy")
                .style(Style::default()),
        );
    frame.render_widget(tree_widget, chunks[0]);

    // Agent details panel
    let details = Paragraph::new(
        "Agent Details\n\
         ─────────────\n\n\
         Select an agent to view details.\n\n\
         During execution:\n\
         • Active agents shown with ●\n\
         • Completed agents shown with ✓\n\
         • Failed agents shown with ✗\n\n\
         Agent Types:\n\
         🎭 Orchestrator - Session director\n\
         📋 Planner - Task coordinator\n\
         💻 Coder - Code generation\n\
         🔍 Reviewer - Code review\n\
         🧪 Tester - Test generation\n\n\
         Use ↑/↓ or j/k to scroll\n\
         Press 'a' to toggle ASCII mode",
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Details")
            .style(Style::default()),
    );
    frame.render_widget(details, chunks[1]);
}

fn render_stats_tab(frame: &mut ratatui::Frame, area: Rect) {
    // Split into multiple stat panels
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7), // Session stats
            Constraint::Length(7), // Token usage
            Constraint::Min(5),    // Recent activity
        ])
        .split(area);

    // Get real data from live events
    let tree = TreeBuilder::from_live_events();
    let events = read_current_session_events();

    let total_agents = tree.count();
    let active_agents = tree.count_active();
    let completed = tree.count_completed();
    let failed = tree.count_failed();
    let total_tokens = tree.tree_tokens();

    // Session stats
    let session_stats = Paragraph::new(format!(
        "Total Agents: {}\n\
         Active Agents: {}\n\
         Completed: {}\n\
         Failed: {}\n\
         Events: {}",
        total_agents,
        active_agents,
        completed,
        failed,
        events.len()
    ))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Session Statistics"),
    );
    frame.render_widget(session_stats, chunks[0]);

    // Token usage
    // Rough cost estimate: $3/million input, $15/million output (Claude pricing)
    let estimated_cost = (total_tokens as f64 / 1_000_000.0) * 10.0; // rough average
    let token_stats = Paragraph::new(format!(
        "Total Tokens: {}\n\
         Estimated Cost: ${:.4}\n\
         \n\
         (Token breakdown per agent\n\
         shown in agent tree)",
        total_tokens, estimated_cost
    ))
    .block(Block::default().borders(Borders::ALL).title("Token Usage"));
    frame.render_widget(token_stats, chunks[1]);

    // Recent activity - show last few events
    let recent_events: Vec<String> = events
        .iter()
        .rev()
        .take(8)
        .map(|e| {
            let time = e.timestamp.format("%H:%M:%S");
            let event_type = match e.event_type {
                demiarch_core::agents::events::AgentEventType::Spawned => "🆕 Spawned",
                demiarch_core::agents::events::AgentEventType::Started => "▶️  Started",
                demiarch_core::agents::events::AgentEventType::StatusUpdate => "📊 Status",
                demiarch_core::agents::events::AgentEventType::Completed => "✓  Done",
                demiarch_core::agents::events::AgentEventType::Failed => "✗  Failed",
                demiarch_core::agents::events::AgentEventType::Cancelled => "⊘  Cancel",
                demiarch_core::agents::events::AgentEventType::TokenUpdate => "🎫 Tokens",
            };
            format!("[{}] {} {}", time, event_type, e.agent.name)
        })
        .collect();

    let activity_text = if recent_events.is_empty() {
        "No recent activity\n\n\
         Events will appear here during code generation:\n\
         • Agent spawned\n\
         • Agent completed\n\
         • File created"
            .to_string()
    } else {
        recent_events.join("\n")
    };

    let activity = Paragraph::new(activity_text).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Recent Activity"),
    );
    frame.render_widget(activity, chunks[2]);
}

fn render_help_tab(frame: &mut ratatui::Frame, area: Rect) {
    let help_text = "\
Demiarch TUI - Agent Monitoring Dashboard
═══════════════════════════════════════════

NAVIGATION
  Tab / →      Next tab
  Shift+Tab / ←  Previous tab
  1-4          Jump to tab
  ↑ / k        Scroll up
  ↓ / j        Scroll down

DISPLAY
  a            Toggle ASCII/Unicode mode

GENERAL
  q            Quit
  ?            Show this help

AGENT HIERARCHY
  The agent tree shows the Russian Doll hierarchy:

  Level 1: Orchestrator (Session Director)
    └─ Spawns: Planner

  Level 2: Planner (Task Coordinator)
    └─ Spawns: Coder, Reviewer, Tester

  Level 3: Workers (Leaf Nodes)
    • Coder - Generates code
    • Reviewer - Reviews code
    • Tester - Creates tests

STATUS ICONS
  ○ Ready       ● Running      ◐ Waiting
  ✓ Completed   ✗ Failed       ⊘ Cancelled

For more information, visit:
  https://github.com/demiarch/demiarch";

    let help = Paragraph::new(help_text).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Help")
            .style(Style::default()),
    );
    frame.render_widget(help, area);
}

fn render_status_bar(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    // Build tree from live events for status bar
    let tree = TreeBuilder::from_live_events();

    // Split status bar into agent status and key hints
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Agent status bar
    let status_block = Block::default().borders(Borders::ALL);
    let inner_status = status_block.inner(chunks[0]);
    frame.render_widget(status_block, chunks[0]);

    let status_bar = AgentStatusBar::new(&tree).colors(TreeColors::default());
    frame.render_widget(status_bar, inner_status);

    // Key hints
    let mode_hint = if app.ascii_mode {
        "[ASCII]"
    } else {
        "[Unicode]"
    };
    let hints = Paragraph::new(format!(
        "q: Quit | Tab: Switch | ↑↓: Scroll | a: Toggle {} | ?: Help",
        mode_hint
    ))
    .style(Style::default().fg(Color::DarkGray))
    .block(Block::default().borders(Borders::ALL));
    frame.render_widget(hints, chunks[1]);
}
