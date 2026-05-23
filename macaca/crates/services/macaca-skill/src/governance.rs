//! Skill governance contracts for self-evolving agent knowledge.
//!
//! This module intentionally stores governance data outside `SKILL.md` content.
//! Agent-authored experience can evolve over time, but the OS must keep that
//! evolution traceable, reversible, and policy-ready without polluting the
//! human-readable skill instructions.  The types here are provider-neutral DTOs:
//! runtime-host can back them with an in-memory store today, while a future
//! Store/EventLog-backed provider can implement the same command surface.

use chrono::{DateTime, Duration, Utc};
use macaca_proto::TraceContext;
use serde::{Deserialize, Serialize};

use crate::lifecycle::SkillCurationLifecycleCommand;
use crate::service_contract::SkillServiceScope;
pub use crate::telemetry::{SkillUsageEventKind, SkillUsageObservation, SkillUsageTelemetry};

/// Lifecycle state for a governed AgentSkill.
///
/// These states are provider-neutral service DTO labels, not filesystem
/// locations.  The enum is intentionally complete before all transition
/// commands exist, so Store/EventLog, Context, Task, SDK, and shell adapters can
/// exchange lifecycle snapshots without inventing local labels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillLifecycleState {
    Draft,
    Active,
    Stale,
    Archived,
    Quarantined,
    Superseded,
    Rejected,
}

impl Default for SkillLifecycleState {
    fn default() -> Self {
        Self::Active
    }
}

/// Describes who originally authored or installed the skill.
///
/// This is a coarse trust and routing hint.  The OS must not infer governance
/// eligibility from paths alone because user, package, and agent-created skills
/// can share the same directory layout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillAuthorKind {
    User,
    Agent,
    System,
    Package,
    Unknown,
}

impl Default for SkillAuthorKind {
    fn default() -> Self {
        Self::Unknown
    }
}

/// Sanitized provenance attached to a governed skill.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillGovernanceProvenance {
    pub skill_id: String,
    pub name: String,
    pub source: String,
    pub source_scope: String,
    pub author_kind: SkillAuthorKind,
    pub created_by: Option<String>,
    pub trust_level: Option<String>,
    pub created_at: DateTime<Utc>,
    pub evidence_ids: Vec<String>,
}

impl SkillGovernanceProvenance {
    /// Create minimal provenance from sanitized skill identity facts.
    pub fn new(
        skill_id: impl Into<String>,
        name: impl Into<String>,
        source: impl Into<String>,
        source_scope: impl Into<String>,
        author_kind: SkillAuthorKind,
    ) -> Self {
        Self {
            skill_id: skill_id.into(),
            name: name.into(),
            source: source.into(),
            source_scope: source_scope.into(),
            author_kind,
            created_by: None,
            trust_level: None,
            created_at: Utc::now(),
            evidence_ids: Vec::new(),
        }
    }
}

/// Full governance record for one skill.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillGovernanceRecord {
    pub provenance: SkillGovernanceProvenance,
    pub lifecycle: SkillLifecycleState,
    pub pinned: bool,
    pub telemetry: SkillUsageTelemetry,
    pub updated_at: DateTime<Utc>,
    pub evidence_ids: Vec<String>,
}

impl SkillGovernanceRecord {
    /// Create a record from the first observed event.
    pub fn from_observation(
        observation: &SkillUsageObservation,
        observed_at: DateTime<Utc>,
    ) -> Self {
        let mut provenance = SkillGovernanceProvenance::new(
            observation.key(),
            observation.name.clone(),
            observation.source.clone(),
            observation.source_scope.clone(),
            observation.author_kind.clone(),
        );
        provenance.created_by = observation.created_by.clone();
        if let Some(evidence_id) = &observation.evidence_id {
            provenance.evidence_ids.push(evidence_id.clone());
        }

        let mut record = Self {
            provenance,
            lifecycle: SkillLifecycleState::Active,
            pinned: observation.pinned.unwrap_or(false),
            telemetry: SkillUsageTelemetry::default(),
            updated_at: observed_at,
            evidence_ids: Vec::new(),
        };
        record.apply(observation, observed_at);
        record
    }

    /// Apply a sanitized observation while preserving existing provenance.
    pub fn apply(&mut self, observation: &SkillUsageObservation, observed_at: DateTime<Utc>) {
        self.telemetry.record(&observation.event, observed_at);
        self.updated_at = observed_at;
        if let Some(pinned) = observation.pinned {
            self.pinned = pinned;
        }
        match observation.event {
            SkillUsageEventKind::Archived => self.lifecycle = SkillLifecycleState::Archived,
            SkillUsageEventKind::Restored => self.lifecycle = SkillLifecycleState::Active,
            SkillUsageEventKind::Quarantined => self.lifecycle = SkillLifecycleState::Quarantined,
            SkillUsageEventKind::QuarantineReleased => self.lifecycle = SkillLifecycleState::Active,
            SkillUsageEventKind::Superseded => self.lifecycle = SkillLifecycleState::Superseded,
            SkillUsageEventKind::Rejected => self.lifecycle = SkillLifecycleState::Rejected,
            SkillUsageEventKind::Pinned | SkillUsageEventKind::Unpinned => {}
            SkillUsageEventKind::Created
            | SkillUsageEventKind::Viewed
            | SkillUsageEventKind::Activated
            | SkillUsageEventKind::Used
            | SkillUsageEventKind::ResourceRead
            | SkillUsageEventKind::SuccessfulTask
            | SkillUsageEventKind::FailedTask
            | SkillUsageEventKind::Patched => {}
        }
        if let Some(evidence_id) = &observation.evidence_id {
            if !self.evidence_ids.contains(evidence_id) {
                self.evidence_ids.push(evidence_id.clone());
            }
        }
    }

    /// Create a metadata-only governance record for a lifecycle operation.
    ///
    /// Lifecycle commands can target a skill before usage telemetry has been
    /// observed by this provider.  The built-in provider records the sanitized
    /// identity and evidence so later snapshots can explain why the lifecycle
    /// state exists without reading or modifying the skill's instruction files.
    pub fn from_lifecycle_command(
        command: &SkillCurationLifecycleCommand,
        observed_at: DateTime<Utc>,
    ) -> Self {
        let mut provenance = SkillGovernanceProvenance::new(
            command.skill_id.clone(),
            command.name.clone(),
            command.source.clone(),
            command.source_scope.clone(),
            command.author_kind.clone(),
        );
        provenance.evidence_ids = non_empty_evidence(&command.evidence_ids);

        Self {
            provenance,
            lifecycle: SkillLifecycleState::Active,
            pinned: false,
            telemetry: SkillUsageTelemetry::default(),
            updated_at: observed_at,
            evidence_ids: non_empty_evidence(&command.evidence_ids),
        }
    }
}

/// Command for recording one sanitized skill usage observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillGovernanceRecordUsageCommand {
    pub trace: TraceContext,
    pub scope: SkillServiceScope,
    pub observation: SkillUsageObservation,
}

/// Result returned after a usage observation is recorded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillGovernanceRecordUsageResult {
    pub record: SkillGovernanceRecord,
    pub captured_at: DateTime<Utc>,
}

/// Command for reading a governance snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillGovernanceSnapshotCommand {
    pub trace: TraceContext,
    pub scope: SkillServiceScope,
    /// Backward-compatible archived visibility switch for older shell callers.
    ///
    /// New callers should prefer `lifecycle_filters` because the governance
    /// lifecycle set is now larger than active/archived.  Keeping this field
    /// preserves existing SDK and Web route behavior while the richer filter is
    /// rolled out across consumers.
    pub include_archived: bool,
    #[serde(default)]
    pub lifecycle_filters: Vec<SkillLifecycleState>,
}

/// Sanitized governance snapshot result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillGovernanceSnapshotResult {
    pub records: Vec<SkillGovernanceRecord>,
    pub captured_at: DateTime<Utc>,
}

/// Command for deterministic, non-destructive curation analysis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillCurationDryRunCommand {
    pub trace: TraceContext,
    pub scope: SkillServiceScope,
    pub stale_after_days: i64,
    pub narrow_use_threshold: u64,
}

/// Recommendation action names are intentionally non-destructive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillCurationAction {
    Keep,
    Protected,
    WouldMarkStale,
    WouldArchive,
    WouldReviewForConsolidation,
}

/// One deterministic recommendation in a curation dry-run report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillCurationRecommendation {
    pub skill_id: String,
    pub name: String,
    pub action: SkillCurationAction,
    pub rationale: String,
    pub protected: bool,
    pub confidence: f32,
    pub evidence_ids: Vec<String>,
}

/// Dry-run curation report produced without mutating files or state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillCurationDryRunResult {
    pub recommendations: Vec<SkillCurationRecommendation>,
    pub semantic_analysis_status: String,
    pub mutated: bool,
    pub captured_at: DateTime<Utc>,
}

impl SkillCurationDryRunResult {
    /// Build deterministic recommendations from governance records.
    pub fn from_records(
        records: impl IntoIterator<Item = SkillGovernanceRecord>,
        command: &SkillCurationDryRunCommand,
        now: DateTime<Utc>,
    ) -> Self {
        let stale_after = Duration::days(command.stale_after_days.max(0));
        let recommendations = records
            .into_iter()
            .map(|record| {
                recommend_for_record(record, stale_after, command.narrow_use_threshold, now)
            })
            .collect();

        Self {
            recommendations,
            semantic_analysis_status:
                "unavailable: deterministic dry-run did not call an LLM or similarity provider"
                    .into(),
            mutated: false,
            captured_at: now,
        }
    }
}

fn recommend_for_record(
    record: SkillGovernanceRecord,
    stale_after: Duration,
    narrow_use_threshold: u64,
    now: DateTime<Utc>,
) -> SkillCurationRecommendation {
    let skill_id = record.provenance.skill_id.clone();
    let name = record.provenance.name.clone();
    let evidence_ids = record.evidence_ids.clone();
    if record.pinned {
        return SkillCurationRecommendation {
            skill_id,
            name,
            action: SkillCurationAction::Protected,
            rationale:
                "skill is pinned, so curation may observe it but must not recommend mutation".into(),
            protected: true,
            confidence: 1.0,
            evidence_ids,
        };
    }

    let last_activity = record
        .telemetry
        .last_activity_at()
        .unwrap_or(record.provenance.created_at);
    let inactive_for = now.signed_duration_since(last_activity);
    let action = if record.lifecycle == SkillLifecycleState::Stale && inactive_for >= stale_after {
        SkillCurationAction::WouldArchive
    } else if inactive_for >= stale_after {
        SkillCurationAction::WouldMarkStale
    } else if record.provenance.author_kind == SkillAuthorKind::Agent
        && record.telemetry.use_count <= narrow_use_threshold
        && record.telemetry.patch_count == 0
    {
        SkillCurationAction::WouldReviewForConsolidation
    } else {
        SkillCurationAction::Keep
    };
    let rationale = match action {
        SkillCurationAction::Keep => {
            "skill has recent or sufficient usage for this deterministic policy".into()
        }
        SkillCurationAction::WouldMarkStale => {
            "skill has no recent usage beyond the configured stale threshold".into()
        }
        SkillCurationAction::WouldArchive => {
            "skill is already stale and remains inactive beyond the configured threshold".into()
        }
        SkillCurationAction::WouldReviewForConsolidation => {
            "agent-created skill has low usage and should be reviewed for umbrella consolidation"
                .into()
        }
        SkillCurationAction::Protected => "skill is protected".into(),
    };

    SkillCurationRecommendation {
        skill_id,
        name,
        action,
        rationale,
        protected: false,
        confidence: 0.75,
        evidence_ids,
    }
}

fn non_empty_evidence(evidence_ids: &[String]) -> Vec<String> {
    evidence_ids
        .iter()
        .filter(|id| !id.trim().is_empty())
        .cloned()
        .collect()
}
