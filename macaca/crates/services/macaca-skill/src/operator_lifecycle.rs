//! Provider-neutral Skill operator lifecycle contracts.
//!
//! These DTOs model the operator surface for AgentSkills-format knowledge
//! skills.  The service owns catalog visibility, bounded markdown reads,
//! config persistence, watch notifications, enablement, and audit references;
//! shells and applications only submit commands and render results.

use std::collections::BTreeMap;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use macaca_proto::TraceContext;
use serde::{Deserialize, Serialize};

use crate::service_contract::{SkillServicePolicyHints, SkillServiceScope};

pub const SKILL_OPERATOR_CATALOG_LIST_COMMAND: &str = "skill.operator.catalog.list";
pub const SKILL_OPERATOR_MARKDOWN_READ_COMMAND: &str = "skill.operator.markdown.read";
pub const SKILL_OPERATOR_CONFIG_WRITE_COMMAND: &str = "skill.operator.config.write";
pub const SKILL_OPERATOR_WATCH_COMMAND: &str = "skill.operator.watch";
pub const SKILL_OPERATOR_UNWATCH_COMMAND: &str = "skill.operator.unwatch";
pub const SKILL_OPERATOR_CHANGED_COMMAND: &str = "skill.operator.changed";
pub const SKILL_OPERATOR_ENABLEMENT_COMMAND: &str = "skill.operator.enablement";

/// Source class used for service-owned precedence and diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SkillOperatorSourceKind {
    Workspace = 0,
    Application = 1,
    User = 2,
    Managed = 3,
    PluginProvided = 4,
}

impl SkillOperatorSourceKind {
    /// Stable label used in logs, snapshots, and audit payloads.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Application => "application",
            Self::Workspace => "workspace",
            Self::User => "user",
            Self::Managed => "managed",
            Self::PluginProvided => "plugin_provided",
        }
    }
}

/// One configured source root for catalog discovery and watch state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillOperatorSourceDescriptor {
    pub kind: SkillOperatorSourceKind,
    pub root: PathBuf,
    #[serde(default)]
    pub provider_id: Option<String>,
}

impl SkillOperatorSourceDescriptor {
    pub fn new(kind: SkillOperatorSourceKind, root: PathBuf) -> Self {
        Self {
            kind,
            root,
            provider_id: None,
        }
    }
}

/// Catalog list command for operator-visible knowledge skills.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillOperatorCatalogListCommand {
    pub trace: TraceContext,
    pub scope: SkillServiceScope,
    pub sources: Vec<SkillOperatorSourceDescriptor>,
    pub include_disabled: bool,
    pub policy: SkillServicePolicyHints,
}

/// Bounded catalog row safe for UI, context planning, and diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillOperatorCatalogEntry {
    pub name: String,
    pub description: String,
    pub source_kind: SkillOperatorSourceKind,
    pub source_label: String,
    pub location: PathBuf,
    pub enabled: bool,
    pub provenance_ref: String,
}

/// Result for catalog listing with precedence diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillOperatorCatalogListResult {
    pub skills: Vec<SkillOperatorCatalogEntry>,
    pub source_count: usize,
    pub skipped_shadowed: usize,
    pub visibility_generation: u64,
    pub captured_at: DateTime<Utc>,
}

/// Command for reading one skill markdown body with byte bounds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillOperatorMarkdownReadCommand {
    pub trace: TraceContext,
    pub scope: SkillServiceScope,
    pub skill_name: String,
    pub sources: Vec<SkillOperatorSourceDescriptor>,
    pub max_bytes: usize,
    pub policy: SkillServicePolicyHints,
}

/// Bounded markdown read result.  Raw full skill bodies never enter snapshots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillOperatorMarkdownReadResult {
    pub skill_name: String,
    pub source_kind: SkillOperatorSourceKind,
    pub markdown: String,
    pub bytes_read: usize,
    pub truncated: bool,
    pub provenance_ref: String,
    pub captured_at: DateTime<Utc>,
}

/// Command for policy-gated config persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillOperatorConfigWriteCommand {
    pub trace: TraceContext,
    pub scope: SkillServiceScope,
    pub skill_name: String,
    pub values: BTreeMap<String, String>,
    pub policy: SkillServicePolicyHints,
}

/// Result for config persistence.  Values are redacted by key/value heuristic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillOperatorConfigWriteResult {
    pub skill_name: String,
    pub redacted_values: BTreeMap<String, String>,
    pub audit_event_ids: Vec<String>,
    pub captured_at: DateTime<Utc>,
}

/// Command that registers one source for change notification handling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillOperatorWatchCommand {
    pub trace: TraceContext,
    pub scope: SkillServiceScope,
    pub source: SkillOperatorSourceDescriptor,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillOperatorWatchResult {
    pub watched_sources: usize,
    pub captured_at: DateTime<Utc>,
}

/// Command that removes one watched source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillOperatorUnwatchCommand {
    pub trace: TraceContext,
    pub scope: SkillServiceScope,
    pub source: SkillOperatorSourceDescriptor,
}

/// Command emitted by a watcher adapter after a skill source changes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillOperatorChangedCommand {
    pub trace: TraceContext,
    pub scope: SkillServiceScope,
    pub skill_name: String,
    pub source: SkillOperatorSourceDescriptor,
    pub change_kind: String,
    pub summary: String,
}

/// Bounded changed notification for context/tool visibility refresh.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillOperatorChangedEvent {
    pub skill_name: String,
    pub source_kind: SkillOperatorSourceKind,
    pub change_kind: String,
    pub sanitized_summary: String,
    pub captured_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillOperatorChangedResult {
    pub changed: SkillOperatorChangedEvent,
    pub visibility_generation: u64,
    pub audit_event_ids: Vec<String>,
}

/// Command for service-owned enablement changes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillOperatorEnablementCommand {
    pub trace: TraceContext,
    pub scope: SkillServiceScope,
    pub skill_name: String,
    pub enabled: bool,
    pub reason: String,
    pub policy: SkillServicePolicyHints,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillOperatorEnablementResult {
    pub skill_name: String,
    pub enabled: bool,
    pub visibility_generation: u64,
    pub audit_event_ids: Vec<String>,
    pub captured_at: DateTime<Utc>,
}
