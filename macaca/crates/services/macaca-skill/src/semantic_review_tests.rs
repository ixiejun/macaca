use macaca_proto::TraceContext;

use crate::{
    SkillSemanticDuplicateInput, SkillSemanticEffectivenessInput, SkillSemanticReferenceGraphInput,
    SkillSemanticReviewProposal, SkillSemanticReviewProposalKind, SkillSemanticReviewRequest,
    SkillSemanticReviewResult, SkillSemanticReviewStatus, SkillSemanticSimilarityInput,
    SkillSemanticSkillSetInput, SkillServiceScope,
};

#[test]
fn semantic_review_request_carries_optional_analysis_inputs() {
    let request = SkillSemanticReviewRequest {
        trace: TraceContext::new("trace-semantic-review-inputs"),
        scope: SkillServiceScope::default(),
        dry_run: true,
        recommendations: Vec::new(),
        similarity_inputs: vec![SkillSemanticSimilarityInput {
            skill_id: "skill://agent/source".into(),
            comparable_skill_id: "skill://agent/target".into(),
            score_hint: Some(0.91),
            evidence_ids: vec!["evidence://similarity".into()],
        }],
        clustering_inputs: vec![SkillSemanticSkillSetInput {
            set_id: "cluster://governance/1".into(),
            skill_ids: vec!["skill://agent/source".into(), "skill://agent/target".into()],
            evidence_ids: vec!["evidence://cluster".into()],
        }],
        duplicate_inputs: vec![SkillSemanticDuplicateInput {
            source_skill_id: "skill://agent/source".into(),
            candidate_skill_id: "skill://agent/target".into(),
            evidence_ids: vec!["evidence://duplicate".into()],
        }],
        effectiveness_inputs: vec![SkillSemanticEffectivenessInput {
            skill_id: "skill://agent/source".into(),
            successful_tasks: 4,
            failed_tasks: 1,
            evidence_ids: vec!["evidence://effectiveness".into()],
        }],
        reference_graph_inputs: vec![SkillSemanticReferenceGraphInput {
            skill_id: "skill://agent/source".into(),
            referenced_skill_ids: vec!["skill://agent/target".into()],
            support_file_refs: vec!["references/example.md".into()],
            evidence_ids: vec!["evidence://reference-graph".into()],
        }],
        captured_at: chrono::Utc::now(),
    };

    assert_eq!(request.similarity_inputs.len(), 1);
    assert_eq!(request.clustering_inputs[0].skill_ids.len(), 2);
    assert_eq!(
        request.duplicate_inputs[0].source_skill_id,
        "skill://agent/source"
    );
    assert_eq!(request.effectiveness_inputs[0].successful_tasks, 4);
    assert_eq!(request.reference_graph_inputs[0].support_file_refs.len(), 1);
}

#[test]
fn semantic_review_result_rejects_direct_mutation_and_invalid_proposals() {
    let captured_at = chrono::Utc::now();
    let direct_mutation = SkillSemanticReviewResult {
        provider_id: "test-semantic-review".into(),
        status: SkillSemanticReviewStatus::Completed,
        proposals: Vec::new(),
        diagnostics: Vec::new(),
        mutated: true,
        captured_at,
    };
    assert!(direct_mutation.validate_provider_output().is_err());

    let invalid_proposal = SkillSemanticReviewResult {
        provider_id: "test-semantic-review".into(),
        status: SkillSemanticReviewStatus::Completed,
        proposals: vec![SkillSemanticReviewProposal {
            proposal_id: String::new(),
            kind: SkillSemanticReviewProposalKind::DuplicateCandidate,
            source_skill_ids: Vec::new(),
            target_skill_id: None,
            rationale: String::new(),
            confidence: 1.5,
            evidence_ids: Vec::new(),
        }],
        diagnostics: Vec::new(),
        mutated: false,
        captured_at,
    };
    assert!(invalid_proposal.validate_provider_output().is_err());
}
