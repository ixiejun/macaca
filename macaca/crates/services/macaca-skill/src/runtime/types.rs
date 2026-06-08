//! Snapshot DTOs (Memento) for frozen per-agent skill catalogs.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::agent_skill::{
    SkillEntry, SkillInstallSpec, SkillMcpServerConfig, SkillSourceScope,
};

/// Skill entry visible in a frozen snapshot (model + audit facing).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillSnapshotEntry {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub source_location: PathBuf,
    #[serde(default)]
    pub source_base_dir: PathBuf,
    pub location: PathBuf,
    pub base_dir: PathBuf,
    pub source: String,
    pub source_scope: SkillSourceScope,
    pub primary_env: Option<String>,
    pub required_env: Vec<String>,
    pub install: Vec<SkillInstallSpec>,
    pub mcp_servers: Vec<SkillMcpServerConfig>,
}

impl SkillSnapshotEntry {
    /// Map a filtered in-memory `SkillEntry` into an immutable snapshot row.
    pub(crate) fn from_skill_entry(entry: SkillEntry) -> Self {
        Self {
            name: entry.skill.name.clone(),
            description: entry.skill.description.clone(),
            source_location: entry.skill.canonical_location.clone(),
            source_base_dir: entry.skill.canonical_base_dir.clone(),
            location: entry.skill.location.clone(),
            base_dir: entry.skill.base_dir.clone(),
            source: entry.skill.source.clone(),
            source_scope: entry.skill.source_scope,
            primary_env: entry.metadata.primary_env.clone(),
            required_env: entry.metadata.requires_env.clone(),
            install: entry.metadata.install.clone(),
            mcp_servers: entry.metadata.mcp_servers.clone(),
        }
    }
}

/// Filtered skill diagnostic (audit trail for denied skills).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FilteredSkill {
    pub name: String,
    pub reason: String,
    pub source: String,
}

/// Frozen skill catalog for one agent run/session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillSnapshot {
    pub agent: String,
    pub prompt: String,
    pub skills: Vec<SkillSnapshotEntry>,
    pub filtered: Vec<FilteredSkill>,
    pub truncated: bool,
    pub compact: bool,
    pub version: u64,
}
