use macaca_autonomy_evolution::{
    AdmissionDecision, EvolutionAdmissionCandidate, EvolutionAdmissionCommand, EvolutionScope,
    EvolutionTargetType, AUTONOMY_EVOLUTION_ADMISSION_COMMAND,
};
use macaca_kernel::SystemService;
use macaca_proto::{ServiceCommand, ServiceCommandName, TraceContext};
use macaca_runtime_host::AutonomyEvolutionSystemServiceProvider;

#[tokio::test]
async fn runtime_host_autonomy_evolution_provider_decodes_admission_command() {
    let provider = AutonomyEvolutionSystemServiceProvider::in_memory();
    let trace = TraceContext::new("trace-runtime-admission");
    let command = EvolutionAdmissionCommand {
        actor_id: "agent.admission".into(),
        trace: trace.clone(),
        scope: EvolutionScope::default(),
        candidate: EvolutionAdmissionCandidate {
            candidate_id: "candidate-runtime-admission".into(),
            target_type: EvolutionTargetType::SkillPackage,
            package_name: "reusable-task-evidence-summarizer".into(),
            trigger_descriptions: vec![
                "Use when repeated task evidence shows reusable summary value.".into(),
            ],
            skill_summary: "A focused reusable skill candidate with bounded metadata.".into(),
            resource_categories: vec!["SKILL.md".into(), "references".into()],
            quick_validation_refs: vec!["validation://runtime/admission".into()],
            forward_test_refs: vec!["forward-test://runtime/admission".into()],
            duplicate_candidate_refs: Vec::new(),
            metadata_stale: false,
        },
        evidence_refs: vec!["eventlog://runtime/admission".into()],
        policy_decision_refs: vec!["policy://runtime/admission".into()],
    };

    let result = provider
        .call(ServiceCommand {
            name: ServiceCommandName::new(AUTONOMY_EVOLUTION_ADMISSION_COMMAND),
            payload: serde_json::to_value(command).unwrap(),
            trace: Some(trace),
            metadata: Default::default(),
        })
        .await
        .unwrap();
    let typed: macaca_autonomy_evolution::EvolutionAdmissionResult =
        serde_json::from_value(result.output).unwrap();

    assert_eq!(typed.decision, AdmissionDecision::Accepted);
    assert_eq!(typed.candidate_id, "candidate-runtime-admission");
}
