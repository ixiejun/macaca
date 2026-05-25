use macaca_autonomy_evolution::{
    BenchmarkDecision, EvolutionBenchmarkCommand, EvolutionBenchmarkMeasurement,
    EvolutionBenchmarkMetrics, EvolutionScope, EvolutionTargetType,
    AUTONOMY_EVOLUTION_BENCHMARK_COMMAND,
};
use macaca_kernel::SystemService;
use macaca_proto::{ServiceCommand, ServiceCommandName, TraceContext};
use macaca_runtime_host::AutonomyEvolutionSystemServiceProvider;

fn measurement(label: &str, total_tokens: u64) -> EvolutionBenchmarkMeasurement {
    EvolutionBenchmarkMeasurement {
        measurement_id: format!("measurement-{label}"),
        task_family_id: "task-family.generic.runtime".into(),
        target_type: EvolutionTargetType::SkillPackage,
        metrics: Some(EvolutionBenchmarkMetrics {
            input_tokens: total_tokens / 2,
            output_tokens: total_tokens / 2,
            total_tokens,
            elapsed_ms: total_tokens,
            tool_calls: 4,
            tool_results: 4,
            retry_count: 1,
            failure_recovery_count: 0,
            quality_score: 90,
            human_intervention_count: 0,
            policy_decision_count: 1,
            activation_count: 1,
            use_count: 1,
            success_count: 1,
        }),
        evidence_refs: vec![format!("eventlog://runtime/benchmark/{label}")],
        artifact_refs: vec![format!("artifact://runtime/benchmark/{label}")],
        regression_reason_codes: Vec::new(),
    }
}

#[tokio::test]
async fn runtime_host_autonomy_evolution_provider_decodes_benchmark_command() {
    let provider = AutonomyEvolutionSystemServiceProvider::in_memory();
    let trace = TraceContext::new("trace-runtime-benchmark");
    let command = EvolutionBenchmarkCommand {
        benchmark_id: "benchmark-runtime".into(),
        run_id: "run-runtime".into(),
        actor_id: "agent.benchmark".into(),
        trace: trace.clone(),
        scope: EvolutionScope::default(),
        baseline: measurement("baseline", 1_000),
        candidate: measurement("candidate", 800),
        policy_decision_refs: vec!["policy://runtime/benchmark".into()],
    };

    let result = provider
        .call(ServiceCommand {
            name: ServiceCommandName::new(AUTONOMY_EVOLUTION_BENCHMARK_COMMAND),
            payload: serde_json::to_value(command).unwrap(),
            trace: Some(trace),
            metadata: Default::default(),
        })
        .await
        .unwrap();
    let typed: macaca_autonomy_evolution::EvolutionBenchmarkResult =
        serde_json::from_value(result.output).unwrap();

    assert_eq!(typed.decision, BenchmarkDecision::Passed);
    assert_eq!(typed.benchmark_id, "benchmark-runtime");
}
