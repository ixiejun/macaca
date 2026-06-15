use macaca_proto::TraceContext;
use macaca_proto::{
    BenchmarkDecision, EvolutionBenchmarkCommand, EvolutionBenchmarkMeasurement,
    EvolutionBenchmarkMetrics, EvolutionScope, EvolutionTargetType,
};
use macaca_sdk::{SystemAutonomyEvolutionClient, UnavailableSystemAutonomyEvolutionClient};

fn measurement(label: &str) -> EvolutionBenchmarkMeasurement {
    EvolutionBenchmarkMeasurement {
        measurement_id: format!("measurement-{label}"),
        task_family_id: "task-family.generic.sdk".into(),
        target_type: EvolutionTargetType::SkillPackage,
        metrics: Some(EvolutionBenchmarkMetrics {
            input_tokens: 100,
            output_tokens: 100,
            total_tokens: 200,
            elapsed_ms: 1_000,
            tool_calls: 2,
            tool_results: 2,
            retry_count: 0,
            failure_recovery_count: 0,
            quality_score: 90,
            human_intervention_count: 0,
            policy_decision_count: 1,
            activation_count: 1,
            use_count: 1,
            success_count: 1,
        }),
        evidence_refs: vec![format!("eventlog://sdk/benchmark/{label}")],
        artifact_refs: vec![format!("artifact://sdk/benchmark/{label}")],
        regression_reason_codes: Vec::new(),
    }
}

#[tokio::test]
async fn unavailable_autonomy_evolution_client_returns_inconclusive_benchmark() {
    let client = UnavailableSystemAutonomyEvolutionClient;
    let result = client
        .run_paired_benchmark(EvolutionBenchmarkCommand {
            benchmark_id: "benchmark-sdk-unavailable".into(),
            run_id: "run-sdk-unavailable".into(),
            actor_id: "agent.benchmark".into(),
            trace: TraceContext::new("trace-sdk-benchmark-unavailable"),
            scope: EvolutionScope::default(),
            baseline: measurement("baseline"),
            candidate: measurement("candidate"),
            policy_decision_refs: vec!["policy://sdk/benchmark".into()],
        })
        .await
        .unwrap();

    assert_eq!(result.decision, BenchmarkDecision::Inconclusive);
    assert!(result
        .reason_codes
        .iter()
        .any(|reason| reason.contains("unavailable")));
}
