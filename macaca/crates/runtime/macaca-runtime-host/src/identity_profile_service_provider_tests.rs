//! Contract tests for the provider-neutral profile runtime adapter.

use std::sync::Arc;

use macaca_kernel::SystemService;
use macaca_proto::domain_pack_contract::identity_profile::IDENTITY_PROFILE_COMMANDS;
use macaca_proto::{
    ServiceBusSource, ServiceCommand, ServiceCommandName, ServiceError, ServiceHealth, TraceContext,
};

use super::identity_profile_service_provider::IdentityProfileSystemServiceProvider;
use crate::{
    InMemoryServiceRuntimeEventSink, ServiceProviderInstance, ServiceRuntime, ServiceRuntimeConfig,
    StaticServiceProviderFactory,
};

#[tokio::test]
async fn profile_commands_are_traceable_and_redacted_through_service_runtime() {
    let events = Arc::new(InMemoryServiceRuntimeEventSink::new());
    let runtime = ServiceRuntime::new(ServiceRuntimeConfig {
        event_sink: Some(events.clone()),
        ..Default::default()
    });
    let provider = Arc::new(IdentityProfileSystemServiceProvider::mock());
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
        .start(&service_id, TraceContext::new("profile-start"))
        .await
        .unwrap();

    for command in IDENTITY_PROFILE_COMMANDS {
        let trace_id = format!("profile-{command}");
        let reply = runtime.call(
            &service_id,
            ServiceBusSource::new("identity-profile-conformance"),
            ServiceCommand::with_trace(
                ServiceCommandName::new(*command),
                serde_json::json!({"credential":"secret-marker", "avatar_bytes":"raw-marker", "provider_payload":"private-marker"}),
                TraceContext::new(trace_id.clone()),
            ),
        ).await.unwrap();
        assert_eq!(reply.status, "ok");
        assert!(!reply.output.unwrap().to_string().contains("marker"));
        assert!(events
            .events()
            .unwrap()
            .iter()
            .any(|event| event.trace_id.as_deref() == Some(trace_id.as_str())
                && event.operation == "service_runtime.call.completed"));
    }
    let observable = format!("{:?}", events.events().unwrap());
    assert!(!observable.contains("secret-marker"));
    assert!(!observable.contains("raw-marker"));
    assert!(!observable.contains("private-marker"));
}

#[tokio::test]
async fn profile_provider_fails_closed_and_releases_bounded_state() {
    let unavailable = IdentityProfileSystemServiceProvider::unavailable("not_installed");
    assert!(matches!(
        unavailable.health().await.unwrap(),
        ServiceHealth::Unavailable { .. }
    ));
    assert!(matches!(
        unavailable
            .call(command("profile.read_profile", "unavailable"))
            .await,
        Err(ServiceError::ServiceUnavailable(_))
    ));
    let provider = IdentityProfileSystemServiceProvider::mock();
    assert!(matches!(
        provider
            .call(command("profile.unsupported", "unsupported"))
            .await,
        Err(ServiceError::UnsupportedCommand(_))
    ));
    provider
        .call(command("profile.read_profile", "one"))
        .await
        .unwrap();
    assert_eq!(provider.snapshot().await["active_reference_count"], "1");
    provider.cleanup().await.unwrap();
    assert_eq!(provider.snapshot().await["active_reference_count"], "0");
    assert_eq!(
        provider.capability().feature_flags.len(),
        IDENTITY_PROFILE_COMMANDS.len()
    );
}

#[tokio::test]
async fn profile_admission_denies_policy_facts_before_reference_state() {
    let provider = IdentityProfileSystemServiceProvider::mock();
    for (trace, payload) in [
        ("policy", serde_json::json!({"policy_denied": true})),
        (
            "entitlement",
            serde_json::json!({"entitlement_missing": true}),
        ),
        ("privacy", serde_json::json!({"privacy_denied": true})),
        ("avatar", serde_json::json!({"avatar_denied": true})),
        ("stale", serde_json::json!({"stale_data": true})),
    ] {
        let result = provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new("profile.plan_patch"),
                payload,
                TraceContext::new(trace),
            ))
            .await;
        assert!(matches!(result, Err(ServiceError::DisabledByPolicy(_))));
    }
    assert_eq!(provider.snapshot().await["active_reference_count"], "0");
}

#[test]
fn profile_capability_reports_unavailable_provider_class() {
    let provider = IdentityProfileSystemServiceProvider::unavailable("module_absent");
    assert_eq!(provider.capability().provider_class, "unavailable");
    assert!(matches!(
        provider.capability().state,
        macaca_proto::DomainPackProviderCapabilityState::Unavailable
    ));
    assert_eq!(
        provider.descriptor().metadata.get("provider_class"),
        Some(&"unavailable".to_string())
    );
}

fn command(name: &str, trace_id: &str) -> ServiceCommand {
    ServiceCommand::with_trace(
        ServiceCommandName::new(name),
        serde_json::json!({}),
        TraceContext::new(trace_id),
    )
}
