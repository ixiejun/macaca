//! Conformance tests for the auth-handoff adapter and its replay guard.

use std::sync::Arc;

use macaca_kernel::SystemService;
use macaca_proto::domain_pack_contract::identity_auth_handoff::IDENTITY_AUTH_HANDOFF_COMMANDS;
use macaca_proto::{
    ServiceBusSource, ServiceCommand, ServiceCommandName, ServiceError, ServiceHealth, TraceContext,
};

use super::identity_auth_handoff_service_provider::IdentityAuthHandoffSystemServiceProvider;
use crate::{
    InMemoryServiceRuntimeEventSink, ServiceProviderInstance, ServiceRuntime, ServiceRuntimeConfig,
    StaticServiceProviderFactory,
};

#[tokio::test]
async fn auth_handoff_commands_are_traceable_and_redacted_through_service_runtime() {
    let events = Arc::new(InMemoryServiceRuntimeEventSink::new());
    let runtime = ServiceRuntime::new(ServiceRuntimeConfig {
        event_sink: Some(events.clone()),
        ..Default::default()
    });
    let provider = Arc::new(IdentityAuthHandoffSystemServiceProvider::mock());
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
        .start(&service_id, TraceContext::new("auth-handoff-start"))
        .await
        .unwrap();
    for command in IDENTITY_AUTH_HANDOFF_COMMANDS {
        let trace_id = format!("auth-handoff-{command}");
        let reply = runtime.call(&service_id, ServiceBusSource::new("auth-handoff-conformance"), ServiceCommand::with_trace(
            ServiceCommandName::new(*command),
            serde_json::json!({"authorization_code":"secret-marker", "token":"raw-marker", "assertion":"private-marker"}),
            TraceContext::new(trace_id.clone()),
        )).await.unwrap();
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
async fn auth_handoff_rejects_reused_callback_reference_before_a_successful_provider_result() {
    let provider = IdentityAuthHandoffSystemServiceProvider::mock();
    let mut first = command("auth_handoff.verify_callback", "callback-one");
    first
        .metadata
        .insert("callback_ref_hash".into(), "callback-hash".into());
    provider.call(first).await.unwrap();
    let mut duplicate = command("auth_handoff.verify_callback", "callback-two");
    duplicate
        .metadata
        .insert("callback_ref_hash".into(), "callback-hash".into());
    assert!(
        matches!(provider.call(duplicate).await, Err(ServiceError::AdapterFailure(message)) if message == "replay_rejected")
    );
    assert_eq!(provider.snapshot().await["active_reference_count"], "1");
}

#[tokio::test]
async fn auth_handoff_provider_fails_closed_and_clears_runtime_state() {
    let unavailable = IdentityAuthHandoffSystemServiceProvider::unavailable("not_installed");
    assert!(matches!(
        unavailable.health().await.unwrap(),
        ServiceHealth::Unavailable { .. }
    ));
    assert!(matches!(
        unavailable
            .call(command("auth_handoff.start_handoff", "unavailable"))
            .await,
        Err(ServiceError::ServiceUnavailable(_))
    ));
    let provider = IdentityAuthHandoffSystemServiceProvider::mock();
    assert!(matches!(
        provider
            .call(command("auth_handoff.unsupported", "unsupported"))
            .await,
        Err(ServiceError::UnsupportedCommand(_))
    ));
    provider
        .call(command("auth_handoff.start_handoff", "one"))
        .await
        .unwrap();
    provider.cleanup().await.unwrap();
    assert_eq!(provider.snapshot().await["active_reference_count"], "0");
    assert_eq!(provider.snapshot().await["consumed_callback_count"], "0");
}

#[tokio::test]
async fn auth_handoff_admission_denies_policy_facts_before_provider_state() {
    let provider = IdentityAuthHandoffSystemServiceProvider::mock();
    for (trace, payload) in [
        ("policy", serde_json::json!({"policy_denied": true})),
        (
            "entitlement",
            serde_json::json!({"entitlement_missing": true}),
        ),
        ("approval", serde_json::json!({"approval_required": true})),
        ("redirect", serde_json::json!({"redirect_denied": true})),
        (
            "protocol",
            serde_json::json!({"protocol_unsupported": true}),
        ),
        ("stale", serde_json::json!({"stale_data": true})),
    ] {
        let result = provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new("auth_handoff.start_handoff"),
                payload,
                TraceContext::new(trace),
            ))
            .await;
        assert!(matches!(result, Err(ServiceError::DisabledByPolicy(_))));
    }
    assert_eq!(provider.snapshot().await["active_reference_count"], "0");
}

fn command(name: &str, trace_id: &str) -> ServiceCommand {
    ServiceCommand::with_trace(
        ServiceCommandName::new(name),
        serde_json::json!({}),
        TraceContext::new(trace_id),
    )
}
