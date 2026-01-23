//! Ratatui widget for agent hierarchy tree visualization
//!
//! Provides a `HierarchyTreeWidget` that can be rendered in ratatui TUIs.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph, Widget, Wrap},
};

use crate::agents::{AgentStatus, AgentType};

use super::{AgentTreeNode, NodeStyle, RenderOptions};

/// Color scheme for the tree widget
#[derive(Debug, Clone)]
pub struct TreeColors {
    /// Color for orchestrator nodes
    pub orchestrator: Color,
    /// Color for planner nodes
    pub planner: Color,
    /// Color for coder nodes
    pub coder: Color,
    /// Color for reviewer nodes
    pub reviewer: Color,
    /// Color for tester nodes
    pub tester: Color,
    /// Color for running status
    pub running: Color,
    /// Color for completed status
    pub completed: Color,
    /// Color for failed status
    pub failed: Color,
    /// Color for waiting status
    pub waiting: Color,
    /// Color for ready status
    pub ready: Color,
    /// Color for tree structure characters
    pub tree_chars: Color,
    /// Color for agent IDs
    pub id: Color,
    /// Color for token counts
    pub tokens: Color,
}

impl Default for TreeColors {
    fn default() -> Self {
        Self {
            orchestrator: Color::Magenta,
            planner: Color::Blue,
            coder: Color::Green,
            reviewer: Color::Yellow,
            tester: Color::Cyan,
            running: Color::Yellow,
            completed: Color::Green,
            failed: Color::Red,
            waiting: Color::Blue,
            ready: Color::Gray,
            tree_chars: Color::DarkGray,
            id: Color::DarkGray,
            tokens: Color::Cyan,
        }
    }
}

impl TreeColors {
    /// Get color for an agent type
    pub fn for_agent_type(&self, agent_type: AgentType) -> Color {
        match agent_type {
            AgentType::Orchestrator => self.orchestrator,
            AgentType::Planner => self.planner,
            AgentType::Coder => self.coder,
            AgentType::Reviewer => self.reviewer,
            AgentType::Tester => self.tester,
        }
    }

    /// Get color for an agent status
    pub fn for_status(&self, status: AgentStatus) -> Color {
        match status {
            AgentStatus::Ready => self.ready,
            AgentStatus::Running => self.running,
            AgentStatus::WaitingForChildren => self.waiting,
            AgentStatus::Completed => self.completed,
            AgentStatus::Failed => self.failed,
            AgentStatus::Cancelled => self.ready,
        }
    }
}

/// Widget for rendering agent hierarchy tree in ratatui
pub struct HierarchyTreeWidget<'a> {
    root: &'a AgentTreeNode,
    options: RenderOptions,
    colors: TreeColors,
    block: Option<Block<'a>>,
    /// Whether to show the summary header
    show_header: bool,
    /// Whether to show the footer with totals
    show_footer: bool,
    /// Scroll offset for long trees
    scroll_offset: usize,
    /// Currently selected node index (for highlighting)
    selected: Option<usize>,
}

impl<'a> HierarchyTreeWidget<'a> {
    /// Create a new tree widget
    pub fn new(root: &'a AgentTreeNode) -> Self {
        Self {
            root,
            options: RenderOptions::default(),
            colors: TreeColors::default(),
            block: None,
            show_header: true,
            show_footer: true,
            scroll_offset: 0,
            selected: None,
        }
    }

    /// Set the render options
    pub fn options(mut self, options: RenderOptions) -> Self {
        self.options = options;
        self
    }

    /// Set the color scheme
    pub fn colors(mut self, colors: TreeColors) -> Self {
        self.colors = colors;
        self
    }

    /// Set the block (border/title)
    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    /// Show or hide the header
    pub fn show_header(mut self, show: bool) -> Self {
        self.show_header = show;
        self
    }

    /// Show or hide the footer
    pub fn show_footer(mut self, show: bool) -> Self {
        self.show_footer = show;
        self
    }

    /// Set scroll offset for viewing long trees
    pub fn scroll(mut self, offset: usize) -> Self {
        self.scroll_offset = offset;
        self
    }

    /// Set the selected node index for highlighting
    pub fn selected(mut self, index: Option<usize>) -> Self {
        self.selected = index;
        self
    }

    /// Build styled lines from the tree
    fn build_lines(&self) -> Vec<Line<'a>> {
        let mut lines = Vec::new();

        // Header
        if self.show_header {
            lines.push(self.build_header_line());
            lines.push(Line::from("")); // Spacer
        }

        // Tree nodes
        let mut node_index = 0;
        self.build_node_lines(self.root, &mut lines, "", true, 0, &mut node_index);

        // Footer
        if self.show_footer {
            lines.push(Line::from("")); // Spacer
            lines.push(self.build_footer_line());
        }

        lines
    }

    fn build_header_line(&self) -> Line<'a> {
        let total = self.root.count();
        let active = self.root.count_active();
        let completed = self.root.count_completed();
        let failed = self.root.count_failed();

        Line::from(vec![
            Span::styled("Agents: ", Style::default().fg(Color::White)),
            Span::styled(total.to_string(), Style::default().fg(Color::Cyan)),
            Span::styled(" total, ", Style::default().fg(Color::DarkGray)),
            Span::styled(active.to_string(), Style::default().fg(self.colors.running)),
            Span::styled(" active, ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                completed.to_string(),
                Style::default().fg(self.colors.completed),
            ),
            Span::styled(" done, ", Style::default().fg(Color::DarkGray)),
            Span::styled(failed.to_string(), Style::default().fg(self.colors.failed)),
            Span::styled(" failed", Style::default().fg(Color::DarkGray)),
        ])
    }

    fn build_footer_line(&self) -> Line<'a> {
        let tokens = self.root.tree_tokens();

        Line::from(vec![
            Span::styled("Total tokens: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format_tokens(tokens),
                Style::default().fg(self.colors.tokens),
            ),
        ])
    }

    fn build_node_lines(
        &self,
        node: &AgentTreeNode,
        lines: &mut Vec<Line<'a>>,
        prefix: &str,
        is_last: bool,
        depth: usize,
        node_index: &mut usize,
    ) {
        // Check max depth
        if self.options.max_depth >= 0 && depth as i32 > self.options.max_depth {
            return;
        }

        // Get tree characters based on style
        let (branch, vertical) = match self.options.style {
            NodeStyle::Ascii => {
                if is_last {
                    ("`-- ", "    ")
                } else {
                    ("+-- ", "|   ")
                }
            }
            NodeStyle::Unicode | NodeStyle::Rounded => {
                if is_last {
                    ("└── ", "    ")
                } else {
                    ("├── ", "│   ")
                }
            }
        };

        let mut spans = Vec::new();

        // Prefix and branch characters
        if depth > 0 {
            spans.push(Span::styled(
                prefix.to_string(),
                Style::default().fg(self.colors.tree_chars),
            ));
            spans.push(Span::styled(
                branch.to_string(),
                Style::default().fg(self.colors.tree_chars),
            ));
        }

        // Status icon
        if self.options.show_status {
            let status_icon = super::StatusIcon::for_status(node.status, self.options.style);
            spans.push(Span::styled(
                format!("{} ", status_icon),
                Style::default().fg(self.colors.for_status(node.status)),
            ));
        }

        // Agent type icon
        if self.options.show_type_icons {
            let type_icon = super::StatusIcon::for_agent_type(node.agent_type, self.options.style);
            spans.push(Span::styled(
                format!("{} ", type_icon),
                Style::default().fg(self.colors.for_agent_type(node.agent_type)),
            ));
        }

        // Agent name
        let is_selected = self.selected == Some(*node_index);
        let name_style = if is_selected {
            Style::default()
                .fg(self.colors.for_agent_type(node.agent_type))
                .add_modifier(Modifier::BOLD | Modifier::REVERSED)
        } else {
            Style::default()
                .fg(self.colors.for_agent_type(node.agent_type))
                .add_modifier(Modifier::BOLD)
        };
        spans.push(Span::styled(node.display_name(), name_style));

        // Agent ID
        if self.options.show_ids {
            spans.push(Span::styled(
                format!(" [{}]", node.id),
                Style::default().fg(self.colors.id),
            ));
        }

        // Token usage
        if self.options.show_tokens {
            if let Some(tokens) = node.tokens_used {
                spans.push(Span::styled(
                    format!(" {}tok", format_tokens(tokens)),
                    Style::default().fg(self.colors.tokens),
                ));
            }
        }

        // Status text for terminal states
        if node.status.is_terminal() {
            let status_color = self.colors.for_status(node.status);
            spans.push(Span::styled(
                format!(" [{}]", node.status),
                Style::default().fg(status_color),
            ));
        }

        lines.push(Line::from(spans));
        *node_index += 1;

        // Render children
        let child_count = node.children.len();
        for (i, child) in node.children.iter().enumerate() {
            let is_last_child = i == child_count - 1;
            let new_prefix = if depth > 0 {
                format!("{}{}", prefix, vertical)
            } else {
                String::new()
            };
            self.build_node_lines(
                child,
                lines,
                &new_prefix,
                is_last_child,
                depth + 1,
                node_index,
            );
        }
    }
}

impl Widget for HierarchyTreeWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Handle block
        let inner_area = if let Some(ref block) = self.block {
            let inner = block.inner(area);
            block.clone().render(area, buf);
            inner
        } else {
            area
        };

        // Build the content
        let lines = self.build_lines();

        // Apply scroll offset
        let visible_lines: Vec<Line> = lines
            .into_iter()
            .skip(self.scroll_offset)
            .take(inner_area.height as usize)
            .collect();

        // Render as paragraph (handles wrapping and overflow)
        let paragraph = Paragraph::new(visible_lines).wrap(Wrap { trim: false });
        paragraph.render(inner_area, buf);
    }
}

/// Format token count for display (e.g., 1500 -> "1.5k")
fn format_tokens(tokens: u32) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}k", tokens as f64 / 1_000.0)
    } else {
        tokens.to_string()
    }
}

/// Compact status bar widget for showing agent status in a single line
pub struct AgentStatusBar<'a> {
    root: &'a AgentTreeNode,
    colors: TreeColors,
}

impl<'a> AgentStatusBar<'a> {
    /// Create a new status bar
    pub fn new(root: &'a AgentTreeNode) -> Self {
        Self {
            root,
            colors: TreeColors::default(),
        }
    }

    /// Set the color scheme
    pub fn colors(mut self, colors: TreeColors) -> Self {
        self.colors = colors;
        self
    }
}

impl Widget for AgentStatusBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let total = self.root.count();
        let active = self.root.count_active();
        let completed = self.root.count_completed();
        let failed = self.root.count_failed();
        let tokens = self.root.tree_tokens();

        let spans = vec![
            Span::styled("Agents ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}/{}", active, total),
                Style::default().fg(Color::White),
            ),
            Span::styled(" | ", Style::default().fg(Color::DarkGray)),
            Span::styled("✓", Style::default().fg(self.colors.completed)),
            Span::styled(format!("{} ", completed), Style::default().fg(Color::White)),
            Span::styled("✗", Style::default().fg(self.colors.failed)),
            Span::styled(format!("{} ", failed), Style::default().fg(Color::White)),
            Span::styled("| ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}tok", format_tokens(tokens)),
                Style::default().fg(self.colors.tokens),
            ),
        ];

        let line = Line::from(spans);
        let paragraph = Paragraph::new(line);
        paragraph.render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::visualization::TreeBuilder;

    #[test]
    fn test_format_tokens() {
        assert_eq!(format_tokens(500), "500");
        assert_eq!(format_tokens(1500), "1.5k");
        assert_eq!(format_tokens(15000), "15.0k");
        assert_eq!(format_tokens(1_500_000), "1.5M");
    }

    #[test]
    fn test_tree_colors_default() {
        let colors = TreeColors::default();
        assert_eq!(colors.for_agent_type(AgentType::Coder), Color::Green);
        assert_eq!(colors.for_status(AgentStatus::Running), Color::Yellow);
        assert_eq!(colors.for_status(AgentStatus::Completed), Color::Green);
    }

    #[test]
    fn test_widget_creation() {
        let tree = TreeBuilder::demo_tree();
        let widget = HierarchyTreeWidget::new(&tree)
            .show_header(true)
            .show_footer(true)
            .scroll(0);

        // Widget should be constructable
        assert!(widget.show_header);
        assert!(widget.show_footer);
    }

    #[test]
    fn test_build_lines() {
        let tree = TreeBuilder::demo_tree();
        let widget = HierarchyTreeWidget::new(&tree);
        let lines = widget.build_lines();

        // Should have header, nodes, and footer
        assert!(!lines.is_empty());
        // At minimum: 1 header + 1 spacer + 5 nodes + 1 spacer + 1 footer = 9 lines
        assert!(lines.len() >= 9);
    }

    #[test]
    fn test_status_bar() {
        let tree = TreeBuilder::demo_tree();
        let _bar = AgentStatusBar::new(&tree).colors(TreeColors::default());
        // Status bar should be constructable
    }
}
