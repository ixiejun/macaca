use crate::{
    SkillMergeEligibilityFacts, SkillMergeEligibilityPolicy, SkillMergeExecutableSemantics,
};

#[test]
fn merge_eligibility_allows_compatible_skill_facts() {
    let policy = SkillMergeEligibilityPolicy::default();
    let facts = vec![
        SkillMergeEligibilityFacts::new("skill://agent/source", "agent", "agent-private"),
        SkillMergeEligibilityFacts::new("skill://agent/target", "agent", "agent-private"),
    ];

    let decision = policy.evaluate(&facts);

    assert!(decision.allowed);
    assert!(decision.issues.is_empty());
}

#[test]
fn merge_eligibility_rejects_boundary_and_executable_mismatches() {
    let policy = SkillMergeEligibilityPolicy::default();
    let mut source =
        SkillMergeEligibilityFacts::new("skill://agent/source", "agent", "agent-private");
    source.tenant_id = Some("tenant-a".into());
    source.executable_semantics = SkillMergeExecutableSemantics::KnowledgeOnly;

    let mut target =
        SkillMergeEligibilityFacts::new("skill://app/target", "application", "marketplace");
    target.tenant_id = Some("tenant-b".into());
    target.executable_semantics = SkillMergeExecutableSemantics::Executable;

    let decision = policy.evaluate(&[source, target]);

    assert!(!decision.allowed);
    assert!(decision
        .issues
        .iter()
        .any(|issue| issue.field == "source_scope"));
    assert!(decision
        .issues
        .iter()
        .any(|issue| issue.field == "package_source"));
    assert!(decision
        .issues
        .iter()
        .any(|issue| issue.field == "tenant_id"));
    assert!(decision
        .issues
        .iter()
        .any(|issue| issue.field == "executable_semantics"));
}
