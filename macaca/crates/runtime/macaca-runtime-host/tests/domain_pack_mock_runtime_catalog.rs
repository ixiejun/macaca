//! Catalog-wide canonical runtime-path tests for the deterministic mock provider.
//!
//! The mock is a provider-neutral test Strategy. This integration test proves that
//! every declared industrial command is still registered, started, decorated,
//! dispatched, observed, and snapshotted by `ServiceRuntime`; mock behavior never
//! becomes an SDK or application-side bypass.

use std::sync::Arc;

use macaca_proto::{
    industrial_reference_domain_pack_definitions, KernelServiceId, ServiceBusSource,
    ServiceCommand, ServiceCommandName, ServiceDescriptor, ServiceHealth, ServiceType,
    TraceContext, TraceSchemaRef,
};
use macaca_runtime_host::{
    domain_pack_service_provider::DomainPackMockSystemServiceProvider,
    InMemoryServiceRuntimeEventSink, ServiceProviderInstance, ServiceRuntime, ServiceRuntimeConfig,
    StaticServiceProviderFactory,
};

#[tokio::test]
async fn every_industrial_command_reaches_mock_provider_through_service_runtime() {
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
            let provider = Arc::new(DomainPackMockSystemServiceProvider::new(
                descriptor.clone(),
                definition.pack_id.clone(),
            ));
            let service_id = runtime
                .register_provider(
                    &StaticServiceProviderFactory::new(ServiceProviderInstance::new(
                        descriptor, provider,
                    )),
                    Default::default(),
                )
                .await
                .expect("descriptor-owned mock provider should register");
            runtime
                .start(
                    &service_id,
                    TraceContext::new(format!("trace-mock-start-{}", definition.pack_id)),
                )
                .await
                .expect("registered mock provider should start");

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
                                "trace-mock-{}",
                                command_name.replace(['.', '_'], "-")
                            )),
                        ),
                    )
                    .await
                    .expect("mock providers should respond through runtime");

                assert_eq!(reply.status, "ok");
                assert_eq!(
                    reply
                        .output
                        .as_ref()
                        .and_then(|output| output["mock"].as_bool()),
                    Some(true)
                );
                assert!(!reply.output.unwrap().to_string().contains("must-not-leak"));
            }

            let snapshot = runtime
                .snapshot()
                .expect("runtime snapshot should be available");
            assert!(snapshot.services.iter().any(|service| {
                service.descriptor.id == service_id
                    && matches!(service.health, ServiceHealth::Healthy)
            }));
        }

        let events = events.events().expect("event sink should remain available");
        assert!(events
            .iter()
            .any(|event| event.operation == "service_runtime.call.completed"));
        let trace_safe_events = events
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
