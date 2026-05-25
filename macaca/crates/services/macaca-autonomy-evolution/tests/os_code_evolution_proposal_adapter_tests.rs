use macaca_autonomy_evolution::{
    DefaultOsCodeEvolutionProposalAdapter, EvolutionScope, OsCodeEvolutionProposalAdapter,
    OsCodeEvolutionProposalCommand, OsCodeEvolutionProposalDecision,
    OsCodeEvolutionProposalEvidence, OsCodeEvolutionProposalIntent,
};
use macaca_proto::TraceContext;

fn scope() -> EvolutionScope {
    EvolutionScope {
        application_id: None,
        tenant_id: Some("tenant.generic".into()),
        session_id: Some("session.generic".into()),
        task_id: Some("task.generic".into()),
    }
}

fn evidence() -> OsCodeEvolutionProposalEvidence {
    OsCodeEvolutionProposalEvidence {
        openspec_proposal_refs: vec!["openspec://changes/generic/proposal.md".into()],
        openspec_design_refs: vec!["openspec://changes/generic/design.md".into()],
        openspec_tasks_refs: vec!["openspec://changes/generic/tasks.md".into()],
        superpowers_design_refs: vec!["superpowers://specs/generic-design".into()],
        superpowers_plan_refs: vec!["superpowers://plans/generic-plan".into()],
        gitnexus_impact_refs: vec!["gitnexus://impact/generic".into()],
        expected_test_refs: vec!["cargo-test://macaca-autonomy-evolution".into()],
        release_gate_refs: vec!["release://gate/generic".into()],
        rollback_refs: vec!["memento://os-code/generic".into()],
    }
}

fn command() -> OsCodeEvolutionProposalCommand {
    OsCodeEvolutionProposalCommand {
        proposal_id: "os-code-proposal-generic".into(),
        run_id: "run-os-code-generic".into(),
        actor_id: "agent.evolution".into(),
        trace: TraceContext::new("trace-os-code-proposal"),
        scope: scope(),
        intent: OsCodeEvolutionProposalIntent {
            title: "Refine generic autonomy service boundary".into(),
            summary: "Create a governed proposal for a generic OS service improvement.".into(),
            affected_capability_refs: vec!["capability://autonomy/evolution".into()],
            requested_change_refs: vec!["change://generic/os-code-proposal".into()],
            allow_source_mutation: false,
            blast_radius_score: 30,
        },
        evidence: evidence(),
        policy_decision_refs: vec!["policy://os-code/proposal".into()],
        audit_refs: vec!["audit://os-code/proposal".into()],
    }
}

#[test]
fn ready_os_code_proposal_is_non_mutating_and_reviewable() {
    let adapter = DefaultOsCodeEvolutionProposalAdapter;

    let result = adapter.evaluate(command()).unwrap();

    assert_eq!(
        result.decision,
        OsCodeEvolutionProposalDecision::ReadyForReview
    );
    assert!(!result.source_mutation_performed);
    assert_eq!(
        result.bundle.proposal_ref.as_deref(),
        Some("os-code-proposal-generic")
    );
    assert!(result
        .reason_codes
        .iter()
        .any(|reason| reason == "proposal_ready_for_review"));
}

#[test]
fn missing_gitnexus_impact_evidence_needs_evidence() {
    let adapter = DefaultOsCodeEvolutionProposalAdapter;
    let mut command = command();
    command.evidence.gitnexus_impact_refs.clear();

    let result = adapter.evaluate(command).unwrap();

    assert_eq!(
        result.decision,
        OsCodeEvolutionProposalDecision::NeedsEvidence
    );
    assert!(result
        .missing_evidence
        .iter()
        .any(|item| item == "gitnexus_impact_refs"));
    assert!(!result.source_mutation_performed);
}

#[test]
fn high_blast_radius_is_quarantined() {
    let adapter = DefaultOsCodeEvolutionProposalAdapter;
    let mut command = command();
    command.intent.blast_radius_score = 95;

    let result = adapter.evaluate(command).unwrap();

    assert_eq!(
        result.decision,
        OsCodeEvolutionProposalDecision::Quarantined
    );
    assert!(result
        .reason_codes
        .iter()
        .any(|reason| reason == "blast_radius_requires_quarantine"));
    assert!(!result.source_mutation_performed);
}

#[test]
fn source_mutation_request_is_denied() {
    let adapter = DefaultOsCodeEvolutionProposalAdapter;
    let mut command = command();
    command.intent.allow_source_mutation = true;

    let result = adapter.evaluate(command).unwrap();

    assert_eq!(result.decision, OsCodeEvolutionProposalDecision::Denied);
    assert!(result
        .reason_codes
        .iter()
        .any(|reason| reason == "source_mutation_not_allowed"));
    assert!(!result.source_mutation_performed);
}
