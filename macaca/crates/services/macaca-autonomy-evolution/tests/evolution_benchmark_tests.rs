use macaca_autonomy_evolution::{
    AutonomyEvolutionService, BenchmarkDecision, EvolutionBenchmarkCommand,
    EvolutionBenchmarkMeasurement, EvolutionBenchmarkMetrics, EvolutionScope, EvolutionTargetType,
    InMemoryAutonomyEvolutionProvider,
};
use macaca_proto::TraceContext;

fn metrics(
    total_tokens: u64,
    elapsed_ms: u64,
    tool_calls: u32,
    quality_score: u32,
) -> EvolutionBenchmarkMetrics {
    EvolutionBenchmarkMetrics {
        input_tokens: total_tokens / 2,
        output_tokens: total_tokens / 2,
        total_tokens,
        elapsed_ms,
        tool_calls,
        tool_results: tool_calls,
        retry_count: 1,
        failure_recovery_count: 0,
        quality_score,
        human_intervention_count: 0,
        policy_decision_count: 1,
        activation_count: 1,
        use_count: 1,
        success_count: 1,
    }
}

fn measurement(label: &str, metrics: EvolutionBenchmarkMetrics) -> EvolutionBenchmarkMeasurement {
    EvolutionBenchmarkMeasurement {
        measurement_id: format!("measurement-{label}"),
        task_family_id: "task-family.generic.reusable".into(),
        target_type: EvolutionTargetType::SkillPackage,
        metrics: Some(metrics),
        evidence_refs: vec![format!("eventlog://benchmark/{label}")],
        artifact_refs: vec![format!("artifact://benchmark/{label}")],
        regression_reason_codes: Vec::new(),
    }
}

fn command(
    baseline: EvolutionBenchmarkMeasurement,
    candidate: EvolutionBenchmarkMeasurement,
) -> EvolutionBenchmarkCommand {
    EvolutionBenchmarkCommand {
        benchmark_id: "benchmark-generic".into(),
        run_id: "run-generic".into(),
        actor_id: "agent.benchmark".into(),
        trace: TraceContext::new("trace-benchmark-generic"),
        scope: EvolutionScope {
            application_id: Some("app.generic".into()),
            tenant_id: Some("tenant.generic".into()),
            session_id: Some("session.generic".into()),
            task_id: Some("task.generic".into()),
        },
        baseline,
        candidate,
        policy_decision_refs: vec!["policy://benchmark/allow".into()],
    }
}

#[tokio::test]
async fn quality_preserving_efficiency_gain_passes() {
    let service = InMemoryAutonomyEvolutionProvider::default();

    let result = service
        .run_paired_benchmark(command(
            measurement("baseline", metrics(1_000, 10_000, 8, 90)),
            measurement("candidate", metrics(800, 8_000, 6, 91)),
        ))
        .await
        .unwrap();

    assert_eq!(result.decision, BenchmarkDecision::Passed);
    assert!(result.score_delta.efficiency_improved);
    assert!(result
        .reason_codes
        .iter()
        .any(|reason| reason == "quality_preserved_efficiency_improved"));
}

#[tokio::test]
async fn quality_regression_fails_even_when_efficiency_improves() {
    let service = InMemoryAutonomyEvolutionProvider::default();

    let result = service
        .run_paired_benchmark(command(
            measurement("baseline", metrics(1_000, 10_000, 8, 90)),
            measurement("candidate", metrics(700, 7_000, 5, 82)),
        ))
        .await
        .unwrap();

    assert_eq!(result.decision, BenchmarkDecision::Failed);
    assert!(result
        .reason_codes
        .iter()
        .any(|reason| reason == "quality_regressed"));
}

#[tokio::test]
async fn different_task_family_is_inconclusive() {
    let service = InMemoryAutonomyEvolutionProvider::default();
    let baseline = measurement("baseline", metrics(1_000, 10_000, 8, 90));
    let mut candidate = measurement("candidate", metrics(800, 8_000, 6, 91));
    candidate.task_family_id = "task-family.generic.other".into();

    let result = service
        .run_paired_benchmark(command(baseline, candidate))
        .await
        .unwrap();

    assert_eq!(result.decision, BenchmarkDecision::Inconclusive);
    assert!(result
        .reason_codes
        .iter()
        .any(|reason| reason == "task_family_mismatch"));
}

#[tokio::test]
async fn missing_metrics_are_inconclusive() {
    let service = InMemoryAutonomyEvolutionProvider::default();
    let baseline = measurement("baseline", metrics(1_000, 10_000, 8, 90));
    let mut candidate = measurement("candidate", metrics(800, 8_000, 6, 91));
    candidate.metrics = None;

    let result = service
        .run_paired_benchmark(command(baseline, candidate))
        .await
        .unwrap();

    assert_eq!(result.decision, BenchmarkDecision::Inconclusive);
    assert!(result
        .reason_codes
        .iter()
        .any(|reason| reason == "candidate_metrics_missing"));
}

#[tokio::test]
async fn candidate_regression_reason_fails_benchmark() {
    let service = InMemoryAutonomyEvolutionProvider::default();
    let mut candidate = measurement("candidate", metrics(800, 8_000, 6, 91));
    candidate.regression_reason_codes = vec!["artifact_quality_regression".into()];

    let result = service
        .run_paired_benchmark(command(
            measurement("baseline", metrics(1_000, 10_000, 8, 90)),
            candidate,
        ))
        .await
        .unwrap();

    assert_eq!(result.decision, BenchmarkDecision::Failed);
    assert!(result
        .reason_codes
        .iter()
        .any(|reason| reason == "candidate_regression_reported"));
}
