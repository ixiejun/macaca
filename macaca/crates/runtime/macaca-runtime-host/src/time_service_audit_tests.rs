//! Audit and replay contracts for foundation-time service calls.
//!
//! The test uses the shared ServiceRouter Observer so time calls inherit the
//! same redaction and replay behavior as every other service. It deliberately
//! verifies only hashes enter audit events, never parse input or timer handles.

use std::collections::BTreeMap;
use std::sync::Arc;

use macaca_kernel::SystemService;
use macaca_proto::{KernelServiceId, ServiceBusSource, ServiceCommandName, TraceContext};
use macaca_time::FrozenTimeProvider;

use crate::{
    InMemoryServiceCallAuditSink, InMemoryServiceContractRegistry, InMemoryServicePolicyEngine,
    ServicePolicyEngine, ServiceProviderInstance, ServiceRouteRequest, ServiceRouter,
    ServiceRuntime, ServiceRuntimeConfig, StaticServiceProviderFactory, TimeSystemServiceProvider,
};

#[tokio::test]
async fn time_router_replay_redacts_parse_input_and_reconstructs_clock_decision() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let provider: Arc<dyn SystemService> = Arc::new(TimeSystemServiceProvider::new(Arc::new(
        FrozenTimeProvider::new(42),
    )));
    let descriptor = provider.descriptor();
    let service_id = runtime
        .register_provider(
            &StaticServiceProviderFactory::new(ServiceProviderInstance::new(descriptor, provider)),
            Default::default(),
        )
        .await
        .unwrap();
    runtime
        .start(&service_id, TraceContext::new("trace-time-audit-start"))
        .await
        .unwrap();

    let sink = Arc::new(InMemoryServiceCallAuditSink::new());
    let policy: Arc<dyn ServicePolicyEngine> = Arc::new(InMemoryServicePolicyEngine::new());
    let router = ServiceRouter::new(
        Arc::clone(&runtime),
        ServiceBusSource::new("test.foundation.time"),
        Arc::new(InMemoryServiceContractRegistry::new()),
        policy,
    )
    .with_audit_sink(sink.clone());
    let trace_id = "trace-time-audit-parse";
    let response = router
        .route(ServiceRouteRequest {
            app_id: Some("app:generic".into()),
            tenant_id: None,
            session_id: Some("session:time".into()),
            service_id: KernelServiceId::new("service.foundation.time"),
            operation: ServiceCommandName::new("time.parse"),
            payload: serde_json::json!({
                "input_ref":"2024-01-01T00:00:00Z",
                "credential":"private-key-marker",
                "prompt":"raw-prompt-marker",
                "manifest":"raw-manifest-marker",
                "package":"raw-package-bytes-marker"
            }),
            metadata: BTreeMap::new(),
            trace: TraceContext::new(trace_id),
        })
        .await
        .unwrap();
    assert_eq!(response.output["epoch_millis"], 1_704_067_200_000_i64);

    let replay = router.replay_audit_by_trace_id(trace_id).unwrap();
    assert!(replay
        .iter()
        .any(|event| event.stage == "service_call_requested"));
    assert!(replay
        .iter()
        .any(|event| event.stage == "service_call_dispatched"));
    assert!(replay
        .iter()
        .any(|event| event.stage == "service_call_succeeded"));
    let audit_text = format!("{replay:?}");
    assert!(!audit_text.contains("2024-01-01T00:00:00Z"));
    assert!(!audit_text.contains("private-key-marker"));
    assert!(!audit_text.contains("raw-prompt-marker"));
    assert!(!audit_text.contains("raw-manifest-marker"));
    assert!(!audit_text.contains("raw-package-bytes-marker"));
    assert!(replay.iter().all(
        |event| event.input_hash.is_none() || event.input_hash.as_deref().unwrap().len() <= 32
    ));
}

#[tokio::test]
async fn every_time_command_has_a_sanitized_trace_replay_entry() {
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig::default()));
    let provider: Arc<dyn SystemService> = Arc::new(TimeSystemServiceProvider::new(Arc::new(
        FrozenTimeProvider::new(42),
    )));
    let descriptor = provider.descriptor();
    let service_id = runtime
        .register_provider(
            &StaticServiceProviderFactory::new(ServiceProviderInstance::new(descriptor, provider)),
            Default::default(),
        )
        .await
        .unwrap();
    runtime
        .start(&service_id, TraceContext::new("trace-time-audit-start-all"))
        .await
        .unwrap();
    let router = ServiceRouter::new(
        runtime,
        ServiceBusSource::new("test.foundation.time"),
        Arc::new(InMemoryServiceContractRegistry::new()),
        Arc::new(InMemoryServicePolicyEngine::new()),
    )
    .with_audit_sink(Arc::new(InMemoryServiceCallAuditSink::new()));

    let cases = [
        ("time.now", serde_json::json!({})),
        ("time.monotonic_now", serde_json::json!({})),
        ("time.clock_health", serde_json::json!({})),
        (
            "time.duration_between",
            serde_json::json!({"start":{"epoch_millis":1},"end":{"epoch_millis":2}}),
        ),
        (
            "time.add_duration",
            serde_json::json!({"instant":{"epoch_millis":1},"duration":{"millis":2}}),
        ),
        (
            "time.resolve_timezone",
            serde_json::json!({"zone_query":"UTC"}),
        ),
        (
            "time.convert_timezone",
            serde_json::json!({"instant":{"epoch_millis":1},"target_timezone":{"zone_id":"UTC"}}),
        ),
        (
            "time.calendar_convert",
            serde_json::json!({"target_calendar":{"calendar_id":"iso8601"}}),
        ),
        (
            "time.format",
            serde_json::json!({"instant":{"epoch_millis":0},"format":{"pattern_ref":"format:rfc3339"}}),
        ),
        (
            "time.parse",
            serde_json::json!({"input_ref":"2024-01-01T00:00:00Z"}),
        ),
        (
            "time.create_timer",
            serde_json::json!({"duration":{"millis":10}}),
        ),
        (
            "time.cancel_timer",
            serde_json::json!({"timer":{"timer_id":"unknown"}}),
        ),
        (
            "time.inspect_timer",
            serde_json::json!({"timer":{"timer_id":"unknown"}}),
        ),
        (
            "time.evaluate_deadline",
            serde_json::json!({"deadline":{"deadline":{"epoch_millis":1}}}),
        ),
    ];
    for (index, (operation, payload)) in cases.into_iter().enumerate() {
        let trace_id = format!("trace-time-replay-{index}");
        router
            .route(ServiceRouteRequest {
                app_id: Some("app:generic".into()),
                tenant_id: None,
                session_id: Some("session:time".into()),
                service_id: KernelServiceId::new("service.foundation.time"),
                operation: ServiceCommandName::new(operation),
                payload,
                metadata: BTreeMap::new(),
                trace: TraceContext::new(trace_id.clone()),
            })
            .await
            .unwrap();
        let replay = router.replay_audit_by_trace_id(&trace_id).unwrap();
        let succeeded = replay
            .iter()
            .find(|event| event.stage == "service_call_succeeded")
            .expect("successful calls emit replay evidence");
        assert_eq!(succeeded.operation.as_deref(), Some(operation));
        assert!(succeeded.output_hash.is_some());
        assert!(succeeded
            .replay_metadata
            .values()
            .all(|value| value.is_ascii()));
    }

    let now_replay = router
        .replay_audit_by_trace_id("trace-time-replay-0")
        .unwrap();
    let now_success = now_replay
        .iter()
        .find(|event| event.stage == "service_call_succeeded")
        .unwrap();
    assert_eq!(
        now_success.replay_metadata.get("replay.clock_source"),
        Some(&"wall_clock".into())
    );
    assert_eq!(
        now_success.replay_metadata.get("replay.provider_class"),
        Some(&"frozen-test-clock".into())
    );
    let monotonic_replay = router
        .replay_audit_by_trace_id("trace-time-replay-1")
        .unwrap();
    let monotonic_success = monotonic_replay
        .iter()
        .find(|event| event.stage == "service_call_succeeded")
        .unwrap();
    assert_eq!(
        monotonic_success
            .replay_metadata
            .get("replay.monotonic_unit"),
        Some(&"nanos".into())
    );
}
