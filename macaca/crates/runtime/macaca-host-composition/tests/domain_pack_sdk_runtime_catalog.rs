//! Catalog-wide SDK-to-runtime conformance tests for optional domain packs.
//!
//! The test composes the real SDK Command/Facade builders with the production
//! host-composition adapter and the generic mock Strategy. It proves that every
//! descriptor-declared command has exactly one canonical dispatch path without
//! teaching the OS any pack, provider, or application-specific business behavior.

use std::sync::Arc;

use macaca_host_composition::HostRuntimeSystemServiceClient;
use macaca_proto::{
    compose_installed_domain_pack_catalog, industrial_reference_domain_pack_definitions,
    AppServiceContractConfig, DomainPackAvailability, DomainPackDefinition, KernelServiceId,
    ServiceDescriptor, ServiceType, TraceContext, TraceSchemaRef,
};
use macaca_runtime_host::{
    domain_pack_service_provider::DomainPackMockSystemServiceProvider,
    InMemoryServiceRuntimeEventSink, ServiceProviderInstance, ServiceRuntime, ServiceRuntimeConfig,
    StaticServiceProviderFactory,
};
use macaca_sdk::{
    CatalogBackedDomainPackClient, DomainPackCommandCatalogBuilder, DomainPackResolveCommand,
    SystemDomainPackClient, SystemServiceClient,
};

#[tokio::test]
async fn every_industrial_command_uses_sdk_facade_and_host_runtime_dispatch_once() {
    let mut pack_count = 0usize;
    let mut command_count = 0usize;

    for definition in industrial_reference_domain_pack_definitions() {
        pack_count += 1;
        let definition = callable(definition);
        let resolved = resolve_callable(&definition).await;
        let catalog = DomainPackCommandCatalogBuilder::from_pack_definition(&definition)
            .expect("callable descriptor should build SDK command catalog");
        let events = Arc::new(InMemoryServiceRuntimeEventSink::new());
        let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig {
            event_sink: Some(events.clone()),
            ..Default::default()
        }));

        for service_id in definition.metadata.service_command_schemas.keys() {
            let descriptor = descriptor_for(service_id);
            let provider = Arc::new(DomainPackMockSystemServiceProvider::new(
                descriptor.clone(),
                definition.pack_id.clone(),
            ));
            let registered_id = runtime
                .register_provider(
                    &StaticServiceProviderFactory::new(ServiceProviderInstance::new(
                        descriptor, provider,
                    )),
                    Default::default(),
                )
                .await
                .expect("host-owned mock provider should register");
            runtime
                .start(
                    &registered_id,
                    TraceContext::new(format!("trace-sdk-start-{service_id}")),
                )
                .await
                .expect("host-owned mock provider should start");
        }

        let client = HostRuntimeSystemServiceClient::new(runtime, "domain-pack-sdk-conformance");
        for spec in catalog.command_specs() {
            command_count += 1;
            let trace = TraceContext::new(format!(
                "trace-sdk-{}",
                spec.command_name.replace(['.', '_'], "-")
            ));
            let command = catalog
                .command_builder(
                    &spec.command_name,
                    serde_json::json!({
                        "raw_secret": "must-not-leak",
                        "provider_payload": {"private": true}
                    }),
                    trace.clone(),
                )
                .expect("descriptor-declared command should build")
                .build(&resolved)
                .expect("admitted command should build canonical service call");

            let result = client
                .call_service(&command)
                .await
                .expect("SDK command should cross host runtime adapter");
            assert_eq!(result.service_id, spec.service_id);
            assert_eq!(result.output["status"], "ok");
            assert_eq!(result.output["mock"], true);
            assert!(!result.output.to_string().contains("must-not-leak"));

            let emitted = events.events().expect("event sink should remain readable");
            assert_eq!(
                emitted
                    .iter()
                    .filter(|event| {
                        event.trace_id.as_deref() == Some(trace.trace_id.as_str())
                            && event.operation == "service_runtime.call.dispatched"
                    })
                    .count(),
                1,
                "{} must have one runtime dispatch",
                spec.command_name
            );
            assert_eq!(
                emitted
                    .iter()
                    .filter(|event| {
                        event.trace_id.as_deref() == Some(trace.trace_id.as_str())
                            && event.operation == "service_runtime.call.completed"
                    })
                    .count(),
                1,
                "{} must have one runtime completion",
                spec.command_name
            );
        }

        let observability = events
            .events()
            .expect("event sink should remain readable")
            .iter()
            .map(|event| event.payload.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(!observability.contains("must-not-leak"));
        assert!(!observability.contains("provider_payload"));
    }

    assert_eq!(pack_count, 74);
    assert!(command_count > pack_count);
}

async fn resolve_callable(
    definition: &DomainPackDefinition,
) -> macaca_sdk::DomainPackResolveResult {
    let catalog = compose_installed_domain_pack_catalog([definition.clone()]);
    let client = CatalogBackedDomainPackClient::new(catalog);
    client
        .resolve_declaration(&DomainPackResolveCommand {
            declaration: AppServiceContractConfig {
                required_packs: vec![definition.pack_id.clone()],
                ..Default::default()
            },
        })
        .await
        .expect("callable pack descriptor should resolve")
}

fn callable(mut definition: DomainPackDefinition) -> DomainPackDefinition {
    definition.metadata.availability = DomainPackAvailability::Available;
    definition
}

fn descriptor_for(service_id: &str) -> ServiceDescriptor {
    ServiceDescriptor::new(
        KernelServiceId::new(service_id),
        ServiceType::new(format!("{service_id}.domain_pack")),
        TraceSchemaRef::new(format!("trace.{service_id}.v1")),
    )
}
