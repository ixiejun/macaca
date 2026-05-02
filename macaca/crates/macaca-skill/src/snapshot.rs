//! Skill registry snapshot primitives.

use serde::{Deserialize, Serialize};

use crate::definition::SkillDefinition;

/// Reloadable snapshot of executable skill definitions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkillRegistrySnapshot {
    pub version: u64,
    pub skills: Vec<SkillDefinition>,
}

impl SkillRegistrySnapshot {
    pub fn new(mut skills: Vec<SkillDefinition>) -> Self {
        skills.sort_by(|a, b| a.name.cmp(&b.name));
        Self { version: 1, skills }
    }

    pub fn len(&self) -> usize {
        self.skills.len()
    }

    pub fn is_empty(&self) -> bool {
        self.skills.is_empty()
    }
}
