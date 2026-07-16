use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
    time::Duration,
};

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::{
    CleanupPolicy, KernelServiceId, ServiceBusSource, ServiceCallResult, ServiceCommand,
    ServiceCommandName, ServiceDescriptor, ServiceError, ServiceHealth, ServiceLifecycleState,
    ServiceResult, ServiceType, TraceContext, TraceSchemaRef,
};
use macaca_runtime_host::{
    InMemoryServiceRuntimeEventSink, ServiceProviderInstance, ServiceRuntime, ServiceRuntimeConfig,
    ServiceRuntimeError, StaticServiceProviderFactory,
};

fn descriptor(service_id: &str) -> ServiceDescriptor {
    ServiceDescriptor::new(
        KernelServiceId::new(service_id),
        ServiceType::new("test.service"),
        TraceSchemaRef::new("trace.test.service.v1"),
    )
}

fn traced_command(trace_id: &str) -> ServiceCommand {
    ServiceCommand::with_trace(
        ServiceCommandName::new("invoke"),
        serde_json::json!({"input": true}),
        TraceContext::new(trace_id),
    )
}

fn runtime_with_controls(
    call_timeout: Option<Duration>,
    max_reply_output_bytes: usize,
    max_stream_frames: usize,
) -> (Arc<ServiceRuntime>, Arc<InMemoryServiceRuntimeEventSink>) {
    let events = Arc::new(InMemoryServiceRuntimeEventSink::new());
    let runtime = Arc::new(ServiceRuntime::new(ServiceRuntimeConfig {
        event_sink: Some(events.clone()),
        call_timeout,
        max_reply_output_bytes,
        max_stream_frames,
        ..Default::default()
    }));
    (runtime, events)
}

async fn register(runtime: &ServiceRuntime, service: Arc<dyn SystemService>) -> KernelServiceId {
    let descriptor = service.descriptor();
    runtime
        .register_provider(
            &StaticServiceProviderFactory::new(ServiceProviderInstance::new(descriptor, service)),
            Default::default(),
        )
        .await
        .unwrap()
}

struct SlowSystemService {
    descriptor: ServiceDescriptor,
    delay: Duration,
}

#[async_trait]
impl SystemService for SlowSystemService {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        tokio::time::sleep(self.delay).await;
        let trace = command.trace.ok_or(ServiceError::MissingTraceContext)?;
        Ok(ServiceCallResult {
            output: serde_json::json!({"status": "late"}),
            trace,
            status: "ok".into(),
            metadata: BTreeMap::new(),
            cleanup_hint: Some(CleanupPolicy::None),
        })
    }

    async fn stop(&self) -> ServiceResult<()> {
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        Ok(())
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        Ok(ServiceHealth::Healthy)
    }
}

struct StaticReplySystemService {
    descriptor: ServiceDescriptor,
    output: serde_json::Value,
    metadata: BTreeMap<String, String>,
}

#[async_trait]
impl SystemService for StaticReplySystemService {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = command.trace.ok_or(ServiceError::MissingTraceContext)?;
        Ok(ServiceCallResult {
            output: self.output.clone(),
            trace,
            status: "ok".into(),
            metadata: self.metadata.clone(),
            cleanup_hint: Some(CleanupPolicy::None),
        })
    }

    async fn stop(&self) -> ServiceResult<()> {
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        Ok(())
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        Ok(match self.descriptor.lifecycle_state {
            ServiceLifecycleState::Failed { ref reason } => ServiceHealth::Unavailable {
                reason: reason.clone(),
            },
            _ => ServiceHealth::Healthy,
        })
    }
}

struct CapturingMetadataSystemService {
    descriptor: ServiceDescriptor,
    seen_metadata: Arc<RwLock<Option<BTreeMap<String, String>>>>,
}

#[async_trait]
impl SystemService for CapturingMetadataSystemService {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = command.trace.ok_or(ServiceError::MissingTraceContext)?;
        let mut seen_metadata = self
            .seen_metadata
            .write()
            .map_err(|_| ServiceError::AdapterFailure("metadata capture lock poisoned".into()))?;
        *seen_metadata = Some(command.metadata);
        Ok(ServiceCallResult {
            output: serde_json::json!({"status": "captured"}),
            trace,
            status: "ok".into(),
            metadata: BTreeMap::new(),
            cleanup_hint: Some(CleanupPolicy::None),
        })
    }

    async fn stop(&self) -> ServiceResult<()> {
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        Ok(())
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        Ok(ServiceHealth::Healthy)
    }
}

#[tokio::test]
async fn service_runtime_times_out_slow_provider_with_auditable_state() {
    let (runtime, events) = runtime_with_controls(Some(Duration::from_millis(20)), 4096, 16);
    let service_id = register(
        &runtime,
        Arc::new(SlowSystemService {
            descriptor: descriptor("service.runtime.timeout"),
            delay: Duration::from_millis(250),
        }),
    )
    .await;
    runtime
        .start(&service_id, TraceContext::new("trace-timeout-start"))
        .await
        .unwrap();

    let err = runtime
        .call(
            &service_id,
            ServiceBusSource::new("test.source"),
            traced_command("trace-timeout-call"),
        )
        .await
        .unwrap_err();

    assert!(matches!(err, ServiceRuntimeError::CallTimedOut { .. }));
    let snapshot = runtime.snapshot().unwrap();
    assert!(matches!(
        snapshot.services[0].health,
        ServiceHealth::Degraded { .. }
    ));
    assert!(events
        .events()
        .unwrap()
        .iter()
        .any(|event| event.operation == "service_runtime.call.timed_out"));
}

#[tokio::test]
async fn service_runtime_strips_runtime_control_metadata_before_provider_dispatch() {
    let (runtime, _events) = runtime_with_controls(Some(Duration::from_secs(5)), 4096, 16);
    let seen_metadata = Arc::new(RwLock::new(None));
    let service_id = register(
        &runtime,
        Arc::new(CapturingMetadataSystemService {
            descriptor: descriptor("service.runtime.metadata-boundary"),
            seen_metadata: seen_metadata.clone(),
        }),
    )
    .await;
    runtime
        .start(&service_id, TraceContext::new("trace-metadata-start"))
        .await
        .unwrap();

    let mut command = traced_command("trace-metadata-call");
    command
        .metadata
        .insert("runtime.timeout_ms".into(), "1000".into());
    command
        .metadata
        .insert("runtime.cancellation_token".into(), "secret-token".into());
    command
        .metadata
        .insert("runtime.cancelled".into(), "false".into());
    command
        .metadata
        .insert("runtime.cancel_requested".into(), "false".into());
    command
        .metadata
        .insert("caller.metadata".into(), "preserved".into());

    runtime
        .call(&service_id, ServiceBusSource::new("test.source"), command)
        .await
        .unwrap();

    let captured = seen_metadata
        .read()
        .unwrap()
        .clone()
        .expect("provider should receive a command");
    assert_eq!(
        captured.get("caller.metadata").map(String::as_str),
        Some("preserved")
    );
    assert!(!captured.contains_key("runtime.timeout_ms"));
    assert!(!captured.contains_key("runtime.cancellation_token"));
    assert!(!captured.contains_key("runtime.cancelled"));
    assert!(!captured.contains_key("runtime.cancel_requested"));
}

#[tokio::test]
async fn service_runtime_cancels_active_call_without_logging_raw_token() {
    let (runtime, events) = runtime_with_controls(Some(Duration::from_secs(5)), 4096, 16);
    let service_id = register(
        &runtime,
        Arc::new(SlowSystemService {
            descriptor: descriptor("service.runtime.cancel"),
            delay: Duration::from_secs(5),
        }),
    )
    .await;
    runtime
        .start(&service_id, TraceContext::new("trace-cancel-start"))
        .await
        .unwrap();

    let mut command = traced_command("trace-cancel-call");
    command
        .metadata
        .insert("runtime.cancellation_token".into(), "secret-token".into());
    let call_runtime = runtime.clone();
    let call_service_id = service_id.clone();
    let handle = tokio::spawn(async move {
        call_runtime
            .call(
                &call_service_id,
                ServiceBusSource::new("test.source"),
                command,
            )
            .await
    });
    tokio::time::sleep(Duration::from_millis(20)).await;
    runtime
        .cancel_call(
            &service_id,
            "secret-token",
            TraceContext::new("trace-cancel-request"),
        )
        .unwrap();

    let err = handle.await.unwrap().unwrap_err();

    assert!(matches!(err, ServiceRuntimeError::CallCancelled { .. }));
    let emitted_events = events.events().unwrap();
    assert!(emitted_events
        .iter()
        .any(|event| event.operation == "service_runtime.call.cancel_requested"));
    assert!(emitted_events
        .iter()
        .any(|event| event.operation == "service_runtime.call.cancelled"));
    let encoded_payloads = emitted_events
        .iter()
        .map(|event| serde_json::to_string(&event.payload).unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(!encoded_payloads.contains("secret-token"));
}

#[tokio::test]
async fn service_runtime_rejects_unbounded_reply_output() {
    let (runtime, events) = runtime_with_controls(Some(Duration::from_secs(5)), 32, 16);
    let service_id = register(
        &runtime,
        Arc::new(StaticReplySystemService {
            descriptor: descriptor("service.runtime.output-bound"),
            output: serde_json::json!({"text": "x".repeat(128)}),
            metadata: BTreeMap::new(),
        }),
    )
    .await;
    runtime
        .start(&service_id, TraceContext::new("trace-output-start"))
        .await
        .unwrap();

    let err = runtime
        .call(
            &service_id,
            ServiceBusSource::new("test.source"),
            traced_command("trace-output-call"),
        )
        .await
        .unwrap_err();

    assert!(matches!(err, ServiceRuntimeError::ReplyTooLarge { .. }));
    assert!(events
        .events()
        .unwrap()
        .iter()
        .any(|event| event.operation == "service_runtime.call.output_rejected"));
}

#[tokio::test]
async fn service_runtime_rejects_unbounded_stream_frame_count() {
    let (runtime, events) = runtime_with_controls(Some(Duration::from_secs(5)), 4096, 2);
    let mut metadata = BTreeMap::new();
    metadata.insert("stream.frame_count".into(), "3".into());
    let service_id = register(
        &runtime,
        Arc::new(StaticReplySystemService {
            descriptor: descriptor("service.runtime.stream-bound"),
            output: serde_json::json!({"stream_frames": [1, 2, 3]}),
            metadata,
        }),
    )
    .await;
    runtime
        .start(&service_id, TraceContext::new("trace-stream-start"))
        .await
        .unwrap();

    let err = runtime
        .call(
            &service_id,
            ServiceBusSource::new("test.source"),
            traced_command("trace-stream-call"),
        )
        .await
        .unwrap_err();

    assert!(matches!(
        err,
        ServiceRuntimeError::StreamFrameLimitExceeded { .. }
    ));
    assert!(events
        .events()
        .unwrap()
        .iter()
        .any(|event| event.operation == "service_runtime.call.stream_rejected"));
}
