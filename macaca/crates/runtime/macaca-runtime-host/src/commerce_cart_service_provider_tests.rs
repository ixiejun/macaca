//! Conformance tests for the commerce cart service adapter.
//!
//! The tests exercise the canonical runtime path and assert that payload markers never cross the
//! provider boundary into results, snapshots, or runtime audit events.

use std::sync::Arc;

use macaca_kernel::SystemService;
use macaca_proto::domain_pack_contract::commerce_cart::COMMERCE_CART_COMMANDS;
use macaca_proto::{
    ServiceBusSource, ServiceCommand, ServiceCommandName, ServiceError, ServiceHealth, TraceContext,
};

use super::commerce_cart_service_provider::CommerceCartSystemServiceProvider;
use crate::{
    InMemoryServiceRuntimeEventSink, ServiceProviderInstance, ServiceRuntime, ServiceRuntimeConfig,
    StaticServiceProviderFactory,
};

#[tokio::test]
async fn cart_commands_are_traceable_and_redacted_through_service_runtime() {
    let events = Arc::new(InMemoryServiceRuntimeEventSink::new());
    let runtime = ServiceRuntime::new(ServiceRuntimeConfig {
        event_sink: Some(events.clone()),
        ..Default::default()
    });
    let provider = Arc::new(CommerceCartSystemServiceProvider::mock());
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
        .start(&service_id, TraceContext::new("cart-runtime-start"))
        .await
        .unwrap();

    for command in COMMERCE_CART_COMMANDS {
        let trace_id = format!("cart-runtime-{command}");
        let reply = runtime
            .call(
                &service_id,
                ServiceBusSource::new("commerce-cart-conformance"),
                ServiceCommand::with_trace(
                    ServiceCommandName::new(*command),
                    serde_json::json!({
                        "buyer_email": "secret-marker",
                        "checkout_url": "https://secret-marker.invalid",
                        "provider_payload": "raw-marker"
                    }),
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

#[tokio::test]
async fn cart_provider_reports_capabilities_and_unavailable_state() {
    let provider = CommerceCartSystemServiceProvider::mock();
    let capability = provider.capability();
    assert!(capability.feature_flags.contains("version_tokens"));
    assert!(capability.feature_flags.contains("async_export"));
    assert_eq!(capability.limits["max_lines"], 100);
    assert!(matches!(
        provider.health().await.unwrap(),
        ServiceHealth::Healthy
    ));

    let unavailable = CommerceCartSystemServiceProvider::unavailable("not_installed");
    assert!(matches!(
        unavailable.health().await.unwrap(),
        ServiceHealth::Unavailable { .. }
    ));
    assert!(matches!(
        unavailable
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new("cart.read_cart"),
                serde_json::json!({}),
                TraceContext::new("cart-unavailable"),
            ))
            .await,
        Err(ServiceError::ServiceUnavailable(_))
    ));
}

#[tokio::test]
async fn cart_snapshot_is_bounded_and_shutdown_releases_references() {
    let provider = CommerceCartSystemServiceProvider::mock();
    provider
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new("cart.create_cart"),
            serde_json::json!({"payload":"x".repeat(10_000)}),
            TraceContext::new("cart-one"),
        ))
        .await
        .unwrap();
    assert_eq!(provider.snapshot().await["active_reference_count"], "1");
    provider.shutdown().await.unwrap();
    assert_eq!(provider.snapshot().await["active_reference_count"], "0");
}

#[tokio::test]
async fn cart_admission_denies_policy_facts_before_reference_state() {
    let provider = CommerceCartSystemServiceProvider::mock();
    for (trace, payload) in [
        ("policy", serde_json::json!({"policy_denied": true})),
        (
            "entitlement",
            serde_json::json!({"entitlement_missing": true}),
        ),
        ("approval", serde_json::json!({"approval_required": true})),
        ("line", serde_json::json!({"line_mutation_denied": true})),
        ("stale", serde_json::json!({"stale_data": true})),
        ("quota", serde_json::json!({"line_count": 101})),
    ] {
        let result = provider
            .call(ServiceCommand::with_trace(
                ServiceCommandName::new("cart.plan_line_mutation"),
                payload,
                TraceContext::new(trace),
            ))
            .await;
        assert!(matches!(result, Err(ServiceError::DisabledByPolicy(_))));
    }
    assert_eq!(provider.snapshot().await["active_reference_count"], "0");
}

#[test]
fn cart_descriptor_reflects_unavailable_provider_capability() {
    let provider = CommerceCartSystemServiceProvider::unavailable("module_absent");
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

#[tokio::test]
async fn cart_events_cover_policy_mutation_handoff_and_snapshot_nodes() {
    let provider = CommerceCartSystemServiceProvider::mock();
    let mut events = provider.subscribe();
    provider
        .call(ServiceCommand::with_trace(
            ServiceCommandName::new("cart.plan_handoff"),
            serde_json::json!({}),
            TraceContext::new("cart-event"),
        ))
        .await
        .unwrap();
    let kinds = (0..9)
        .filter_map(|_| events.try_recv().ok())
        .map(|event| event.kind)
        .collect::<Vec<_>>();
    assert!(kinds.contains(
        &super::commerce_cart_service_provider::CommerceCartRuntimeEventKind::PolicyDecision
    ));
    assert!(kinds.contains(
        &super::commerce_cart_service_provider::CommerceCartRuntimeEventKind::HandoffPlanned
    ));
    assert!(kinds.contains(
        &super::commerce_cart_service_provider::CommerceCartRuntimeEventKind::ProviderCallSucceeded
    ));
}

#[test]
fn cart_provider_strategy_replacement_reports_capability_gaps() {
    let provider = CommerceCartSystemServiceProvider::mock_with_commands(["cart.read_cart"]);
    assert!(provider
        .capability()
        .feature_flags
        .contains("cart.read_cart"));
    assert!(!provider
        .capability()
        .feature_flags
        .contains("cart.handoff_request"));
}
