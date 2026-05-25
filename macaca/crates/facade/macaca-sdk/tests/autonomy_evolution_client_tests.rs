use macaca_autonomy_evolution::{
    EvolutionRunTransition, EvolutionScope, EvolutionTargetType, EvolutionTransitionCommand,
};
use macaca_proto::TraceContext;
use macaca_sdk::{SystemAutonomyEvolutionClient, UnavailableSystemAutonomyEvolutionClient};

#[tokio::test]
async fn unavailable_autonomy_evolution_client_fails_closed() {
    let client = UnavailableSystemAutonomyEvolutionClient;
    let result = client
        .transition(EvolutionTransitionCommand {
            run_id: "run-sdk-unavailable".into(),
            from_state: None,
            transition: EvolutionRunTransition::QueueCandidate,
            target_type: EvolutionTargetType::SkillPackage,
            scope: EvolutionScope::default(),
            actor_id: "agent.coordinator".into(),
            trace: TraceContext::new("trace-sdk-unavailable"),
            evidence_refs: vec!["eventlog://sessions/sdk/observer".into()],
            policy_decision_refs: Vec::new(),
            audit_refs: Vec::new(),
            rollback_refs: Vec::new(),
            diagnostics: Default::default(),
        })
        .await
        .unwrap();

    assert!(!result.accepted);
    assert!(result
        .denial_reason
        .as_deref()
        .unwrap_or_default()
        .contains("unavailable"));
}
