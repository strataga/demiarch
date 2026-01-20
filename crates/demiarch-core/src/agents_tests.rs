//! Agents module tests

use crate::agents::AgentType;
use crate::agents::{CoderAgent, OrchestratorAgent, PlannerAgent, ReviewerAgent, TesterAgent};

#[test]
fn test_agent_type_display_orchestrator() {
    assert_eq!(AgentType::Orchestrator.to_string(), "orchestrator");
}

#[test]
fn test_agent_type_display_planner() {
    assert_eq!(AgentType::Planner.to_string(), "planner");
}

#[test]
fn test_agent_type_display_coder() {
    assert_eq!(AgentType::Coder.to_string(), "coder");
}

#[test]
fn test_agent_type_display_reviewer() {
    assert_eq!(AgentType::Reviewer.to_string(), "reviewer");
}

#[test]
fn test_agent_type_display_tester() {
    assert_eq!(AgentType::Tester.to_string(), "tester");
}

#[test]
fn test_agent_type_eq() {
    assert_eq!(AgentType::Orchestrator, AgentType::Orchestrator);
    assert_eq!(AgentType::Planner, AgentType::Planner);
    assert_eq!(AgentType::Coder, AgentType::Coder);
    assert_eq!(AgentType::Reviewer, AgentType::Reviewer);
    assert_eq!(AgentType::Tester, AgentType::Tester);
}

#[test]
fn test_agent_type_ne() {
    assert_ne!(AgentType::Orchestrator, AgentType::Planner);
    assert_ne!(AgentType::Planner, AgentType::Coder);
    assert_ne!(AgentType::Coder, AgentType::Reviewer);
    assert_ne!(AgentType::Reviewer, AgentType::Tester);
    assert_ne!(AgentType::Tester, AgentType::Orchestrator);
}

#[test]
fn test_agent_type_clone() {
    let agent = AgentType::Orchestrator;
    let cloned = agent.clone();
    assert_eq!(agent, cloned);
}

#[test]
fn test_agent_type_debug() {
    let format = format!("{:?}", AgentType::Orchestrator);
    assert!(format.contains("Orchestrator"));
}

#[test]
fn test_all_agent_types() {
    let all_types = [
        AgentType::Orchestrator,
        AgentType::Planner,
        AgentType::Coder,
        AgentType::Reviewer,
        AgentType::Tester,
    ];

    // Verify all have unique string representations
    let strings: Vec<String> = all_types
        .iter()
        .map(|a: &AgentType| a.to_string())
        .collect();
    let unique_strings: std::collections::HashSet<String> = strings.iter().cloned().collect();

    assert_eq!(strings.len(), unique_strings.len());
    assert_eq!(strings.len(), 5);
}

#[test]
fn test_agent_type_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<AgentType>();
}

// Agent implementation tests

#[test]
fn test_orchestrator_agent_new() {
    let agent = OrchestratorAgent::new();
    assert_eq!(agent.agent_type(), AgentType::Orchestrator);
}

#[test]
fn test_orchestrator_agent_process() {
    let agent = OrchestratorAgent::new();
    let result = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(agent.process("test request"));
    assert!(result.contains("test request"));
}

#[test]
fn test_planner_agent_new() {
    let agent = PlannerAgent::new();
    assert_eq!(agent.agent_type(), AgentType::Planner);
}

#[test]
fn test_planner_agent_plan() {
    let agent = PlannerAgent::new();
    let tasks = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(agent.plan("test feature"));
    assert_eq!(tasks.len(), 2);
}

#[test]
fn test_coder_agent_new() {
    let agent = CoderAgent::new();
    assert_eq!(agent.agent_type(), AgentType::Coder);
}

#[test]
fn test_coder_agent_generate() {
    let agent = CoderAgent::new();
    let result = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(agent.generate("test task"));
    assert!(result.contains("test task"));
}

#[test]
fn test_reviewer_agent_new() {
    let agent = ReviewerAgent::new();
    assert_eq!(agent.agent_type(), AgentType::Reviewer);
}

#[test]
fn test_reviewer_agent_review() {
    let agent = ReviewerAgent::new();
    let result = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(agent.review("let x = 1;"));
    assert!(result.contains("lines of code"));
}

#[test]
fn test_tester_agent_new() {
    let agent = TesterAgent::new();
    assert_eq!(agent.agent_type(), AgentType::Tester);
}

#[test]
fn test_tester_agent_test() {
    let agent = TesterAgent::new();
    let result = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(agent.test("fn main() {}"));
    assert!(result.contains("bytes"));
}
