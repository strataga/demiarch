//! Agent domain service

use super::{
    entity::{Agent, AgentType},
    repository::AgentRepository,
};
use anyhow::Result;
use chrono::Utc;
use uuid::Uuid;

pub struct AgentService {
    repository: Box<dyn AgentRepository>,
}

impl AgentService {
    pub fn new(repository: Box<dyn AgentRepository>) -> Self {
        Self { repository }
    }

    pub async fn create_agent(&self, name: String, agent_type: AgentType) -> Result<Agent> {
        let now = Utc::now();
        let agent = Agent {
            id: Uuid::new_v4(),
            name,
            agent_type,
            capabilities: vec![],
            created_at: now,
            updated_at: now,
        };
        self.repository.create(&agent).await?;
        Ok(agent)
    }
}
