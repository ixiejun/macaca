use crate::{
    SkillOwnershipOperation, SkillOwnershipPolicy, SkillOwnershipPolicyContext,
    SkillOwnershipPolicyDecision, SkillPackageOwnershipClass,
};

#[test]
fn ownership_policy_requires_marketplace_overlay_instead_of_upstream_mutation() {
    let policy = SkillOwnershipPolicy::default();
    let context = SkillOwnershipPolicyContext::generic_agent_flow(
        SkillPackageOwnershipClass::Marketplace,
        SkillOwnershipOperation::PatchActiveContent,
    );

    let decision = policy.evaluate(&context);

    assert_eq!(decision, SkillOwnershipPolicyDecision::RequiresLocalOverlay);
}

#[test]
fn ownership_policy_requires_application_scope_for_application_owned_skill_changes() {
    let policy = SkillOwnershipPolicy::default();
    let context = SkillOwnershipPolicyContext::generic_agent_flow(
        SkillPackageOwnershipClass::ApplicationOwned,
        SkillOwnershipOperation::Archive,
    );

    let decision = policy.evaluate(&context);

    assert_eq!(
        decision,
        SkillOwnershipPolicyDecision::RequiresApplicationScope
    );
}

#[test]
fn ownership_policy_restricts_paid_and_encrypted_without_mutation_entitlement() {
    let policy = SkillOwnershipPolicy::default();

    for ownership in [
        SkillPackageOwnershipClass::Paid,
        SkillPackageOwnershipClass::Encrypted,
    ] {
        let mutation = SkillOwnershipPolicyContext::generic_agent_flow(
            ownership.clone(),
            SkillOwnershipOperation::PatchActiveContent,
        );
        let alias = SkillOwnershipPolicyContext::generic_agent_flow(
            ownership,
            SkillOwnershipOperation::Alias,
        );

        assert_eq!(
            policy.evaluate(&mutation),
            SkillOwnershipPolicyDecision::RequiresMutationEntitlement
        );
        assert_eq!(
            policy.evaluate(&alias),
            SkillOwnershipPolicyDecision::Allowed
        );
    }
}

#[test]
fn ownership_policy_denies_generic_auto_mutation_for_protected_classes() {
    let policy = SkillOwnershipPolicy::default();

    for ownership in [
        SkillPackageOwnershipClass::Bundled,
        SkillPackageOwnershipClass::Marketplace,
        SkillPackageOwnershipClass::ApplicationOwned,
        SkillPackageOwnershipClass::Paid,
        SkillPackageOwnershipClass::Encrypted,
        SkillPackageOwnershipClass::PluginProvided,
    ] {
        let context = SkillOwnershipPolicyContext::generic_agent_flow(
            ownership,
            SkillOwnershipOperation::Merge,
        );

        assert_ne!(
            policy.evaluate(&context),
            SkillOwnershipPolicyDecision::Allowed
        );
    }
}
