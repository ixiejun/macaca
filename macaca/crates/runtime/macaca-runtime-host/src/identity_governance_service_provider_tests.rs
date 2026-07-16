//! Conformance tests shared by the organization and tenant provider adapters.
//!
//! The tests exercise the public `SystemService` boundary through the runtime,
//! proving that the adapters remain replaceable and that caller payloads do not
//! become audit, trace, snapshot, or result data.

use std::sync::Arc;

use macaca_kernel::SystemService;
use macaca_proto::{
    ServiceBusSource, ServiceCommand, ServiceCommandName, ServiceError, ServiceHealth, TraceContext,
};

use super::identity_organization_service_provider::IdentityOrganizationSystemServiceProvider;
use super::identity_tenant_service_provider::IdentityTenantSystemServiceProvider;
use crate::{
    InMemoryServiceRuntimeEventSink, ServiceProviderInstance, ServiceRuntime, ServiceRuntimeConfig,
    StaticServiceProviderFactory,
};

#[tokio::test]
async fn organization_commands_are_traceable_and_redacted_through_service_runtime() {
    assert_runtime_contract(
        Arc::new(IdentityOrganizationSystemServiceProvider::mock()),
        macaca_proto::domain_pack_contract::identity_organization::IDENTITY_ORGANIZATION_COMMANDS,
        "organization",
    )
    .await;
}

#[tokio::test]
async fn tenant_commands_are_traceable_and_redacted_through_service_runtime() {
    assert_runtime_contract(
        Arc::new(IdentityTenantSystemServiceProvider::mock()),
        macaca_proto::domain_pack_contract::identity_tenant::IDENTITY_TENANT_COMMANDS,
        "tenant",
    )
    .await;
}

#[tokio::test]
async fn governance_providers_fail_closed_when_unavailable_or_unsupported() {
    let organization = IdentityOrganizationSystemServiceProvider::unavailable("not_installed");
    let tenant = IdentityTenantSystemServiceProvider::unavailable("not_installed");
    for (provider, command) in [
        (&organization as &dyn SystemService, "organization.get"),
        (&tenant as &dyn SystemService, "tenant.get"),
    ] {
        assert!(matches!(
            provider.health().await.unwrap(),
            ServiceHealth::Unavailable { .. }
        ));
        assert!(matches!(
            provider
                .call(command_with_trace(command, "unavailable"))
                .await,
            Err(ServiceError::ServiceUnavailable(_))
        ));
    }
    let mock = IdentityOrganizationSystemServiceProvider::mock();
    assert!(matches!(
        mock.call(command_with_trace(
            "organization.unsupported",
            "unsupported"
        ))
        .await,
        Err(ServiceError::UnsupportedCommand(_))
    ));
}

#[tokio::test]
async fn governance_provider_snapshots_are_bounded_and_cleanup_releases_references() {
    let organization = IdentityOrganizationSystemServiceProvider::mock();
    let tenant = IdentityTenantSystemServiceProvider::mock();
    organization
        .call(command_with_trace("organization.get", "org-one"))
        .await
        .unwrap();
    tenant
        .call(command_with_trace("tenant.get", "tenant-one"))
        .await
        .unwrap();
    assert_eq!(organization.snapshot().await["active_reference_count"], "1");
    assert_eq!(tenant.snapshot().await["active_reference_count"], "1");
    organization.cleanup().await.unwrap();
    tenant.cleanup().await.unwrap();
    assert_eq!(organization.snapshot().await["active_reference_count"], "0");
    assert_eq!(tenant.snapshot().await["active_reference_count"], "0");
}

async fn assert_runtime_contract<S>(provider: Arc<S>, commands: &[&str], label: &str)
where
    S: SystemService + 'static,
{
    let events = Arc::new(InMemoryServiceRuntimeEventSink::new());
    let runtime = ServiceRuntime::new(ServiceRuntimeConfig {
        event_sink: Some(events.clone()),
        ..Default::default()
    });
    let service_id = runtime
        .register_provider(
            &StaticServiceProviderFactory::new(ServiceProviderInstance::new(
                provider.descriptor(),
                provider,
            )),
            Default::default(),
        )
        .await
        .unwrap();
    runtime
        .start(&service_id, TraceContext::new(format!("{label}-start")))
        .await
        .unwrap();

    for command in commands {
        let trace_id = format!("{label}-{command}");
        let reply = runtime
            .call(
                &service_id,
                ServiceBusSource::new("identity-governance-conformance"),
                ServiceCommand::with_trace(
                    ServiceCommandName::new(*command),
                    serde_json::json!({"credential": "secret-marker", "provider_payload": "raw-marker"}),
                    TraceContext::new(trace_id.clone()),
                ),
            )
            .await
            .unwrap();
        assert_eq!(reply.status, "ok");
        assert!(!reply.output.unwrap().to_string().contains("marker"));
        assert!(events.events().unwrap().iter().any(|event| {
            event.trace_id.as_deref() == Some(trace_id.as_str())
                && event.operation == "service_runtime.call.completed"
        }));
    }
    let observable = format!("{:?}", events.events().unwrap());
    assert!(!observable.contains("secret-marker"));
    assert!(!observable.contains("raw-marker"));
}

fn command_with_trace(command: &str, trace_id: &str) -> ServiceCommand {
    ServiceCommand::with_trace(
        ServiceCommandName::new(command),
        serde_json::json!({}),
        TraceContext::new(trace_id),
    )
}
