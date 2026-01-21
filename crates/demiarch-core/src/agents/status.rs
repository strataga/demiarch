//! Status tracking for agents
//!
//! Provides a thread-safe status tracking mechanism used by all agents.

use std::sync::atomic::{AtomicU8, Ordering};

use super::traits::AgentStatus;

/// Thread-safe status tracker for agents
///
/// Encapsulates the atomic status management used by all agent types.
/// Uses AtomicU8 for lock-free status updates.
pub struct StatusTracker {
    status: AtomicU8,
}

impl StatusTracker {
    /// Create a new status tracker in Ready state
    pub fn new() -> Self {
        Self {
            status: AtomicU8::new(AgentStatus::Ready as u8),
        }
    }

    /// Set the current status
    pub fn set(&self, status: AgentStatus) {
        self.status.store(status as u8, Ordering::SeqCst);
    }

    /// Get the current status
    pub fn get(&self) -> AgentStatus {
        match self.status.load(Ordering::SeqCst) {
            0 => AgentStatus::Ready,
            1 => AgentStatus::Running,
            2 => AgentStatus::WaitingForChildren,
            3 => AgentStatus::Completed,
            4 => AgentStatus::Failed,
            _ => AgentStatus::Cancelled,
        }
    }

    /// Check if currently in a terminal state
    pub fn is_terminal(&self) -> bool {
        self.get().is_terminal()
    }

    /// Check if currently active
    pub fn is_active(&self) -> bool {
        self.get().is_active()
    }
}

impl Default for StatusTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_tracker_new() {
        let tracker = StatusTracker::new();
        assert_eq!(tracker.get(), AgentStatus::Ready);
    }

    #[test]
    fn test_status_tracker_set_get() {
        let tracker = StatusTracker::new();

        tracker.set(AgentStatus::Running);
        assert_eq!(tracker.get(), AgentStatus::Running);

        tracker.set(AgentStatus::Completed);
        assert_eq!(tracker.get(), AgentStatus::Completed);
    }

    #[test]
    fn test_status_tracker_terminal() {
        let tracker = StatusTracker::new();
        assert!(!tracker.is_terminal());

        tracker.set(AgentStatus::Completed);
        assert!(tracker.is_terminal());
    }

    #[test]
    fn test_status_tracker_active() {
        let tracker = StatusTracker::new();
        assert!(!tracker.is_active());

        tracker.set(AgentStatus::Running);
        assert!(tracker.is_active());
    }
}
