use macaca_autonomy_evolution::{
    EvolutionRunTransition, EvolutionScope, EvolutionTargetType, EvolutionTransitionCommand,
    AUTONOMY_EVOLUTION_TRANSITION_COMMAND,
};
use macaca_kernel::SystemService;
use macaca_proto::{ServiceCommand, ServiceCommandName, TraceContext};
use macaca_runtime_host::AutonomyEvolutionSystemServiceProvider;

#[tokio::test]
async fn runtime_host_autonomy_evolution_provider_decodes_transition_command() {
    let provider = AutonomyEvolutionSystemServiceProvider::in_memory();
    let trace = TraceContext::new("trace-runtime-autonomy-evolution");
    let command = EvolutionTransitionCommand {
        run_id: "run-runtime-adapter".into(),
        from_state: None,
        transition: EvolutionRunTransition::QueueCandidate,
        target_type: EvolutionTargetType::SkillPackage,
        scope: EvolutionScope::default(),
        actor_id: "agent.coordinator".into(),
        trace: trace.clone(),
        evidence_refs: vec!["eventlog://sessions/runtime/observer".into()],
        policy_decision_refs: Vec::new(),
        audit_refs: Vec::new(),
        rollback_refs: Vec::new(),
        diagnostics: Default::default(),
    };

    let result = provider
        .call(ServiceCommand {
            name: ServiceCommandName::new(AUTONOMY_EVOLUTION_TRANSITION_COMMAND),
            payload: serde_json::to_value(command).unwrap(),
            trace: Some(trace),
            metadata: Default::default(),
        })
        .await
        .unwrap();
    let typed: macaca_autonomy_evolution::EvolutionTransitionResult =
        serde_json::from_value(result.output).unwrap();

    assert!(typed.accepted);
    assert_eq!(typed.run_id, "run-runtime-adapter");
}
