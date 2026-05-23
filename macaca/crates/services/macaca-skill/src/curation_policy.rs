//! Deterministic Skill curation policy.
//!
//! The policy lives outside the curation DTO module so report contracts remain
//! small and reviewable.  It is intentionally provider-neutral: callers pass
//! sanitized governance records, and the evaluator returns bounded
//! recommendations without reading skill files, shell state, prompts, provider
//! payloads, or application-specific data.

use chrono::{DateTime, Duration, Utc};

use crate::curation::{
    SkillCurationAction, SkillCurationDryRunResult, SkillCurationPhase, SkillCurationRecommendation,
};
use crate::governance::{SkillAuthorKind, SkillGovernanceRecord, SkillLifecycleState};
use crate::semantic_review::SkillSemanticReviewResult;
use crate::telemetry::SkillUsageTelemetry;

const DEFAULT_MAX_SKILL_CURATION_SIZE_BYTES: u64 = 64 * 1024;

/// Build a deterministic report for governance records.
///
/// The function is the first Strategy-independent curation phase.  Semantic
/// review providers can later decorate this result, but they must not bypass
/// these service-owned safety phases or mutate active skill content directly.
pub(crate) fn deterministic_report(
    records: impl IntoIterator<Item = SkillGovernanceRecord>,
    stale_after_days: i64,
    narrow_use_threshold: u64,
    now: DateTime<Utc>,
) -> SkillCurationDryRunResult {
    let stale_after = Duration::days(stale_after_days.max(0));
    let recommendations = records
        .into_iter()
        .map(|record| recommend_for_record(record, stale_after, narrow_use_threshold, now))
        .collect();

    let semantic_review = SkillSemanticReviewResult::unavailable(now);

    SkillCurationDryRunResult {
        recommendations,
        semantic_analysis_status: semantic_review.status_message(),
        semantic_review,
        mutated: false,
        captured_at: now,
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
    if record.pinned || is_protected_author_kind(&record.provenance.author_kind) {
        return SkillCurationRecommendation {
            skill_id,
            name,
            action: SkillCurationAction::Protected,
            phases: vec![SkillCurationPhase::Protected],
            rationale: "skill is pinned or protected by ownership, so curation may observe it but must not recommend mutation".into(),
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
    let phases = phases_for_record(&record, inactive_for, stale_after, narrow_use_threshold);
    let action = action_for_phases(&phases);
    let rationale = rationale_for_action(&action);

    SkillCurationRecommendation {
        skill_id,
        name,
        action,
        phases,
        rationale,
        protected: false,
        confidence: 0.75,
        evidence_ids,
    }
}

fn phases_for_record(
    record: &SkillGovernanceRecord,
    inactive_for: Duration,
    stale_after: Duration,
    narrow_use_threshold: u64,
) -> Vec<SkillCurationPhase> {
    let mut phases = Vec::new();
    if record.lifecycle == SkillLifecycleState::Quarantined
        || record.diagnostics.quarantine_required
    {
        phases.push(SkillCurationPhase::Quarantine);
    }
    if record
        .diagnostics
        .size_bytes
        .is_some_and(|size| size > DEFAULT_MAX_SKILL_CURATION_SIZE_BYTES)
    {
        phases.push(SkillCurationPhase::Size);
    }
    if record.diagnostics.invalid_metadata {
        phases.push(SkillCurationPhase::InvalidMetadata);
    }
    if !record.diagnostics.missing_dependency_refs.is_empty() {
        phases.push(SkillCurationPhase::MissingDependency);
    }
    if record.lifecycle == SkillLifecycleState::Stale && inactive_for >= stale_after {
        phases.push(SkillCurationPhase::Archive);
    } else if inactive_for >= stale_after {
        phases.push(SkillCurationPhase::Stale);
    }
    if should_review_for_consolidation(&record.telemetry, narrow_use_threshold) {
        phases.push(SkillCurationPhase::Consolidation);
    }
    if phases.is_empty() {
        phases.push(SkillCurationPhase::Keep);
    }
    phases
}

fn action_for_phases(phases: &[SkillCurationPhase]) -> SkillCurationAction {
    if phases.contains(&SkillCurationPhase::Quarantine) {
        SkillCurationAction::WouldQuarantine
    } else if phases.contains(&SkillCurationPhase::Size) {
        SkillCurationAction::WouldReduceSize
    } else if phases.contains(&SkillCurationPhase::InvalidMetadata) {
        SkillCurationAction::WouldFixMetadata
    } else if phases.contains(&SkillCurationPhase::MissingDependency) {
        SkillCurationAction::WouldResolveMissingDependency
    } else if phases.contains(&SkillCurationPhase::Archive) {
        SkillCurationAction::WouldArchive
    } else if phases.contains(&SkillCurationPhase::Stale) {
        SkillCurationAction::WouldMarkStale
    } else if phases.contains(&SkillCurationPhase::Consolidation) {
        SkillCurationAction::WouldReviewForConsolidation
    } else {
        SkillCurationAction::Keep
    }
}

fn rationale_for_action(action: &SkillCurationAction) -> String {
    match action {
        SkillCurationAction::Keep => {
            "skill has recent or sufficient usage for this deterministic policy".into()
        }
        SkillCurationAction::WouldQuarantine => {
            "skill carries quarantine diagnostics or is already quarantined".into()
        }
        SkillCurationAction::WouldReduceSize => {
            "skill package diagnostics exceed the bounded curation size policy".into()
        }
        SkillCurationAction::WouldFixMetadata => {
            "skill metadata diagnostics are invalid and require review before promotion".into()
        }
        SkillCurationAction::WouldResolveMissingDependency => {
            "skill declares missing dependency diagnostics that block safe reuse".into()
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
    }
}

fn is_protected_author_kind(author_kind: &SkillAuthorKind) -> bool {
    matches!(
        author_kind,
        SkillAuthorKind::System | SkillAuthorKind::Package
    )
}

fn should_review_for_consolidation(
    telemetry: &SkillUsageTelemetry,
    narrow_use_threshold: u64,
) -> bool {
    let total_effective_task_outcomes = telemetry
        .successful_task_count
        .saturating_add(telemetry.failed_task_count);
    telemetry.use_count <= narrow_use_threshold
        && telemetry.patch_count == 0
        && telemetry.successful_task_count == 0
        && total_effective_task_outcomes <= narrow_use_threshold
}
