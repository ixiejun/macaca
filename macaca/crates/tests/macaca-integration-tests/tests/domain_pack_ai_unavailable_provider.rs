//! Integration tests for AI domain-pack unavailable provider wiring.
//!
//! AI packs are optional serviceized capabilities. These tests verify that the
//! generic runtime-host domain-pack bootstrap can bind descriptor-derived AI
//! unavailable providers through `ServiceRuntime` without constructing hosted
//! models, local runtimes, OCR engines, speech engines, evaluation runners, or
//! provider-specific adapters.

use std::sync::Arc;

use macaca_ipc::InMemoryServiceBusTraceSink;
use macaca_proto::{
    domain_pack_contract::{
        ai_embedding::{AI_EMBEDDING_COMMANDS, AI_EMBEDDING_PACK_ID, AI_EMBEDDING_SERVICE_ID},
        ai_llm::{AI_LLM_COMMANDS, AI_LLM_PACK_ID, AI_LLM_SERVICE_ID},
        ai_model_evaluation::{
            AI_MODEL_EVALUATION_COMMANDS, AI_MODEL_EVALUATION_PACK_ID,
            AI_MODEL_EVALUATION_SERVICE_ID,
        },
        ai_rerank::{AI_RERANK_COMMANDS, AI_RERANK_PACK_ID, AI_RERANK_SERVICE_ID},
        ai_speech::{AI_SPEECH_COMMANDS, AI_SPEECH_PACK_ID, AI_SPEECH_SERVICE_ID},
        ai_vision::{AI_VISION_COMMANDS, AI_VISION_PACK_ID, AI_VISION_SERVICE_ID},
    },
    KernelServiceId, ServiceBusSource, ServiceCommand, ServiceCommandName, ServiceDescriptor,
    ServiceHealth, ServiceLifecycleState, ServiceType, TraceContext, TraceSchemaRef,
};
use macaca_runtime_host::{
    bootstrap_domain_pack_services,
    domain_pack_service_provider::unavailable_domain_pack_provider_registration,
    InMemoryServiceRuntimeEventSink, ServiceRuntime, ServiceRuntimeConfig,
};

#[tokio::test]
async fn ai_unavailable_providers_bootstrap_and_reject_traced_calls_without_payload_echo() {
    let runtime_events = Arc::new(InMemoryServiceRuntimeEventSink::new());
    let bus_events = Arc::new(InMemoryServiceBusTraceSink::new());
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig {
        event_sink: Some(runtime_events.clone()),
        bus_trace_sink: Some(bus_events.clone()),
        ..Default::default()
    }));
    let cases = ai_cases();
    let registrations = cases.iter().map(|case| {
        unavailable_domain_pack_provider_registration(
            ServiceDescriptor::new(
                KernelServiceId::new(case.service_id),
                ServiceType::new("domain_pack.ai"),
                TraceSchemaRef::new(case.trace_schema),
            ),
            case.pack_id,
            case.reason_code,
            case.trace_suffix,
        )
    });

    let bundle = bootstrap_domain_pack_services(
        Arc::clone(&runtime),
        registrations,
        "integration-ai-unavailable-provider",
    )
    .await
    .expect("AI unavailable provider bootstrap should succeed");

    assert_eq!(bundle.started_services.len(), cases.len());
    assert_eq!(bundle.provider_snapshots.len(), cases.len());

    let snapshot = runtime
        .snapshot()
        .expect("runtime snapshot should remain available after AI provider bootstrap");
    assert_eq!(snapshot.services.len(), cases.len());

    for case in &cases {
        assert!(
            bundle
                .started_services
                .iter()
                .any(|started| started.as_str() == case.service_id),
            "expected {} to be started",
            case.service_id
        );
        assert!(
            snapshot
                .services
                .iter()
                .any(|service| service.descriptor.id.as_str() == case.service_id),
            "expected {} to be visible in the runtime snapshot",
            case.service_id
        );
        assert!(
            snapshot.services.iter().any(|service| {
                service.descriptor.id.as_str() == case.service_id
                    && matches!(service.health, ServiceHealth::Unavailable { .. })
            }),
            "expected {} runtime snapshot to retain provider-reported unavailable health",
            case.service_id
        );

        for command in case.commands {
            let trace_id = format!(
                "trace-ai-unavailable-{}-{}",
                case.trace_suffix,
                command.replace('.', "-")
            );
            let reply = runtime
                .call(
                    &KernelServiceId::new(case.service_id),
                    ServiceBusSource::new("domain-pack-ai-unavailable-provider-test"),
                    ServiceCommand::with_trace(
                        ServiceCommandName::new(*command),
                        serde_json::json!({
                            "raw_prompt": "must-not-leak",
                            "raw_media": "must-not-leak",
                            "provider_payload": "must-not-leak"
                        }),
                        TraceContext::new(trace_id.clone()),
                    ),
                )
                .await
                .expect("unavailable AI provider call should return a structured reply");

            assert!(reply.success, "{}", case.pack_id);
            assert_eq!(reply.status, "unavailable", "{}", case.pack_id);
            assert_eq!(
                reply.metadata.get("provider_class").map(String::as_str),
                Some("unavailable")
            );
            assert_eq!(
                reply.metadata.get("pack_id").map(String::as_str),
                Some(case.pack_id)
            );
            assert_eq!(
                reply.metadata.get("reason_code").map(String::as_str),
                Some(case.reason_code)
            );

            let output = reply
                .output
                .expect("unavailable reply should carry JSON output");
            assert_eq!(output["status"], "unavailable", "{}", case.pack_id);
            assert_eq!(output["pack_id"], case.pack_id, "{}", case.pack_id);
            assert_eq!(output["service_id"], case.service_id, "{}", case.pack_id);
            assert_eq!(output["command"], *command, "{}", case.pack_id);
            assert_eq!(output["reason_code"], case.reason_code, "{}", case.pack_id);
            assert!(
                !output.to_string().contains("must-not-leak"),
                "{} unavailable reply must not echo raw command payloads",
                case.pack_id
            );
            assert_trace_replay_evidence(&runtime_events, &bus_events, case, command, &trace_id);
        }

        runtime
            .stop(
                &KernelServiceId::new(case.service_id),
                TraceContext::new(format!("trace-ai-stop-{}", case.trace_suffix)),
            )
            .await
            .expect("unavailable AI provider should stop through ServiceRuntime");
        runtime
            .cleanup(
                &KernelServiceId::new(case.service_id),
                TraceContext::new(format!("trace-ai-cleanup-{}", case.trace_suffix)),
            )
            .await
            .expect("unavailable AI provider should clean up through ServiceRuntime");
    }

    let final_snapshot = runtime
        .snapshot()
        .expect("runtime snapshot should remain available after AI provider cleanup");
    for case in &cases {
        assert!(
            final_snapshot.services.iter().any(|service| {
                service.descriptor.id.as_str() == case.service_id
                    && matches!(service.lifecycle_state, ServiceLifecycleState::CleanedUp)
            }),
            "expected {} cleanup lifecycle to be retained in the runtime snapshot",
            case.service_id
        );
    }

    assert_sanitized_runtime_observability(&runtime_events, &bus_events);
}

struct AiUnavailableCase {
    pack_id: &'static str,
    service_id: &'static str,
    commands: &'static [&'static str],
    reason_code: &'static str,
    trace_schema: &'static str,
    trace_suffix: &'static str,
}

fn ai_cases() -> Vec<AiUnavailableCase> {
    vec![
        AiUnavailableCase {
            pack_id: AI_LLM_PACK_ID,
            service_id: AI_LLM_SERVICE_ID,
            commands: AI_LLM_COMMANDS,
            reason_code: "ai_llm_provider_not_installed",
            trace_schema: "trace.ai.llm.v1",
            trace_suffix: "llm",
        },
        AiUnavailableCase {
            pack_id: AI_EMBEDDING_PACK_ID,
            service_id: AI_EMBEDDING_SERVICE_ID,
            commands: AI_EMBEDDING_COMMANDS,
            reason_code: "ai_embedding_provider_not_installed",
            trace_schema: "trace.ai.embedding.v1",
            trace_suffix: "embedding",
        },
        AiUnavailableCase {
            pack_id: AI_RERANK_PACK_ID,
            service_id: AI_RERANK_SERVICE_ID,
            commands: AI_RERANK_COMMANDS,
            reason_code: "ai_rerank_provider_not_installed",
            trace_schema: "trace.ai.rerank.v1",
            trace_suffix: "rerank",
        },
        AiUnavailableCase {
            pack_id: AI_VISION_PACK_ID,
            service_id: AI_VISION_SERVICE_ID,
            commands: AI_VISION_COMMANDS,
            reason_code: "ai_vision_provider_not_installed",
            trace_schema: "trace.ai.vision.v1",
            trace_suffix: "vision",
        },
        AiUnavailableCase {
            pack_id: AI_SPEECH_PACK_ID,
            service_id: AI_SPEECH_SERVICE_ID,
            commands: AI_SPEECH_COMMANDS,
            reason_code: "ai_speech_provider_not_installed",
            trace_schema: "trace.ai.speech.v1",
            trace_suffix: "speech",
        },
        AiUnavailableCase {
            pack_id: AI_MODEL_EVALUATION_PACK_ID,
            service_id: AI_MODEL_EVALUATION_SERVICE_ID,
            commands: AI_MODEL_EVALUATION_COMMANDS,
            reason_code: "ai_model_evaluation_provider_not_installed",
            trace_schema: "trace.ai.model_evaluation.v1",
            trace_suffix: "model-evaluation",
        },
    ]
}

fn assert_trace_replay_evidence(
    runtime_events: &InMemoryServiceRuntimeEventSink,
    bus_events: &InMemoryServiceBusTraceSink,
    case: &AiUnavailableCase,
    command: &str,
    trace_id: &str,
) {
    let runtime_events = runtime_events
        .events()
        .expect("runtime event sink should be readable");
    assert!(
        runtime_events.iter().any(|event| {
            event.service_id.as_str() == case.service_id
                && event.operation == "service_runtime.call.dispatched"
                && event.trace_id.as_deref() == Some(trace_id)
                && event.payload["command"].as_str() == Some(command)
                && event.payload["admission_decorators"]
                    .as_array()
                    .is_some_and(|decorators| {
                        [
                            "policy",
                            "resource_placeholder",
                            "entitlement_placeholder",
                            "audit_placeholder",
                        ]
                        .iter()
                        .all(|name| decorators.iter().any(|value| value.as_str() == Some(*name)))
                    })
        }),
        "{} {} should emit replayable admission evidence",
        case.pack_id,
        command
    );
    assert!(
        runtime_events.iter().any(|event| {
            event.service_id.as_str() == case.service_id
                && event.operation == "service_runtime.call.completed"
                && event.trace_id.as_deref() == Some(trace_id)
                && event.payload["status"].as_str() == Some("unavailable")
        }),
        "{} {} should emit a replayable unavailable completion event",
        case.pack_id,
        command
    );

    let bus_events = bus_events
        .events()
        .expect("service bus trace sink should be readable");
    for event_type in [
        "service_bus.call.accepted",
        "service_bus.call.routed",
        "service_bus.call.completed",
    ] {
        assert!(
            bus_events.iter().any(|event| {
                event.target_service == case.service_id
                    && event.event_type == event_type
                    && event
                        .trace
                        .as_ref()
                        .is_some_and(|trace| trace.trace_id == trace_id)
            }),
            "{} {} should be trace-addressable through {}",
            case.pack_id,
            command,
            event_type
        );
    }
}

fn assert_sanitized_runtime_observability(
    runtime_events: &InMemoryServiceRuntimeEventSink,
    bus_events: &InMemoryServiceBusTraceSink,
) {
    let runtime_payloads = runtime_events
        .events()
        .expect("runtime event sink should be readable")
        .into_iter()
        .map(|event| event.payload.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    let bus_payloads = bus_events
        .events()
        .expect("service bus trace sink should be readable")
        .into_iter()
        .map(|event| event.payload.to_string())
        .collect::<Vec<_>>()
        .join("\n");

    for raw_fragment in [
        "must-not-leak",
        "raw_prompt",
        "raw_media",
        "provider_payload",
    ] {
        assert!(
            !runtime_payloads.contains(raw_fragment),
            "runtime events must not expose {raw_fragment}"
        );
        assert!(
            !bus_payloads.contains(raw_fragment),
            "service bus trace events must not expose {raw_fragment}"
        );
    }
}
