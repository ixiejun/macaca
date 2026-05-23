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

/// Resolution behavior applied when a consumer follows an alias record.
///
/// The policy is part of Skill service governance metadata so Task, Scheduler,
/// Context, and shell consumers do not need to invent their own redirect rules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillAliasResolutionPolicy {
    Redirect,
    WarnAndRedirect,
    Deny,
}

/// Bounded decision emitted by alias resolution.
///
/// Consumers can log and persist this status without copying rationale text or
/// loading skill bodies.  The enum keeps audit surfaces stable as curation adds
/// more alias policies over time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillAliasResolutionStatus {
    Miss,
    Redirected,
    WarnAndRedirected,
    Denied,
    Expired,
    LoopPrevented,
}

/// Sanitized alias record from one skill identity to another.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillAliasRecord {
    pub source_skill_id: String,
    pub source_name: String,
    pub target_skill_id: String,
    pub target_name: String,
    pub kind: SkillAliasKind,
    pub resolution_policy: SkillAliasResolutionPolicy,
    pub valid_from: DateTime<Utc>,
    pub valid_until: Option<DateTime<Utc>>,
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

    /// Stable lookup key for the target side of the alias.
    pub fn target_key(&self) -> String {
        alias_key(&self.target_skill_id, &self.target_name)
    }

    /// Return whether the alias can be considered active at the given instant.
    ///
    /// This treats future-valid records as inactive too, which prevents clock
    /// skew or replayed mementos from accidentally activating redirects early.
    pub fn is_active_at(&self, now: DateTime<Utc>) -> bool {
        self.valid_from <= now && self.valid_until.map(|until| until > now).unwrap_or(true)
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
    pub status: SkillAliasResolutionStatus,
    pub target_skill_id: Option<String>,
    pub target_name: Option<String>,
    pub kind: Option<SkillAliasKind>,
    pub resolution_policy: Option<SkillAliasResolutionPolicy>,
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
            status: SkillAliasResolutionStatus::Miss,
            target_skill_id: None,
            target_name: None,
            kind: None,
            resolution_policy: None,
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
        let status = match &record.resolution_policy {
            SkillAliasResolutionPolicy::Redirect => SkillAliasResolutionStatus::Redirected,
            SkillAliasResolutionPolicy::WarnAndRedirect => {
                SkillAliasResolutionStatus::WarnAndRedirected
            }
            SkillAliasResolutionPolicy::Deny => SkillAliasResolutionStatus::Denied,
        };
        let resolved = !matches!(status, SkillAliasResolutionStatus::Denied);
        Self::from_record(command, record, captured_at, status, resolved)
    }

    /// Build a non-redirecting decision from a stored alias record.
    pub fn blocked(
        command: &SkillAliasResolveCommand,
        record: SkillAliasRecord,
        captured_at: DateTime<Utc>,
        status: SkillAliasResolutionStatus,
    ) -> Self {
        Self::from_record(command, record, captured_at, status, false)
    }

    fn from_record(
        command: &SkillAliasResolveCommand,
        record: SkillAliasRecord,
        captured_at: DateTime<Utc>,
        status: SkillAliasResolutionStatus,
        resolved: bool,
    ) -> Self {
        Self {
            requested_skill_id: command.skill_id.clone(),
            requested_name: command.name.clone(),
            resolved,
            status,
            target_skill_id: resolved.then_some(record.target_skill_id),
            target_name: resolved.then_some(record.target_name),
            kind: Some(record.kind),
            resolution_policy: Some(record.resolution_policy),
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
