//! Optional-provider boundary gates for self-evolving Skill OS.
//!
//! These tests pin the fail-closed contracts required by the governance
//! OpenSpec change.  Optional Store, Entitlement, marketplace, semantic review,
//! and governance-store providers may be absent, but absence must surface as
//! structured unavailable/denied data instead of a panic, silent fallback, or
//! fake success.

use macaca_kernel::SystemService;
use macaca_proto::{
    CommerceMetadata, DeveloperId, LicenseType, PackageId, ServiceCommand, ServiceCommandName,
    ServiceError, StorePackageInstallCommand, StorePackageScope, TraceContext,
    STORE_PACKAGE_INSTALL_COMMAND,
};
use macaca_skill::{
    SkillGovernanceStoreStrategy, SkillOwnershipOperation, SkillOwnershipPolicy,
    SkillOwnershipPolicyContext, SkillOwnershipPolicyDecision, SkillPackageOwnershipClass,
    SkillSemanticReviewRequest, SkillSemanticReviewStatus, SkillServiceScope,
    UnavailableSkillGovernanceStore,
};

use crate::{
    skill_service_provider_semantic_review::RuntimeSkillSemanticReviewProviderFactory,
    StoreSystemServiceProvider,
};

fn traced_command<T: serde::Serialize>(
    name: &str,
    payload: T,
    trace: TraceContext,
) -> ServiceCommand {
    ServiceCommand::with_trace(
        ServiceCommandName::new(name),
        serde_json::to_value(payload).expect("test payload should serialize"),
        trace,
    )
}

#[tokio::test]
async fn absent_semantic_review_provider_returns_structured_unavailable() {
    let provider = RuntimeSkillSemanticReviewProviderFactory::default().build();
    let trace = TraceContext::new("trace-skill-optional-semantic-unavailable");

    let result = provider
        .review(SkillSemanticReviewRequest {
            trace,
            scope: SkillServiceScope::default(),
            dry_run: true,
            recommendations: Vec::new(),
            similarity_inputs: Vec::new(),
            clustering_inputs: Vec::new(),
            duplicate_inputs: Vec::new(),
            effectiveness_inputs: Vec::new(),
            reference_graph_inputs: Vec::new(),
            budget: Default::default(),
            resource_limits: Default::default(),
            sanitization_policy: Default::default(),
            captured_at: chrono::Utc::now(),
        })
        .await;

    assert_eq!(result.status, SkillSemanticReviewStatus::Unavailable);
    assert_eq!(result.provider_id, "unavailable-skill-semantic-review");
    assert!(!result.mutated);
    assert!(result.proposals.is_empty());
    assert!(result.status_message().contains("unavailable"));
}

#[tokio::test]
async fn absent_governance_store_never_reports_fake_append_or_replay_success() {
    let store = UnavailableSkillGovernanceStore::new("store event log is not configured");

    let replay = store
        .read_model()
        .await
        .expect_err("absent Store/EventLog provider must fail closed");

    assert_eq!(replay.reason, "store event log is not configured");
    assert!(!replay.retryable_without_reconfiguration);
}

#[tokio::test]
async fn paid_store_install_without_entitlement_provider_is_structured_unavailable() {
    let provider = StoreSystemServiceProvider::metadata_only();
    let trace = TraceContext::new("trace-skill-optional-store-entitlement");
    let commerce = CommerceMetadata {
        license_type: LicenseType::paid(),
        store_required: true,
        ..CommerceMetadata::default()
    };
    let command = StorePackageInstallCommand {
        trace: trace.clone(),
        scope: StorePackageScope {
            package_id: Some(PackageId::new("skill.optional.paid")),
            developer_id: Some(DeveloperId::new("developer.optional")),
            package_ref: None,
            source_id: Some("store://optional-provider-boundary".into()),
        },
        commerce,
        runtime_kind: None,
    };

    let error = provider
        .call(traced_command(STORE_PACKAGE_INSTALL_COMMAND, command, trace))
        .await
        .expect_err("paid install must not fake success without Entitlement service");

    assert!(matches!(error, ServiceError::ServiceUnavailable(_)));
    assert!(error.to_string().contains("Entitlement service"));
}

#[test]
fn marketplace_paid_and_encrypted_skill_mutations_return_policy_denials() {
    let policy = SkillOwnershipPolicy;

    let marketplace = policy.evaluate(&SkillOwnershipPolicyContext::generic_agent_flow(
        SkillPackageOwnershipClass::Marketplace,
        SkillOwnershipOperation::PatchActiveContent,
    ));
    assert_eq!(
        marketplace,
        SkillOwnershipPolicyDecision::RequiresLocalOverlay
    );
    assert!(marketplace
        .denied_reason()
        .expect("marketplace mutation should carry a bounded denial reason")
        .contains("local overlay"));

    for ownership in [
        SkillPackageOwnershipClass::Paid,
        SkillPackageOwnershipClass::Encrypted,
    ] {
        let decision = policy.evaluate(&SkillOwnershipPolicyContext::generic_agent_flow(
            ownership,
            SkillOwnershipOperation::PatchActiveContent,
        ));
        assert_eq!(
            decision,
            SkillOwnershipPolicyDecision::RequiresMutationEntitlement
        );
        assert!(decision
            .denied_reason()
            .expect("entitlement denial should be operator-visible")
            .contains("mutation entitlement"));
    }
}
