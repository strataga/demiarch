//! Agent Visualization Module
//!
//! Provides visual tree display of agent hierarchy and status.
//! Supports both text-based CLI output and structured data for TUI rendering.
//!
//! # Features
//!
//! - **Tree Rendering**: Build and render agent hierarchy trees with Unicode or ASCII characters
//! - **Ratatui Widgets**: `HierarchyTreeWidget` and `AgentStatusBar` for TUI integration
//! - **Flexible Styling**: Customizable colors, icons, and display options
//! - **Status Tracking**: Visual indicators for agent states (running, completed, failed)
//!
//! # Example
//!
//! ```ignore
//! use demiarch_core::visualization::{TreeBuilder, HierarchyTree, RenderOptions};
//!
//! // Build a tree from shared agent state
//! let tree = TreeBuilder::from_shared_state(&state).await;
//!
//! // Render as text
//! let renderer = HierarchyTree::new(tree);
//! println!("{}", renderer.render_with_summary());
//!
//! // Or use with ratatui (in TUI context)
//! use demiarch_core::visualization::{HierarchyTreeWidget, TreeColors};
//! let widget = HierarchyTreeWidget::new(&tree)
//!     .colors(TreeColors::default())
//!     .block(Block::default().title("Agents").borders(Borders::ALL));
//! ```

mod tree;
mod widget;

pub use tree::{AgentTreeNode, HierarchyTree, NodeStyle, RenderOptions, StatusIcon, TreeBuilder};
pub use widget::{AgentStatusBar, HierarchyTreeWidget, TreeColors};
