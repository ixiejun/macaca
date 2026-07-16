//! Catalog-wide conformance tests for generic domain-pack unavailable providers.
//!
//! The runtime-host unavailable provider is a reusable Null Object adapter. These
//! tests prove that every industrial pack descriptor can fail closed through the
//! same service boundary without any application-specific command handling,
//! provider construction, credential access, or payload echo.

use macaca_kernel::SystemService;
use macaca_proto::{
    industrial_reference_domain_pack_definitions, KernelServiceId, ServiceCommand,
    ServiceCommandName, ServiceDescriptor, ServiceHealth, ServiceType, TraceContext,
    TraceSchemaRef,
};
use macaca_runtime_host::domain_pack_service_provider::DomainPackUnavailableSystemServiceProvider;

#[tokio::test]
async fn unavailable_provider_covers_every_industrial_pack_command_without_payload_echo() {
    let mut pack_count = 0usize;
    let mut command_count = 0usize;

    for definition in industrial_reference_domain_pack_definitions() {
        pack_count += 1;
        for (service_id, commands) in &definition.metadata.service_command_schemas {
            let provider = DomainPackUnavailableSystemServiceProvider::new(
                descriptor_for(service_id),
                definition.pack_id.clone(),
                "provider_not_installed",
            );
            let descriptor = provider.descriptor();
            let snapshot = provider.snapshot();
            let health = provider.health().await.unwrap();

            assert_eq!(
                descriptor
                    .metadata
                    .get("provider_class")
                    .map(String::as_str),
                Some("unavailable")
            );
            assert_eq!(snapshot.pack_id, definition.pack_id);
            assert_eq!(snapshot.service_id, *service_id);
            assert!(matches!(health, ServiceHealth::Unavailable { .. }));

            for command_name in commands {
                command_count += 1;
                let result = provider
                    .call(ServiceCommand::with_trace(
                        ServiceCommandName::new(command_name.clone()),
                        serde_json::json!({
                            "raw_secret": "must-not-leak",
                            "provider_payload": {"private": true}
                        }),
                        TraceContext::new(format!(
                            "trace-unavailable-{}",
                            command_name.replace(['.', '_'], "-")
                        )),
                    ))
                    .await
                    .unwrap();

                assert_eq!(result.status, "unavailable");
                assert_eq!(result.output["status"], "unavailable");
                assert_eq!(result.output["pack_id"], definition.pack_id);
                assert_eq!(result.output["service_id"], *service_id);
                assert_eq!(result.output["command"], *command_name);
                assert_eq!(result.output["reason_code"], "provider_not_installed");
                assert!(!result.output.to_string().contains("must-not-leak"));
                assert!(!result.output.to_string().contains("provider_payload"));
            }
        }
    }

    assert_eq!(pack_count, 74);
    assert!(command_count > pack_count);
}

#[tokio::test]
async fn unavailable_provider_preserves_structured_reason_codes_without_fake_success() {
    for reason_code in [
        "provider_not_installed",
        "unsupported",
        "disabled",
        "missing_entitlement",
        "provider_health_failed",
    ] {
        let provider = DomainPackUnavailableSystemServiceProvider::new(
            descriptor_for("service.domain_pack.fixture"),
            "pack.fixture.v1",
            reason_code,
        );

        let result = provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new("fixture.command"),
                serde_json::json!({"payload": "redacted"}),
                TraceContext::new(format!("trace-{reason_code}")),
            ))
            .await
            .unwrap();

        assert_eq!(result.status, "unavailable");
        assert_eq!(result.output["reason_code"], reason_code);
        assert_ne!(result.status, "ok");
    }
}

fn descriptor_for(service_id: impl Into<String>) -> ServiceDescriptor {
    let service_id = service_id.into();
    ServiceDescriptor::new(
        KernelServiceId::new(service_id.clone()),
        ServiceType::new(format!("{service_id}.domain_pack")),
        TraceSchemaRef::new(format!("trace.{service_id}.v1")),
    )
}
