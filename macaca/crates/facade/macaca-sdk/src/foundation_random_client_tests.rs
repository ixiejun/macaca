//! SDK random Facade tests proving canonical dispatch and fail-closed rejection.

use macaca_proto::domain_pack_contract::foundation_random_semantics::{
    RandomAdmissionFailure, RandomResourceReservation,
};
use macaca_proto::{
    compose_installed_domain_pack_catalog, reference_domain_pack_definitions,
    AppServiceContractConfig, DomainPackAvailability, TraceContext,
};
use macaca_proto::{FOUNDATION_RANDOM_PACK_ID, FOUNDATION_RANDOM_SERVICE_ID};

use super::{random_bytes_command, RandomDomainPackCommandBuildOutcome};
use crate::domain_pack_client::{DomainPackResolveResult, SystemDomainPackClient};
use crate::{CatalogBackedDomainPackClient, DomainPackInspectCommand, DomainPackResolveCommand};

async fn resolved() -> DomainPackResolveResult {
    let mut definition = macaca_proto::foundation_random_pack_definition();
    definition.metadata.availability = DomainPackAvailability::Available;
    CatalogBackedDomainPackClient::new(compose_installed_domain_pack_catalog(vec![definition]))
        .resolve_declaration(&DomainPackResolveCommand {
            declaration: AppServiceContractConfig {
                optional_packs: vec![FOUNDATION_RANDOM_PACK_ID.into()],
                ..Default::default()
            },
        })
        .await
        .unwrap()
}

#[tokio::test]
async fn random_helper_builds_canonical_traced_service_call() {
    let command = random_bytes_command(
        serde_json::json!({"length": 16}),
        Ok(RandomResourceReservation {
            byte_units: 16,
            request_units: 1,
            ..Default::default()
        }),
        TraceContext::new("trace-sdk-random"),
    );
    let outcome = command.build(&resolved().await).unwrap();
    let RandomDomainPackCommandBuildOutcome::Ready(command) = outcome else {
        panic!("expected ready")
    };
    assert_eq!(command.service_id, FOUNDATION_RANDOM_SERVICE_ID);
    assert_eq!(command.command_name, "random.bytes");
    assert_eq!(
        command.trace.as_ref().unwrap().trace_id.as_str(),
        "trace-sdk-random"
    );
}

#[tokio::test]
async fn random_helper_rejects_without_creating_a_service_call() {
    let command = random_bytes_command(
        serde_json::json!({"length": 16}),
        Err(RandomAdmissionFailure::QuotaExceeded),
        TraceContext::new("trace-sdk-random-denied"),
    );
    let outcome = command.build(&resolved().await).unwrap();
    assert_eq!(
        outcome,
        RandomDomainPackCommandBuildOutcome::Rejected(RandomAdmissionFailure::QuotaExceeded)
    );
}

#[tokio::test]
async fn random_descriptor_remains_discoverable_when_provider_is_absent() {
    let client = CatalogBackedDomainPackClient::new(compose_installed_domain_pack_catalog(
        reference_domain_pack_definitions(),
    ));
    let result = client
        .inspect_pack(&DomainPackInspectCommand::new(FOUNDATION_RANDOM_PACK_ID).unwrap())
        .await
        .unwrap();
    assert!(!result.pack.unwrap().is_callable());
}
