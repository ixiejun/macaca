//! Catalog-wide canonical runtime-path tests for optional domain packs.
//!
//! The unavailable provider is intentionally generic, so this test proves every
//! declared command crosses the same registration, lifecycle, trace, decorator,
//! dispatch, event, and snapshot boundaries before it reports absence.  It does
//! not encode provider, application, or business-domain behavior.

use std::sync::Arc;

use macaca_proto::{
    industrial_reference_domain_pack_definitions, KernelServiceId, ServiceBusSource,
    ServiceCommand, ServiceCommandName, ServiceDescriptor, ServiceHealth, ServiceType,
    TraceContext, TraceSchemaRef,
};
use macaca_runtime_host::{
    domain_pack_service_provider::DomainPackUnavailableSystemServiceProvider,
    InMemoryServiceRuntimeEventSink, ServiceProviderInstance, ServiceRuntime, ServiceRuntimeConfig,
    StaticServiceProviderFactory,
};

#[tokio::test]
async fn every_industrial_command_reaches_unavailable_provider_through_service_runtime() {
    let mut pack_count = 0usize;
    let mut command_count = 0usize;

    for definition in industrial_reference_domain_pack_definitions() {
        pack_count += 1;
        let events = Arc::new(InMemoryServiceRuntimeEventSink::new());
        let runtime = ServiceRuntime::new(ServiceRuntimeConfig {
            event_sink: Some(events.clone()),
            ..Default::default()
        });

        for (service_name, commands) in &definition.metadata.service_command_schemas {
            let descriptor = descriptor_for(service_name);
            let provider = Arc::new(DomainPackUnavailableSystemServiceProvider::new(
                descriptor.clone(),
                definition.pack_id.clone(),
                "provider_not_installed",
            ));
            let service_id = runtime
                .register_provider(
                    &StaticServiceProviderFactory::new(ServiceProviderInstance::new(
                        descriptor, provider,
                    )),
                    Default::default(),
                )
                .await
                .expect("descriptor-owned unavailable provider should register");
            runtime
                .start(
                    &service_id,
                    TraceContext::new(format!("trace-start-{}", definition.pack_id)),
                )
                .await
                .expect("registered unavailable provider should start");

            for command_name in commands {
                command_count += 1;
                let reply = runtime
                    .call(
                        &service_id,
                        ServiceBusSource::new("domain-pack-conformance"),
                        ServiceCommand::with_trace(
                            ServiceCommandName::new(command_name.clone()),
                            serde_json::json!({
                                "raw_secret": "must-not-leak",
                                "provider_payload": {"private": true}
                            }),
                            TraceContext::new(format!(
                                "trace-runtime-{}",
                                command_name.replace(['.', '_'], "-")
                            )),
                        ),
                    )
                    .await
                    .expect("unavailable providers report structured results through runtime");

                assert_eq!(reply.status, "unavailable");
                assert_eq!(
                    reply
                        .output
                        .as_ref()
                        .and_then(|output| output["status"].as_str()),
                    Some("unavailable")
                );
                assert!(!reply.output.unwrap().to_string().contains("must-not-leak"));
            }

            let snapshot = runtime
                .snapshot()
                .expect("runtime snapshot should be available");
            assert!(snapshot.services.iter().any(|service| {
                service.descriptor.id == service_id
                    && matches!(service.health, ServiceHealth::Unavailable { .. })
            }));
        }

        let emitted_events = events.events().expect("event sink should remain available");
        assert!(emitted_events
            .iter()
            .any(|event| event.operation == "service_runtime.call.completed"));
        let trace_safe_events = emitted_events
            .iter()
            .map(|event| event.payload.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(!trace_safe_events.contains("must-not-leak"));
        assert!(!trace_safe_events.contains("provider_payload"));
    }

    assert_eq!(pack_count, 74);
    assert!(command_count > pack_count);
}

fn descriptor_for(service_id: &str) -> ServiceDescriptor {
    ServiceDescriptor::new(
        KernelServiceId::new(service_id),
        ServiceType::new(format!("{service_id}.domain_pack")),
        TraceSchemaRef::new(format!("trace.{service_id}.v1")),
    )
}
