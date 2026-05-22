//! Skill alias contracts for curation-safe identity resolution.
//!
//! Alias records are governance metadata, not filesystem mutations.  Future
//! curation providers can record that one skill was absorbed into another while
//! schedulers, tasks, and context providers keep their original references and
//! resolve them through the Skill service boundary.

use chrono::{DateTime, Utc};
use macaca_proto::TraceContext;
use serde::{Deserialize, Serialize};

use crate::service_contract::SkillServiceScope;

/// Relationship kind used by the service-owned skill alias map.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillAliasKind {
    Redirect,
    SupersededBy,
    AbsorbedInto,
}

/// Sanitized alias record from one skill identity to another.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillAliasRecord {
    pub source_skill_id: String,
    pub source_name: String,
    pub target_skill_id: String,
    pub target_name: String,
    pub kind: SkillAliasKind,
    pub rationale: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub evidence_ids: Vec<String>,
}

impl SkillAliasRecord {
    /// Stable lookup key used by providers before a dedicated id index exists.
    pub fn key(&self) -> String {
        alias_key(&self.source_skill_id, &self.source_name)
    }
}

/// Command for adding or replacing one sanitized skill alias record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillAliasUpsertCommand {
    pub trace: TraceContext,
    pub scope: SkillServiceScope,
    pub record: SkillAliasRecord,
}

/// Result returned after alias upsert.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillAliasUpsertResult {
    pub record: SkillAliasRecord,
    pub captured_at: DateTime<Utc>,
}

/// Command for resolving one skill identity through the alias map.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillAliasResolveCommand {
    pub trace: TraceContext,
    pub scope: SkillServiceScope,
    pub skill_id: String,
    pub name: Option<String>,
}

impl SkillAliasResolveCommand {
    /// Stable lookup key derived from the preferred id and optional name.
    pub fn key(&self) -> String {
        alias_key(&self.skill_id, self.name.as_deref().unwrap_or_default())
    }
}

/// Result returned by alias resolution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillAliasResolveResult {
    pub requested_skill_id: String,
    pub requested_name: Option<String>,
    pub resolved: bool,
    pub target_skill_id: Option<String>,
    pub target_name: Option<String>,
    pub kind: Option<SkillAliasKind>,
    pub rationale: Option<String>,
    pub evidence_ids: Vec<String>,
    pub captured_at: DateTime<Utc>,
}

impl SkillAliasResolveResult {
    /// Build an unresolved result without inventing fallback replacements.
    pub fn unresolved(command: &SkillAliasResolveCommand, captured_at: DateTime<Utc>) -> Self {
        Self {
            requested_skill_id: command.skill_id.clone(),
            requested_name: command.name.clone(),
            resolved: false,
            target_skill_id: None,
            target_name: None,
            kind: None,
            rationale: None,
            evidence_ids: Vec::new(),
            captured_at,
        }
    }

    /// Build a resolved result from a stored alias record.
    pub fn resolved(
        command: &SkillAliasResolveCommand,
        record: SkillAliasRecord,
        captured_at: DateTime<Utc>,
    ) -> Self {
        Self {
            requested_skill_id: command.skill_id.clone(),
            requested_name: command.name.clone(),
            resolved: true,
            target_skill_id: Some(record.target_skill_id),
            target_name: Some(record.target_name),
            kind: Some(record.kind),
            rationale: Some(record.rationale),
            evidence_ids: record.evidence_ids,
            captured_at,
        }
    }
}

/// Command for reading all sanitized alias records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillAliasSnapshotCommand {
    pub trace: TraceContext,
    pub scope: SkillServiceScope,
}

/// Diagnostic alias snapshot result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillAliasSnapshotResult {
    pub aliases: Vec<SkillAliasRecord>,
    pub captured_at: DateTime<Utc>,
}

fn alias_key(skill_id: &str, name: &str) -> String {
    let trimmed_id = skill_id.trim();
    if trimmed_id.is_empty() {
        name.trim().to_string()
    } else {
        trimmed_id.to_string()
    }
}
