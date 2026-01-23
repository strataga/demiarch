use super::entity::SkillsEntity;

/// Validates extracted skills before they become reusable
pub struct SkillValidator {
    min_success_rate: f32,
    min_unique_contexts: u32,
}

impl SkillValidator {
    pub fn new(min_success_rate: f32, min_unique_contexts: u32) -> Self {
        Self {
            min_success_rate,
            min_unique_contexts,
        }
    }

    /// Validate a candidate skill meets quality gates
    pub fn validate(&self, skill: &SkillsEntity) -> Result<(), String> {
        if skill.success_rate < self.min_success_rate {
            return Err(format!(
                "Success rate {} below minimum {}",
                skill.success_rate, self.min_success_rate
            ));
        }

        if skill.tested_contexts < self.min_unique_contexts {
            return Err(format!(
                "Only tested in {} contexts, minimum {} required",
                skill.tested_contexts, self.min_unique_contexts
            ));
        }

        Ok(())
    }
}
