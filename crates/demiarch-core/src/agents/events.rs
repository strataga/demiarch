//! Agent Event Stream
//!
//! Provides real-time event streaming for agent lifecycle events.
//! Events are written to a JSONL file that can be watched by the TUI.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::sync::Mutex;
use uuid::Uuid;

use super::{AgentId, AgentStatus, AgentType};

/// Path to the agent events file
pub fn events_file_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".demiarch")
        .join("agent-events.jsonl")
}

/// Agent lifecycle event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEvent {
    /// Event timestamp
    pub timestamp: DateTime<Utc>,
    /// Unique event ID
    pub event_id: Uuid,
    /// Session ID for grouping events from same execution
    pub session_id: Uuid,
    /// Event type
    pub event_type: AgentEventType,
    /// Agent details
    pub agent: AgentEventData,
}

/// Type of agent event
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AgentEventType {
    /// Agent was spawned
    Spawned,
    /// Agent started executing
    Started,
    /// Agent status updated
    StatusUpdate,
    /// Agent completed successfully
    Completed,
    /// Agent failed
    Failed,
    /// Agent was cancelled
    Cancelled,
    /// Token usage update
    TokenUpdate,
}

/// Agent data included in events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEventData {
    /// Agent ID
    pub id: String,
    /// Agent type
    pub agent_type: String,
    /// Agent name
    pub name: String,
    /// Parent agent ID (if not root)
    pub parent_id: Option<String>,
    /// Agent path in hierarchy
    pub path: String,
    /// Current status
    pub status: String,
    /// Token count
    pub tokens: u64,
    /// Task description
    pub task: Option<String>,
    /// Error message (if failed)
    pub error: Option<String>,
}

/// Event writer for streaming agent events to file
pub struct AgentEventWriter {
    session_id: Uuid,
    file: Mutex<Option<File>>,
}

impl AgentEventWriter {
    /// Create a new event writer
    pub fn new() -> Self {
        let path = events_file_path();

        // Ensure directory exists
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        // Open file for appending (create if doesn't exist)
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .ok();

        Self {
            session_id: Uuid::new_v4(),
            file: Mutex::new(file),
        }
    }

    /// Get the session ID
    pub fn session_id(&self) -> Uuid {
        self.session_id
    }

    /// Write an event to the file
    pub fn write_event(&self, event_type: AgentEventType, agent: AgentEventData) {
        let event = AgentEvent {
            timestamp: Utc::now(),
            event_id: Uuid::new_v4(),
            session_id: self.session_id,
            event_type,
            agent,
        };

        if let Ok(mut file_guard) = self.file.lock()
            && let Some(ref mut file) = *file_guard
                && let Ok(json) = serde_json::to_string(&event) {
                    let _ = writeln!(file, "{}", json);
                    let _ = file.flush();
                }
    }

    /// Emit a spawned event
    pub fn emit_spawned(
        &self,
        id: &AgentId,
        agent_type: AgentType,
        name: &str,
        parent_id: Option<&AgentId>,
        path: &str,
        task: Option<&str>,
    ) {
        self.write_event(
            AgentEventType::Spawned,
            AgentEventData {
                id: id.to_string(),
                agent_type: agent_type.to_string(),
                name: name.to_string(),
                parent_id: parent_id.map(|p| p.to_string()),
                path: path.to_string(),
                status: "spawned".to_string(),
                tokens: 0,
                task: task.map(|t| t.to_string()),
                error: None,
            },
        );
    }

    /// Emit a status update event
    pub fn emit_status_update(&self, id: &AgentId, status: AgentStatus, tokens: u64) {
        self.write_event(
            AgentEventType::StatusUpdate,
            AgentEventData {
                id: id.to_string(),
                agent_type: String::new(),
                name: String::new(),
                parent_id: None,
                path: String::new(),
                status: status.to_string(),
                tokens,
                task: None,
                error: None,
            },
        );
    }

    /// Emit a completed event
    pub fn emit_completed(&self, id: &AgentId, tokens: u64) {
        self.write_event(
            AgentEventType::Completed,
            AgentEventData {
                id: id.to_string(),
                agent_type: String::new(),
                name: String::new(),
                parent_id: None,
                path: String::new(),
                status: "completed".to_string(),
                tokens,
                task: None,
                error: None,
            },
        );
    }

    /// Emit a failed event
    pub fn emit_failed(&self, id: &AgentId, error: &str) {
        self.write_event(
            AgentEventType::Failed,
            AgentEventData {
                id: id.to_string(),
                agent_type: String::new(),
                name: String::new(),
                parent_id: None,
                path: String::new(),
                status: "failed".to_string(),
                tokens: 0,
                task: None,
                error: Some(error.to_string()),
            },
        );
    }
}

impl Default for AgentEventWriter {
    fn default() -> Self {
        Self::new()
    }
}

/// Event reader for watching agent events
pub struct AgentEventReader {
    reader: BufReader<File>,
    /// Only read events from this session (if set)
    filter_session: Option<Uuid>,
}

impl AgentEventReader {
    /// Create a new event reader
    pub fn new() -> Option<Self> {
        let path = events_file_path();
        let file = File::open(&path).ok()?;
        Some(Self {
            reader: BufReader::new(file),
            filter_session: None,
        })
    }

    /// Filter to only events from a specific session
    pub fn with_session_filter(mut self, session_id: Uuid) -> Self {
        self.filter_session = Some(session_id);
        self
    }

    /// Read the next event (blocking)
    pub fn next_event(&mut self) -> Option<AgentEvent> {
        loop {
            let mut line = String::new();
            match self.reader.read_line(&mut line) {
                Ok(0) => return None, // EOF
                Ok(_) => {
                    if let Ok(event) = serde_json::from_str::<AgentEvent>(&line) {
                        // Apply session filter if set
                        if let Some(filter) = self.filter_session
                            && event.session_id != filter {
                                continue;
                            }
                        return Some(event);
                    }
                }
                Err(_) => return None,
            }
        }
    }

    /// Read all available events (non-blocking)
    pub fn read_all(&mut self) -> Vec<AgentEvent> {
        let mut events = Vec::new();
        while let Some(event) = self.next_event() {
            events.push(event);
        }
        events
    }

    /// Get the latest session ID from events
    pub fn latest_session_id(&mut self) -> Option<Uuid> {
        self.read_all().last().map(|e| e.session_id)
    }
}

/// Clear old events from the file
pub fn clear_events() {
    let path = events_file_path();
    let _ = std::fs::write(&path, "");
}

/// Read recent events (last N)
pub fn read_recent_events(count: usize) -> Vec<AgentEvent> {
    let path = events_file_path();
    let file = match File::open(&path) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };

    let reader = BufReader::new(file);
    let mut events: Vec<AgentEvent> = reader
        .lines()
        .map_while(Result::ok)
        .filter_map(|line| serde_json::from_str(&line).ok())
        .collect();

    // Return last N events
    if events.len() > count {
        events.drain(0..events.len() - count);
    }
    events
}

/// Read events from the most recent session
pub fn read_current_session_events() -> Vec<AgentEvent> {
    let all_events = read_recent_events(1000);
    if let Some(last) = all_events.last() {
        let session_id = last.session_id;
        all_events
            .into_iter()
            .filter(|e| e.session_id == session_id)
            .collect()
    } else {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_serialization() {
        let event = AgentEvent {
            timestamp: Utc::now(),
            event_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            event_type: AgentEventType::Spawned,
            agent: AgentEventData {
                id: "abc123".to_string(),
                agent_type: "orchestrator".to_string(),
                name: "orchestrator-0".to_string(),
                parent_id: None,
                path: "orchestrator-0".to_string(),
                status: "spawned".to_string(),
                tokens: 0,
                task: Some("Build a hello world app".to_string()),
                error: None,
            },
        };

        let json = serde_json::to_string(&event).unwrap();
        let parsed: AgentEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.event_type, AgentEventType::Spawned);
    }
}
