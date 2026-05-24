//! Provider-neutral Skill service contract for Route C S6.
//!
//! The Skill service separates skill discovery, executable loading, tool
//! catalog, invocation, status, and snapshots from Web/CLI presentation code.
//! Commands are typed so policy, trace, entitlement readiness, and audit can be
//! validated before any concrete skill runtime is called.

use chrono::{DateTime, Utc};
use macaca_proto::{
    ApplicationId, CapabilityToolDescriptor, CapabilityToolInvocation,
    CapabilityToolInvocationResult, MacacaError, MacacaResult, TraceContext,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::evaluation::{
    SelfEvolutionEvaluationCheckpoint, SelfEvolutionEvaluationRecord, SelfEvolutionReportBuilder,
    SelfEvolutionScore,
};
use crate::governance::SkillTelemetryAggregate;
use crate::runtime::{SkillPolicy, SkillSnapshot};
use crate::snapshot::SkillRegistrySnapshot;

/// Stable service id used by runtime-host registration and SDK clients.
pub const SKILL_SERVICE_ID: &str = "service.skill";

/// Command names accepted by the Skill service provider adapter.
pub const SKILL_SNAPSHOT_COMMAND: &str = "skill.snapshot";
pub const SKILL_EXECUTABLE_LOAD_COMMAND: &str = "skill.executable.load";
pub const SKILL_TOOL_CATALOG_COMMAND: &str = "skill.tool.catalog";
pub const SKILL_TOOL_INVOKE_COMMAND: &str = "skill.tool.invoke";
pub const SKILL_STATUS_COMMAND: &str = "skill.status";
pub const SKILL_SERVICE_SNAPSHOT_COMMAND: &str = "skill.service.snapshot";
pub const SKILL_CLEANUP_COMMAND: &str = "skill.cleanup";
pub const SKILL_GOVERNANCE_RECORD_USAGE_COMMAND: &str = "skill.governance.record_usage";
pub const SKILL_GOVERNANCE_SNAPSHOT_COMMAND: &str = "skill.governance.snapshot";
pub const SKILL_CURATION_STATUS_COMMAND: &str = "skill.curation.status";
pub const SKILL_CURATION_DRY_RUN_COMMAND: &str = "skill.curation.dry_run";
pub const SKILL_CURATION_RUN_COMMAND: &str = "skill.curation.run";
pub const SKILL_CURATION_SNAPSHOT_COMMAND: &str = "skill.curation.snapshot";
pub const SKILL_CURATION_ROLLBACK_COMMAND: &str = "skill.curation.rollback";
pub const SKILL_CURATION_MERGE_APPLY_COMMAND: &str = "skill.curation.merge_apply";
pub const SKILL_CURATION_PIN_COMMAND: &str = "skill.curation.pin";
pub const SKILL_CURATION_UNPIN_COMMAND: &str = "skill.curation.unpin";
pub const SKILL_CURATION_ARCHIVE_COMMAND: &str = "skill.curation.archive";
pub const SKILL_CURATION_RESTORE_COMMAND: &str = "skill.curation.restore";
pub const SKILL_CURATION_QUARANTINE_COMMAND: &str = "skill.curation.quarantine";
pub const SKILL_CURATION_RELEASE_QUARANTINE_COMMAND: &str = "skill.curation.release_quarantine";
pub const SKILL_CURATION_SUPERSEDE_COMMAND: &str = "skill.curation.supersede";
pub const SKILL_CURATION_REJECT_COMMAND: &str = "skill.curation.reject";
pub const SKILL_ALIAS_UPSERT_COMMAND: &str = "skill.alias.upsert";
pub const SKILL_ALIAS_RESOLVE_COMMAND: &str = "skill.alias.resolve";
pub const SKILL_ALIAS_SNAPSHOT_COMMAND: &str = "skill.alias.snapshot";
pub const SKILL_EVOLUTION_PROPOSE_FROM_TASK_COMMAND: &str = "skill.evolution.propose_from_task";
pub const SKILL_EVOLUTION_PROPOSE_PATCH_COMMAND: &str = "skill.evolution.propose_patch";
pub const SKILL_EVOLUTION_PROMOTE_DRAFT_COMMAND: &str = "skill.evolution.promote_draft";
pub const SKILL_EVOLUTION_REJECT_DRAFT_COMMAND: &str = "skill.evolution.reject_draft";
pub const SKILL_EVOLUTION_SNAPSHOT_COMMAND: &str = "skill.evolution.snapshot";
pub const SKILL_EVOLUTION_PROCESSING_RUN_COMMAND: &str = "skill.evolution.processing.run";
pub const SKILL_EVOLUTION_PROCESSING_SNAPSHOT_COMMAND: &str = "skill.evolution.processing.snapshot";
pub const SKILL_EVOLUTION_MATERIALIZATION_APPLY_COMMAND: &str =
    "skill.evolution.materialization.apply";
pub const SKILL_EVOLUTION_MATERIALIZATION_OPERATOR_RUN_COMMAND: &str =
    "skill.evolution.materialization.operator.run";
pub const SKILL_EVOLUTION_MATERIALIZATION_OPERATOR_SNAPSHOT_COMMAND: &str =
    "skill.evolution.materialization.operator.snapshot";
pub const SKILL_EVALUATION_CHECKPOINT_APPEND_COMMAND: &str = "skill.evaluation.checkpoint.append";
pub const SKILL_EVALUATION_SCORE_COMMAND: &str = "skill.evaluation.score";
pub const SKILL_EVALUATION_REPORT_COMMAND: &str = "skill.evaluation.report";
pub use crate::mutation::SKILL_CONTENT_MUTATE_COMMAND;

/// Explicit scope for Skill service commands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillServiceScope {
    pub application_id: Option<ApplicationId>,
    pub session_id: Option<String>,
    /// Optional tenant boundary for future multi-tenant Skill governance.
    ///
    /// The Skill service treats tenant identity as routing and audit metadata
    /// only.  It must not infer application behavior from the value, but
    /// Store/EventLog providers need the reference to replay who owned a
    /// governance event when curation, promotion, or mutation becomes durable.
    #[serde(default)]
    pub tenant_id: Option<String>,
    pub agent_name: Option<String>,
}

impl SkillServiceScope {
    /// Build agent-scoped metadata for skill snapshot and invocation commands.
    pub fn agent(
        application_id: ApplicationId,
        session_id: impl Into<String>,
        agent_name: impl Into<String>,
    ) -> MacacaResult<Self> {
        Ok(Self {
            application_id: Some(application_id),
            session_id: Some(non_empty(
                session_id.into(),
                "skill service requires session_id",
            )?),
            tenant_id: None,
            agent_name: Some(non_empty(
                agent_name.into(),
                "skill service requires agent_name",
            )?),
        })
    }
}

impl Default for SkillServiceScope {
    fn default() -> Self {
        Self {
            application_id: None,
            session_id: None,
            tenant_id: None,
            agent_name: None,
        }
    }
}

/// Policy and package readiness hints for skill operations.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillServicePolicyHints {
    pub required_permissions: Vec<String>,
    pub entitlement_ready: Option<bool>,
    pub package_ready: Option<bool>,
    pub metadata: BTreeMap<String, String>,
}

/// Command for building an agent skill snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillSnapshotServiceCommand {
    pub trace: TraceContext,
    pub scope: SkillServiceScope,
    pub agent_name: String,
    pub workspace_dir: Option<PathBuf>,
    pub app_dir: Option<PathBuf>,
    pub include_instructions: bool,
    pub exposure_policy: SkillPolicy,
    pub policy: SkillServicePolicyHints,
}

/// Command for loading executable YAML skills from configured directories.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillExecutableLoadCommand {
    pub trace: TraceContext,
    pub scope: SkillServiceScope,
    pub directories: Vec<PathBuf>,
    pub policy: SkillServicePolicyHints,
}

impl SkillExecutableLoadCommand {
    /// Build a traced load command for one or more directories.
    pub fn new(trace: TraceContext, directories: Vec<PathBuf>) -> MacacaResult<Self> {
        validate_trace(&trace, "skill executable load command requires trace_id")?;
        Ok(Self {
            trace,
            scope: SkillServiceScope::default(),
            directories,
            policy: SkillServicePolicyHints::default(),
        })
    }
}

/// Command for reading sanitized executable skill tools.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillToolCatalogCommand {
    pub trace: TraceContext,
    pub scope: SkillServiceScope,
    pub include_disabled: bool,
}

/// Command for invoking one skill-owned tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillToolInvokeCommand {
    pub invocation: CapabilityToolInvocation,
}

/// Command for lightweight skill status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillStatusCommand {
    pub trace: TraceContext,
    pub scope: SkillServiceScope,
}

/// Command for deterministic Skill service snapshots.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillServiceSnapshotCommand {
    pub trace: TraceContext,
    pub scope: SkillServiceScope,
    pub include_registry: bool,
}

/// Command for skill resource cleanup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillCleanupCommand {
    pub trace: TraceContext,
    pub scope: SkillServiceScope,
}

/// Command for deterministic self-evolution evaluation scoring.
///
/// The command carries a sanitized evaluation record instead of raw task output.
/// Runtime-host providers can score the record through a Strategy implementation
/// while Store/EventLog, policy, and audit decorators retain traceability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillEvaluationScoreCommand {
    pub trace: TraceContext,
    pub scope: SkillServiceScope,
    pub record: SelfEvolutionEvaluationRecord,
}

/// Command for appending one sanitized checkpoint to an evaluation record.
///
/// This keeps checkpoint observation behind the Skill service contract. Shells
/// submit typed refs and counts through SDK/SystemFacade; they never interpret
/// white-box gates or mutate scoring semantics locally.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillEvaluationCheckpointAppendCommand {
    pub trace: TraceContext,
    pub scope: SkillServiceScope,
    pub record: SelfEvolutionEvaluationRecord,
    pub checkpoint: SelfEvolutionEvaluationCheckpoint,
}

/// Result for a checkpoint append operation.
///
/// Counts are bounded diagnostics for logs and reports. The result returns the
/// updated record so callers can persist it through Store/EventLog without the
/// command handler storing raw checkpoint payloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillEvaluationCheckpointAppendResult {
    pub record: SelfEvolutionEvaluationRecord,
    pub checkpoint_count: usize,
    pub audit_event_count: usize,
}

impl SkillEvaluationCheckpointAppendResult {
    /// Apply a checkpoint append command to a cloned evaluation record.
    pub fn from_command(command: &SkillEvaluationCheckpointAppendCommand) -> Self {
        let mut record = command.record.clone();
        record.white_box.append_checkpoint(&command.checkpoint);
        let checkpoint_count = populated_checkpoint_count(&record);
        let audit_event_count = record.white_box.audit_event_ids.len();

        tracing::info!(
            trace_id = %command.trace.trace_id,
            evaluation_id = %record.evaluation_id,
            task_family_id = %record.task_family_id,
            checkpoint_kind = ?command.checkpoint.kind,
            checkpoint_count,
            audit_event_count,
            "appended sanitized self-evolution evaluation checkpoint"
        );

        Self {
            record,
            checkpoint_count,
            audit_event_count,
        }
    }
}

/// Result for deterministic self-evolution evaluation scoring.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillEvaluationScoreResult {
    pub score: SelfEvolutionScore,
}

fn populated_checkpoint_count(record: &SelfEvolutionEvaluationRecord) -> usize {
    let white_box = &record.white_box;
    [
        white_box.verified_task_completion_ref.as_ref(),
        white_box.experience_candidate_ref.as_ref(),
        white_box.classification_ref.as_ref(),
        white_box.proposal_id.as_ref(),
        white_box.curation_run_id.as_ref(),
        white_box.promotion_or_apply_ref.as_ref(),
        white_box.active_catalog_snapshot_ref.as_ref(),
        white_box.later_skill_activation_ref.as_ref(),
    ]
    .into_iter()
    .filter(|value| value.is_some())
    .count()
}

/// Command for building sanitized self-evolution evaluation reports.
///
/// Report generation is explicit so shells can request bounded operator output
/// without learning scoring semantics or reconstructing checkpoint evidence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillEvaluationReportCommand {
    pub trace: TraceContext,
    pub scope: SkillServiceScope,
    pub record: SelfEvolutionEvaluationRecord,
    pub score: SelfEvolutionScore,
    pub include_markdown: bool,
}

/// Result for sanitized self-evolution evaluation reports.
///
/// JSON is the durable machine-readable summary. Markdown is optional operator
/// presentation text and remains generated from the same sanitized refs/counts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillEvaluationReportResult {
    pub score: SelfEvolutionScore,
    pub json_report: serde_json::Value,
    pub markdown_report: Option<String>,
}

impl SkillEvaluationReportResult {
    /// Build a sanitized report result from the typed report command.
    pub fn from_command(command: &SkillEvaluationReportCommand) -> Self {
        let markdown_report = command
            .include_markdown
            .then(|| SelfEvolutionReportBuilder::markdown_summary(&command.record, &command.score));

        Self {
            score: command.score.clone(),
            json_report: SelfEvolutionReportBuilder::json_summary(&command.record, &command.score),
            markdown_report,
        }
    }
}

/// Structured load result for executable skills.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillExecutableLoadResult {
    pub loaded: usize,
    pub failed: usize,
    pub skipped: usize,
    pub captured_at: DateTime<Utc>,
    pub failures: Vec<String>,
}

impl SkillExecutableLoadResult {
    /// Build a successful load result.
    pub fn loaded(loaded: usize) -> Self {
        Self {
            loaded,
            failed: 0,
            skipped: 0,
            captured_at: Utc::now(),
            failures: Vec::new(),
        }
    }
}

/// Result for executable skill tool catalog.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillToolCatalogResult {
    pub tools: Vec<CapabilityToolDescriptor>,
    pub captured_at: DateTime<Utc>,
}

impl SkillToolCatalogResult {
    /// Build a sanitized catalog result.
    pub fn new(tools: Vec<CapabilityToolDescriptor>) -> Self {
        Self {
            tools,
            captured_at: Utc::now(),
        }
    }
}

/// Lightweight skill service status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillStatusResult {
    pub service_id: String,
    pub healthy: bool,
    pub snapshot_skills: usize,
    pub executable_skills: usize,
    #[serde(default)]
    pub telemetry_aggregate: SkillTelemetryAggregate,
    pub captured_at: DateTime<Utc>,
}

/// Deterministic Skill service snapshot.
///
/// The snapshot is sanitized by construction.  It may include registry
/// metadata, but it must not serialize full `SKILL.md` bodies unless a future
/// trusted context-provider command explicitly requests instruction content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillServiceSnapshot {
    pub service_id: String,
    pub healthy: bool,
    pub agent_snapshot: Option<SkillSnapshot>,
    pub executable_registry: Option<SkillRegistrySnapshot>,
    pub tool_count: usize,
    pub last_audit_ids: Vec<String>,
    pub captured_at: DateTime<Utc>,
}

impl SkillServiceSnapshot {
    /// Build a snapshot from optional skill sources.
    pub fn new(
        agent_snapshot: Option<SkillSnapshot>,
        executable_registry: Option<SkillRegistrySnapshot>,
        tool_count: usize,
    ) -> Self {
        Self {
            service_id: SKILL_SERVICE_ID.into(),
            healthy: true,
            agent_snapshot,
            executable_registry,
            tool_count,
            last_audit_ids: Vec::new(),
            captured_at: Utc::now(),
        }
    }

    /// Build a structured unavailable snapshot for hosts without skills.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            service_id: SKILL_SERVICE_ID.into(),
            healthy: false,
            agent_snapshot: None,
            executable_registry: None,
            tool_count: 0,
            last_audit_ids: vec![reason.into()],
            captured_at: Utc::now(),
        }
    }
}

/// Result returned by agent skill snapshot commands.
pub type SkillSnapshotServiceResult = SkillSnapshot;

/// Result returned by skill tool invocation.
pub type SkillToolInvokeResult = CapabilityToolInvocationResult;

fn validate_trace(trace: &TraceContext, message: &'static str) -> MacacaResult<()> {
    if trace.trace_id.trim().is_empty() {
        return Err(MacacaError::Config(message.into()));
    }
    Ok(())
}

fn non_empty(value: String, message: &'static str) -> MacacaResult<String> {
    let trimmed = value.trim().to_string();
    if trimmed.is_empty() {
        return Err(MacacaError::Config(message.into()));
    }
    Ok(trimmed)
}
