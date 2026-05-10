//! Skill runtime lifecycle handle.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Lifecycle state for a provisioned skill runtime handle.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SkillRuntimeState {
    Installed,
    Provisioned,
    Active,
    Error(String),
    Released,
}

/// Additive lifecycle handle returned by provisioning/runtime operations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillRuntimeHandle {
    pub skill_id: String,
    pub client_id: String,
    pub target_dir: PathBuf,
    pub state: SkillRuntimeState,
}

impl SkillRuntimeHandle {
    pub fn provisioned(
        skill_id: impl Into<String>,
        client_id: impl Into<String>,
        target_dir: PathBuf,
    ) -> Self {
        Self {
            skill_id: skill_id.into(),
            client_id: client_id.into(),
            target_dir,
            state: SkillRuntimeState::Provisioned,
        }
    }

    pub fn released(mut self) -> Self {
        self.state = SkillRuntimeState::Released;
        self
    }
}
