use macaca_proto::domain_pack_contract::finance_accounting::{
    finance_accounting_pack_definition, AccountingCommandPreflight, AccountingResultStatus,
    FINANCE_ACCOUNTING_PACK_ID, FINANCE_ACCOUNTING_SERVICE_ID,
};
use macaca_proto::{
    compose_installed_domain_pack_catalog, AppServiceContractConfig, DomainPackAvailability,
    DomainPackDefinition, TraceContext,
};

use crate::domain_pack_client::{
    CatalogBackedDomainPackClient, DomainPackResolveCommand, SystemDomainPackClient,
};

use super::*;

// These tests keep accounting SDK helpers at the Facade boundary. They use a
// callable descriptor-only fixture so command construction can be verified
// without registering any concrete accounting provider.

#[tokio::test]
async fn accounting_helper_builds_canonical_traced_service_command() {
    let resolved = resolve_callable_accounting_pack().await;
    let preflight = AccountingCommandPreflight::allowed("accounting.post_journal");

    let outcome = AccountingDomainPackCommandBuilder::new(
        "accounting.post_journal",
        serde_json::json!({"plan_ref": "journal-plan"}),
        preflight,
        TraceContext::new("trace-accounting-helper"),
    )
    .unwrap()
    .build(&resolved)
    .unwrap();

    let AccountingDomainPackCommandBuildOutcome::Ready(command) = outcome else {
        panic!("accepted accounting preflight should build a service command");
    };
    assert_eq!(command.service_id, FINANCE_ACCOUNTING_SERVICE_ID);
    assert_eq!(command.command_name, "accounting.post_journal");
    assert_eq!(
        command.trace.as_ref().map(|trace| trace.trace_id.as_str()),
        Some("trace-accounting-helper")
    );
}

#[tokio::test]
async fn accounting_helper_rejects_before_service_command_build() {
    let resolved = resolve_callable_accounting_pack().await;
    let mut preflight = AccountingCommandPreflight::allowed("accounting.post_journal");
    preflight.policy.allowed = false;
    preflight.policy.reason_code = "policy_denied".into();

    let outcome = AccountingDomainPackCommandBuilder::new(
        "accounting.post_journal",
        serde_json::json!({"plan_ref": "journal-plan"}),
        preflight,
        TraceContext::new("trace-accounting-denied"),
    )
    .unwrap()
    .build(&resolved)
    .unwrap();

    let AccountingDomainPackCommandBuildOutcome::Rejected(rejection) = outcome else {
        panic!("denied accounting preflight must not build a service command");
    };
    assert_eq!(rejection.status, AccountingResultStatus::Denied);
    assert_eq!(rejection.reason_code, "policy_denied");
}

#[test]
fn accounting_helper_rejects_preflight_command_mismatch() {
    let preflight = AccountingCommandPreflight::allowed("accounting.post_journal");
    let rejected = AccountingDomainPackCommandBuilder::new(
        "accounting.account_request",
        serde_json::json!({}),
        preflight,
        TraceContext::new("trace-accounting-mismatch"),
    );

    assert!(rejected.is_err());
}

async fn resolve_callable_accounting_pack() -> crate::domain_pack_client::DomainPackResolveResult {
    let catalog = compose_installed_domain_pack_catalog([callable_accounting_definition()]);
    let client = CatalogBackedDomainPackClient::new(catalog);
    let declaration = AppServiceContractConfig {
        required_packs: vec![FINANCE_ACCOUNTING_PACK_ID.into()],
        ..Default::default()
    };

    client
        .resolve_declaration(&DomainPackResolveCommand { declaration })
        .await
        .expect("callable accounting descriptor resolves")
}

fn callable_accounting_definition() -> DomainPackDefinition {
    let mut definition = finance_accounting_pack_definition();
    definition.metadata.availability = DomainPackAvailability::Available;
    definition
}
