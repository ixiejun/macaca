use macaca_proto::domain_pack_contract::{
    workflow_approval::{WORKFLOW_APPROVAL_PACK_ID, WORKFLOW_APPROVAL_SERVICE_ID},
    workflow_delegation::{WORKFLOW_DELEGATION_PACK_ID, WORKFLOW_DELEGATION_SERVICE_ID},
    workflow_recovery::{WORKFLOW_RECOVERY_PACK_ID, WORKFLOW_RECOVERY_SERVICE_ID},
    workflow_review::{WORKFLOW_REVIEW_PACK_ID, WORKFLOW_REVIEW_SERVICE_ID},
    workflow_schedule::{WORKFLOW_SCHEDULE_PACK_ID, WORKFLOW_SCHEDULE_SERVICE_ID},
    workflow_task::{WORKFLOW_TASK_PACK_ID, WORKFLOW_TASK_SERVICE_ID},
};
use macaca_proto::{
    compose_installed_domain_pack_catalog, reference_domain_pack_definitions,
    AppServiceContractConfig,
};

use super::*;

// Workflow SDK tests validate catalog discovery and explicit unavailable states only.
// The SDK must not construct task engines, schedulers, approval surfaces, delegation
// routers, reviewers, recovery engines, plugin adapters, remote providers, mocks, or
// application-specific workflows; it only reports provider-neutral descriptors.

#[tokio::test]
async fn catalog_client_discovers_workflow_contract_metadata() {
    let catalog = compose_installed_domain_pack_catalog(reference_domain_pack_definitions());
    let client = CatalogBackedDomainPackClient::new(catalog);

    let cases = [
        (
            WORKFLOW_TASK_PACK_ID,
            WORKFLOW_TASK_SERVICE_ID,
            "workflow_task.claim",
            "workflow_task_provider_not_installed",
            "durable-task-engine",
        ),
        (
            WORKFLOW_SCHEDULE_PACK_ID,
            WORKFLOW_SCHEDULE_SERVICE_ID,
            "workflow_schedule.backfill",
            "workflow_schedule_provider_not_installed",
            "durable-scheduler",
        ),
        (
            WORKFLOW_APPROVAL_PACK_ID,
            WORKFLOW_APPROVAL_SERVICE_ID,
            "approval.record_decision",
            "workflow_approval_provider_not_installed",
            "durable-approval",
        ),
        (
            WORKFLOW_DELEGATION_PACK_ID,
            WORKFLOW_DELEGATION_SERVICE_ID,
            "delegation.accept_delegation",
            "workflow_delegation_provider_not_installed",
            "durable-delegation",
        ),
        (
            WORKFLOW_REVIEW_PACK_ID,
            WORKFLOW_REVIEW_SERVICE_ID,
            "review.evaluate_gate",
            "workflow_review_provider_not_installed",
            "durable-review",
        ),
        (
            WORKFLOW_RECOVERY_PACK_ID,
            WORKFLOW_RECOVERY_SERVICE_ID,
            "recovery.export_replay",
            "workflow_recovery_provider_not_installed",
            "durable-recovery",
        ),
    ];

    for (pack_id, service_id, command, unavailable_reason, provider_class) in cases {
        let inspect = client
            .inspect_pack(&DomainPackInspectCommand::new(pack_id).expect("valid workflow id"))
            .await
            .unwrap();
        let pack = inspect.pack.expect("workflow descriptor exists");

        assert!(!pack.is_callable());
        assert_eq!(
            pack.metadata.diagnostics.unavailable_reason,
            unavailable_reason
        );
        assert!(pack
            .metadata
            .service_command_schemas
            .get(service_id)
            .is_some_and(|commands| commands.contains(command)));
        assert!(pack
            .metadata
            .provider_descriptors
            .contains_key(provider_class));
        assert!(pack
            .metadata
            .sdk
            .docs_url
            .contains("developer-packs/workflow"));
    }
}

#[tokio::test]
async fn catalog_client_reports_workflow_unavailable_reasons() {
    let catalog = compose_installed_domain_pack_catalog(reference_domain_pack_definitions());
    let client = CatalogBackedDomainPackClient::new(catalog);
    let command = DomainPackResolveCommand {
        declaration: AppServiceContractConfig {
            optional_packs: vec![
                WORKFLOW_TASK_PACK_ID.into(),
                WORKFLOW_SCHEDULE_PACK_ID.into(),
                WORKFLOW_APPROVAL_PACK_ID.into(),
                WORKFLOW_DELEGATION_PACK_ID.into(),
                WORKFLOW_REVIEW_PACK_ID.into(),
                WORKFLOW_RECOVERY_PACK_ID.into(),
            ],
            ..Default::default()
        },
    };

    let result = client.resolve_declaration(&command).await.unwrap();

    for (pack_id, reason) in [
        (
            WORKFLOW_TASK_PACK_ID,
            "workflow_task_provider_not_installed",
        ),
        (
            WORKFLOW_SCHEDULE_PACK_ID,
            "workflow_schedule_provider_not_installed",
        ),
        (
            WORKFLOW_APPROVAL_PACK_ID,
            "workflow_approval_provider_not_installed",
        ),
        (
            WORKFLOW_DELEGATION_PACK_ID,
            "workflow_delegation_provider_not_installed",
        ),
        (
            WORKFLOW_REVIEW_PACK_ID,
            "workflow_review_provider_not_installed",
        ),
        (
            WORKFLOW_RECOVERY_PACK_ID,
            "workflow_recovery_provider_not_installed",
        ),
    ] {
        assert!(result
            .effective
            .unresolved_optional_packs
            .contains(&pack_id.to_string()));
        assert_eq!(
            result.effective.unavailable_pack_reasons.get(pack_id),
            Some(&reason.to_string())
        );
    }
}
