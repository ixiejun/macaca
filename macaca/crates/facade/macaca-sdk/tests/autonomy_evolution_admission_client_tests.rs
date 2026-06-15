use macaca_proto::TraceContext;
use macaca_proto::{
    AdmissionDecision, EvolutionAdmissionCandidate, EvolutionAdmissionCommand, EvolutionScope,
    EvolutionTargetType,
};
use macaca_sdk::{SystemAutonomyEvolutionClient, UnavailableSystemAutonomyEvolutionClient};

#[tokio::test]
async fn unavailable_autonomy_evolution_client_denies_admission() {
    let client = UnavailableSystemAutonomyEvolutionClient;
    let result = client
        .admit_candidate(EvolutionAdmissionCommand {
            actor_id: "agent.admission".into(),
            trace: TraceContext::new("trace-sdk-admission-unavailable"),
            scope: EvolutionScope::default(),
            candidate: EvolutionAdmissionCandidate {
                candidate_id: "candidate-sdk-unavailable".into(),
                target_type: EvolutionTargetType::SkillPackage,
                package_name: "reusable-task-evidence-summarizer".into(),
                trigger_descriptions: vec![
                    "Use when generic task evidence should be summarized.".into()
                ],
                skill_summary: "A bounded reusable skill candidate.".into(),
                resource_categories: vec!["SKILL.md".into()],
                quick_validation_refs: vec!["validation://sdk/admission".into()],
                forward_test_refs: vec!["forward-test://sdk/admission".into()],
                duplicate_candidate_refs: Vec::new(),
                metadata_stale: false,
            },
            evidence_refs: vec!["eventlog://sdk/admission".into()],
            policy_decision_refs: vec!["policy://sdk/admission".into()],
        })
        .await
        .unwrap();

    assert_eq!(result.decision, AdmissionDecision::Denied);
    assert!(result
        .summary_reason
        .as_deref()
        .unwrap_or_default()
        .contains("unavailable"));
}
