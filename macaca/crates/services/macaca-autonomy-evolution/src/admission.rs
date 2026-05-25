//! Executable admission Specifications for evolution candidates.
//!
//! Admission is deliberately metadata-only. The evaluator receives bounded
//! candidate descriptors and evidence references, then emits deterministic gate
//! findings. It never opens package files, runs validation commands, reads raw
//! prompts, or embeds application-specific workflow decisions. This keeps the
//! control plane generic while preventing low-quality candidates from entering
//! later benchmark and release states.

use macaca_proto::{MacacaError, MacacaResult};
use tracing::{info, warn};

use crate::{
    AdmissionDecision, EvolutionAdmissionCandidate, EvolutionAdmissionCommand,
    EvolutionAdmissionFinding, EvolutionAdmissionResult, AUTONOMY_EVOLUTION_SERVICE_ID,
};

const MAX_REASON_LEN: usize = 160;
const MAX_TEXT_LEN: usize = 2_000;
const MAX_FINDING_EVIDENCE_REFS: usize = 4;

/// Strategy-ready interface for candidate admission quality checks.
///
/// Providers can replace this Specification with a stricter policy without
/// changing SDK callers or runtime-host command envelopes.
pub trait EvolutionAdmissionSpecification: Send + Sync {
    fn evaluate(
        &self,
        command: &EvolutionAdmissionCommand,
    ) -> MacacaResult<EvolutionAdmissionResult>;
}

/// Default admission policy used by the built-in provider.
#[derive(Debug, Default, Clone)]
pub struct DefaultEvolutionAdmissionSpecification;

#[derive(Debug, Clone, PartialEq, Eq)]
struct GateOutcome {
    gate: &'static str,
    decision: AdmissionDecision,
    reason_code: &'static str,
    missing_evidence: Option<&'static str>,
    evidence_refs: Vec<String>,
}

impl EvolutionAdmissionSpecification for DefaultEvolutionAdmissionSpecification {
    fn evaluate(
        &self,
        command: &EvolutionAdmissionCommand,
    ) -> MacacaResult<EvolutionAdmissionResult> {
        validate_command_shape(command)?;
        info!(
            service_id = AUTONOMY_EVOLUTION_SERVICE_ID,
            candidate_id = command.candidate.candidate_id.as_str(),
            trace_id = command.trace.trace_id.as_str(),
            "evolution admission quality gate evaluation started"
        );

        let outcomes = vec![
            semantic_package_name(&command.candidate),
            trigger_quality(&command.candidate),
            skill_body_focus(&command.candidate),
            resource_structure(&command.candidate),
            quick_validation_refs(&command.candidate),
            forward_test_refs(&command.candidate),
            duplicate_suppression(&command.candidate),
            stale_metadata_regeneration(&command.candidate),
        ];
        let decision = aggregate_decision(&outcomes);
        let missing_evidence = outcomes
            .iter()
            .filter_map(|outcome| outcome.missing_evidence.map(str::to_owned))
            .collect::<Vec<_>>();
        let summary_reason = summarize_reason(&outcomes, &decision);
        if decision != AdmissionDecision::Accepted {
            warn!(
                service_id = AUTONOMY_EVOLUTION_SERVICE_ID,
                candidate_id = command.candidate.candidate_id.as_str(),
                trace_id = command.trace.trace_id.as_str(),
                decision = ?decision,
                "evolution admission quality gate evaluation did not accept candidate"
            );
        }

        Ok(EvolutionAdmissionResult {
            candidate_id: command.candidate.candidate_id.clone(),
            target_type: command.candidate.target_type.clone(),
            decision,
            trace: command.trace.clone(),
            findings: outcomes
                .into_iter()
                .map(|outcome| EvolutionAdmissionFinding {
                    gate: outcome.gate.into(),
                    decision: outcome.decision,
                    reason_code: outcome.reason_code.into(),
                    evidence_refs: outcome.evidence_refs,
                })
                .collect(),
            missing_evidence,
            summary_reason,
            policy_decision_refs: command.policy_decision_refs.clone(),
            captured_at: chrono::Utc::now(),
        })
    }
}

fn validate_command_shape(command: &EvolutionAdmissionCommand) -> MacacaResult<()> {
    if command.actor_id.trim().is_empty() {
        return Err(MacacaError::Config(
            "evolution admission requires actor_id".into(),
        ));
    }
    if command.trace.trace_id.trim().is_empty() {
        return Err(MacacaError::Config(
            "evolution admission requires trace_id".into(),
        ));
    }
    if command.candidate.candidate_id.trim().is_empty() {
        return Err(MacacaError::Config(
            "evolution admission requires candidate_id".into(),
        ));
    }
    if command.evidence_refs.is_empty() {
        return Err(MacacaError::Config(
            "evolution admission requires evidence refs".into(),
        ));
    }
    if command.policy_decision_refs.is_empty() {
        return Err(MacacaError::Config(
            "evolution admission requires policy decision refs".into(),
        ));
    }
    Ok(())
}

fn semantic_package_name(candidate: &EvolutionAdmissionCandidate) -> GateOutcome {
    let name = candidate.package_name.trim();
    let generated = name.starts_with("skill-exp-")
        || name.starts_with("skill_exp_")
        || name.starts_with("exp-")
        || name.chars().all(|ch| ch.is_ascii_digit() || ch == '-');
    if name.len() < 8 || generated || !name.contains('-') {
        return outcome(
            "semantic_package_name",
            AdmissionDecision::Denied,
            "meaningless_or_generated_package_name",
            None,
            Vec::new(),
        );
    }
    outcome(
        "semantic_package_name",
        AdmissionDecision::Accepted,
        "semantic_name_present",
        None,
        Vec::new(),
    )
}

fn trigger_quality(candidate: &EvolutionAdmissionCandidate) -> GateOutcome {
    let good_trigger = candidate
        .trigger_descriptions
        .iter()
        .any(|item| meaningful_text(item, 24));
    if good_trigger {
        outcome(
            "trigger_quality",
            AdmissionDecision::Accepted,
            "trigger_quality_present",
            None,
            Vec::new(),
        )
    } else {
        outcome(
            "trigger_quality",
            AdmissionDecision::NeedsEvidence,
            "missing_trigger_quality",
            Some("trigger_quality"),
            Vec::new(),
        )
    }
}

fn skill_body_focus(candidate: &EvolutionAdmissionCandidate) -> GateOutcome {
    if candidate.skill_summary.len() > MAX_TEXT_LEN {
        return outcome(
            "skill_body_focus",
            AdmissionDecision::Quarantined,
            "skill_summary_exceeds_bounded_size",
            Some("focused_skill_summary"),
            Vec::new(),
        );
    }
    if meaningful_text(&candidate.skill_summary, 32) {
        outcome(
            "skill_body_focus",
            AdmissionDecision::Accepted,
            "focused_skill_summary_present",
            None,
            Vec::new(),
        )
    } else {
        outcome(
            "skill_body_focus",
            AdmissionDecision::NeedsEvidence,
            "missing_focused_skill_summary",
            Some("focused_skill_summary"),
            Vec::new(),
        )
    }
}

fn resource_structure(candidate: &EvolutionAdmissionCandidate) -> GateOutcome {
    let has_skill_md = candidate
        .resource_categories
        .iter()
        .any(|item| item.eq_ignore_ascii_case("skill.md"));
    if has_skill_md {
        outcome(
            "resource_structure",
            AdmissionDecision::Accepted,
            "declared_resource_structure_present",
            None,
            Vec::new(),
        )
    } else {
        outcome(
            "resource_structure",
            AdmissionDecision::NeedsEvidence,
            "missing_declared_skill_resource",
            Some("resource_structure"),
            Vec::new(),
        )
    }
}

fn quick_validation_refs(candidate: &EvolutionAdmissionCandidate) -> GateOutcome {
    evidence_gate(
        "quick_validation",
        "quick_validation_refs_present",
        "missing_quick_validation_refs",
        "quick_validation",
        &candidate.quick_validation_refs,
    )
}

fn forward_test_refs(candidate: &EvolutionAdmissionCandidate) -> GateOutcome {
    evidence_gate(
        "forward_test",
        "forward_test_refs_present",
        "missing_forward_test_refs",
        "forward_test",
        &candidate.forward_test_refs,
    )
}

fn duplicate_suppression(candidate: &EvolutionAdmissionCandidate) -> GateOutcome {
    if candidate.duplicate_candidate_refs.is_empty() {
        return outcome(
            "duplicate_suppression",
            AdmissionDecision::Accepted,
            "no_duplicate_candidate_refs",
            None,
            Vec::new(),
        );
    }
    outcome(
        "duplicate_suppression",
        AdmissionDecision::Denied,
        "duplicate_candidate_declared",
        None,
        bounded_refs(&candidate.duplicate_candidate_refs),
    )
}

fn stale_metadata_regeneration(candidate: &EvolutionAdmissionCandidate) -> GateOutcome {
    if candidate.metadata_stale {
        outcome(
            "metadata_freshness",
            AdmissionDecision::NeedsEvidence,
            "metadata_regeneration_required",
            Some("metadata_regeneration"),
            Vec::new(),
        )
    } else {
        outcome(
            "metadata_freshness",
            AdmissionDecision::Accepted,
            "metadata_is_fresh",
            None,
            Vec::new(),
        )
    }
}

fn evidence_gate(
    gate: &'static str,
    accepted_code: &'static str,
    missing_code: &'static str,
    missing_key: &'static str,
    refs: &[String],
) -> GateOutcome {
    if refs.is_empty() {
        outcome(
            gate,
            AdmissionDecision::NeedsEvidence,
            missing_code,
            Some(missing_key),
            Vec::new(),
        )
    } else {
        outcome(
            gate,
            AdmissionDecision::Accepted,
            accepted_code,
            None,
            bounded_refs(refs),
        )
    }
}

fn aggregate_decision(outcomes: &[GateOutcome]) -> AdmissionDecision {
    if outcomes
        .iter()
        .any(|outcome| outcome.decision == AdmissionDecision::Denied)
    {
        AdmissionDecision::Denied
    } else if outcomes
        .iter()
        .any(|outcome| outcome.decision == AdmissionDecision::Quarantined)
    {
        AdmissionDecision::Quarantined
    } else if outcomes
        .iter()
        .any(|outcome| outcome.decision == AdmissionDecision::NeedsEvidence)
    {
        AdmissionDecision::NeedsEvidence
    } else {
        AdmissionDecision::Accepted
    }
}

fn summarize_reason(outcomes: &[GateOutcome], decision: &AdmissionDecision) -> Option<String> {
    if decision == &AdmissionDecision::Accepted {
        return None;
    }
    let mut reason = outcomes
        .iter()
        .filter(|outcome| outcome.decision != AdmissionDecision::Accepted)
        .map(|outcome| format!("{}:{}", outcome.gate, outcome.reason_code))
        .collect::<Vec<_>>()
        .join(",");
    if reason.len() > MAX_REASON_LEN {
        reason.truncate(MAX_REASON_LEN);
    }
    Some(reason)
}

fn outcome(
    gate: &'static str,
    decision: AdmissionDecision,
    reason_code: &'static str,
    missing_evidence: Option<&'static str>,
    evidence_refs: Vec<String>,
) -> GateOutcome {
    GateOutcome {
        gate,
        decision,
        reason_code,
        missing_evidence,
        evidence_refs,
    }
}

fn meaningful_text(value: &str, min_len: usize) -> bool {
    let trimmed = value.trim();
    trimmed.len() >= min_len && trimmed.split_whitespace().count() >= 4
}

fn bounded_refs(refs: &[String]) -> Vec<String> {
    refs.iter()
        .take(MAX_FINDING_EVIDENCE_REFS)
        .map(|item| {
            if item.len() > MAX_REASON_LEN {
                let mut bounded = item.clone();
                bounded.truncate(MAX_REASON_LEN);
                bounded
            } else {
                item.clone()
            }
        })
        .collect()
}
