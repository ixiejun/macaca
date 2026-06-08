//! Shared test fixtures for A2A contract tests.
//!
//! Uses neutral harness identifiers only — no application-specific agent names.

use crate::a2a::{AgentCapabilities, AgentCard, AgentSkillInfo};

/// Build a minimal [`AgentCard`] for serde and resolver contract tests.
pub(super) fn sample_card() -> AgentCard {
    AgentCard {
        name: "test-agent".into(),
        url: "http://localhost:9000".into(),
        version: "1.0.0".into(),
        description: Some("A test agent".into()),
        capabilities: AgentCapabilities::default(),
        default_input_modes: vec!["text/plain".into()],
        default_output_modes: vec!["text/plain".into()],
        skills: vec![AgentSkillInfo {
            name: "echo".into(),
            description: Some("Echoes back the input".into()),
        }],
    }
}
