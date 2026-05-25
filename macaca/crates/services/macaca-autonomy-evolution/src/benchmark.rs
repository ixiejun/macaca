//! Normalized paired benchmark scoring for evolution candidates.
//!
//! This module owns scoring semantics only. It does not run workloads, call
//! providers, inspect raw artifacts, or understand application workflows. The
//! Strategy receives two already-collected measurements for the same generic
//! task family and returns a conservative pass/fail/inconclusive decision that
//! can be replayed from bounded evidence references.

use macaca_proto::{MacacaError, MacacaResult};
use tracing::{info, warn};

use crate::{
    BenchmarkDecision, EvolutionBenchmarkCommand, EvolutionBenchmarkMetrics,
    EvolutionBenchmarkResult, EvolutionBenchmarkScoreDelta, AUTONOMY_EVOLUTION_SERVICE_ID,
};

const MAX_REASON_CODES: usize = 8;
const QUALITY_TOLERANCE: i64 = 2;

/// Replaceable scoring Strategy for paired evolution benchmarks.
pub trait EvolutionBenchmarkScoringStrategy: Send + Sync {
    fn score(&self, command: &EvolutionBenchmarkCommand) -> MacacaResult<EvolutionBenchmarkResult>;
}

/// Conservative default Strategy for first-slice normalized benchmarking.
#[derive(Debug, Default, Clone)]
pub struct DefaultEvolutionBenchmarkScoringStrategy;

impl EvolutionBenchmarkScoringStrategy for DefaultEvolutionBenchmarkScoringStrategy {
    fn score(&self, command: &EvolutionBenchmarkCommand) -> MacacaResult<EvolutionBenchmarkResult> {
        validate_command_shape(command)?;
        info!(
            service_id = AUTONOMY_EVOLUTION_SERVICE_ID,
            benchmark_id = command.benchmark_id.as_str(),
            run_id = command.run_id.as_str(),
            trace_id = command.trace.trace_id.as_str(),
            "normalized evolution benchmark scoring started"
        );

        let mut reason_codes = Vec::new();
        collect_comparability_reasons(command, &mut reason_codes);
        if !reason_codes.is_empty() {
            warn!(
                service_id = AUTONOMY_EVOLUTION_SERVICE_ID,
                benchmark_id = command.benchmark_id.as_str(),
                reasons = ?reason_codes,
                "normalized evolution benchmark is inconclusive before scoring"
            );
            return Ok(result(
                command,
                BenchmarkDecision::Inconclusive,
                EvolutionBenchmarkScoreDelta::default(),
                reason_codes,
            ));
        }

        let baseline = command
            .baseline
            .metrics
            .as_ref()
            .expect("comparability check ensures baseline metrics exist");
        let candidate = command
            .candidate
            .metrics
            .as_ref()
            .expect("comparability check ensures candidate metrics exist");
        let delta = score_delta(baseline, candidate);

        if !command.candidate.regression_reason_codes.is_empty() {
            reason_codes.push("candidate_regression_reported".into());
            reason_codes.extend(bounded_regression_codes(
                &command.candidate.regression_reason_codes,
            ));
            return Ok(result(
                command,
                BenchmarkDecision::Failed,
                delta,
                reason_codes,
            ));
        }

        if !delta.quality_preserved {
            return Ok(result(
                command,
                BenchmarkDecision::Failed,
                delta,
                vec!["quality_regressed".into()],
            ));
        }

        if delta.efficiency_improved {
            Ok(result(
                command,
                BenchmarkDecision::Passed,
                delta,
                vec!["quality_preserved_efficiency_improved".into()],
            ))
        } else {
            Ok(result(
                command,
                BenchmarkDecision::Inconclusive,
                delta,
                vec!["no_material_efficiency_improvement".into()],
            ))
        }
    }
}

fn validate_command_shape(command: &EvolutionBenchmarkCommand) -> MacacaResult<()> {
    if command.benchmark_id.trim().is_empty() {
        return Err(MacacaError::Config(
            "evolution benchmark requires benchmark_id".into(),
        ));
    }
    if command.run_id.trim().is_empty() {
        return Err(MacacaError::Config(
            "evolution benchmark requires run_id".into(),
        ));
    }
    if command.actor_id.trim().is_empty() {
        return Err(MacacaError::Config(
            "evolution benchmark requires actor_id".into(),
        ));
    }
    if command.trace.trace_id.trim().is_empty() {
        return Err(MacacaError::Config(
            "evolution benchmark requires trace_id".into(),
        ));
    }
    if command.policy_decision_refs.is_empty() {
        return Err(MacacaError::Config(
            "evolution benchmark requires policy decision refs".into(),
        ));
    }
    Ok(())
}

fn collect_comparability_reasons(command: &EvolutionBenchmarkCommand, reasons: &mut Vec<String>) {
    if command.baseline.task_family_id != command.candidate.task_family_id {
        reasons.push("task_family_mismatch".into());
    }
    if command.baseline.evidence_refs.is_empty() {
        reasons.push("baseline_evidence_missing".into());
    }
    if command.candidate.evidence_refs.is_empty() {
        reasons.push("candidate_evidence_missing".into());
    }
    if command.baseline.metrics.is_none() {
        reasons.push("baseline_metrics_missing".into());
    }
    if command.candidate.metrics.is_none() {
        reasons.push("candidate_metrics_missing".into());
    }
}

fn score_delta(
    baseline: &EvolutionBenchmarkMetrics,
    candidate: &EvolutionBenchmarkMetrics,
) -> EvolutionBenchmarkScoreDelta {
    let total_token_delta = candidate.total_tokens as i64 - baseline.total_tokens as i64;
    let elapsed_ms_delta = candidate.elapsed_ms as i64 - baseline.elapsed_ms as i64;
    let tool_call_delta = candidate.tool_calls as i64 - baseline.tool_calls as i64;
    let retry_delta = candidate.retry_count as i64 - baseline.retry_count as i64;
    let quality_delta = candidate.quality_score as i64 - baseline.quality_score as i64;
    let human_intervention_delta =
        candidate.human_intervention_count as i64 - baseline.human_intervention_count as i64;
    let success_count_delta = candidate.success_count as i64 - baseline.success_count as i64;
    let quality_preserved = quality_delta >= -QUALITY_TOLERANCE && success_count_delta >= 0;
    let efficiency_improved = total_token_delta < 0
        || elapsed_ms_delta < 0
        || tool_call_delta < 0
        || retry_delta < 0
        || human_intervention_delta < 0
        || success_count_delta > 0;

    EvolutionBenchmarkScoreDelta {
        total_token_delta,
        elapsed_ms_delta,
        tool_call_delta,
        retry_delta,
        quality_delta,
        human_intervention_delta,
        success_count_delta,
        efficiency_improved,
        quality_preserved,
    }
}

fn result(
    command: &EvolutionBenchmarkCommand,
    decision: BenchmarkDecision,
    score_delta: EvolutionBenchmarkScoreDelta,
    reason_codes: Vec<String>,
) -> EvolutionBenchmarkResult {
    info!(
        service_id = AUTONOMY_EVOLUTION_SERVICE_ID,
        benchmark_id = command.benchmark_id.as_str(),
        run_id = command.run_id.as_str(),
        decision = ?decision,
        "normalized evolution benchmark scoring completed"
    );
    EvolutionBenchmarkResult {
        benchmark_id: command.benchmark_id.clone(),
        run_id: command.run_id.clone(),
        task_family_id: command.baseline.task_family_id.clone(),
        target_type: command.candidate.target_type.clone(),
        decision,
        trace: command.trace.clone(),
        score_delta,
        reason_codes: reason_codes.into_iter().take(MAX_REASON_CODES).collect(),
        evidence_refs: bounded_refs(
            &command.baseline.evidence_refs,
            &command.candidate.evidence_refs,
        ),
        artifact_refs: bounded_refs(
            &command.baseline.artifact_refs,
            &command.candidate.artifact_refs,
        ),
        policy_decision_refs: command.policy_decision_refs.clone(),
        captured_at: chrono::Utc::now(),
    }
}

fn bounded_refs(left: &[String], right: &[String]) -> Vec<String> {
    left.iter()
        .chain(right.iter())
        .take(8)
        .map(|value| {
            if value.len() > 160 {
                let mut bounded = value.clone();
                bounded.truncate(160);
                bounded
            } else {
                value.clone()
            }
        })
        .collect()
}

fn bounded_regression_codes(codes: &[String]) -> Vec<String> {
    codes
        .iter()
        .take(4)
        .map(|code| {
            if code.len() > 80 {
                let mut bounded = code.clone();
                bounded.truncate(80);
                bounded
            } else {
                code.clone()
            }
        })
        .collect()
}
