//! Runtime-host provider for optional `service.realtime`.
//!
//! Realtime transports are optional modules.  This provider supplies the
//! service boundary, trace/audit wrapping, and Null Object unavailable behavior
//! while concrete WebRTC, websocket, or loopback adapters can be installed
//! behind the [`RealtimeProvider`] trait by composition roots.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::workbench::realtime::*;
use macaca_proto::{
    BoundedSummary, CleanupPolicy, ServiceCallResult, ServiceCommand, ServiceDescriptor,
    ServiceError, ServiceHealth, ServiceResult, TraceContext, WorkbenchCommandResult,
    WorkbenchCommandStatus,
};
use tracing::{info, warn};

#[async_trait]
pub trait RealtimeProvider: Send + Sync {
    async fn start(&self, request: RealtimeStartRequest) -> ServiceResult<RealtimeServiceResponse>;
    async fn append_text(
        &self,
        request: RealtimeTextAppendRequest,
    ) -> ServiceResult<RealtimeServiceResponse>;
    async fn append_audio(
        &self,
        request: RealtimeAudioAppendRequest,
    ) -> ServiceResult<RealtimeServiceResponse>;
    async fn stop(
        &self,
        request: RealtimeSessionRefRequest,
    ) -> ServiceResult<RealtimeServiceResponse>;
    async fn health(&self) -> ServiceResult<RealtimeServiceResponse>;
}

/// Optional realtime system service wrapper.
pub struct RealtimeSystemServiceProvider {
    descriptor: ServiceDescriptor,
    provider: Option<Arc<dyn RealtimeProvider>>,
    unavailable_reason: Option<String>,
}

impl RealtimeSystemServiceProvider {
    /// Build a service around a concrete realtime Adapter.
    pub fn new(provider: Arc<dyn RealtimeProvider>) -> Self {
        Self {
            descriptor: descriptor().base,
            provider: Some(provider),
            unavailable_reason: None,
        }
    }

    /// Build the default Null Object provider for hosts without realtime.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            descriptor: descriptor().base,
            provider: None,
            unavailable_reason: Some(reason.into()),
        }
    }

    fn trace(command: &ServiceCommand) -> ServiceResult<TraceContext> {
        command
            .trace
            .clone()
            .ok_or(ServiceError::MissingTraceContext)
    }

    fn result(
        response: RealtimeServiceResponse,
        trace: TraceContext,
        status: WorkbenchCommandStatus,
    ) -> ServiceResult<ServiceCallResult> {
        let result = WorkbenchCommandResult {
            trace: trace.clone(),
            status,
            output: Some(response),
            summary: None,
            audit_ref: Some(format!("realtime:{}", trace.trace_id)),
            completed_at: chrono::Utc::now(),
        };
        Ok(ServiceCallResult {
            output: serde_json::to_value(result)
                .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?,
            trace,
            status: "ok".into(),
            metadata: BTreeMap::new(),
            cleanup_hint: Some(CleanupPolicy::None),
        })
    }

    fn unavailable_result(&self, trace: TraceContext) -> ServiceResult<ServiceCallResult> {
        let reason = self
            .unavailable_reason
            .clone()
            .unwrap_or_else(|| "realtime provider is not configured".into());
        warn!(service_id = %self.descriptor.id, trace_id = %trace.trace_id, reason = %reason, "realtime optional provider unavailable");
        Self::result(
            RealtimeServiceResponse::Health {
                configured: false,
                reason: Some(reason.clone()),
            },
            trace,
            WorkbenchCommandStatus::Unavailable { reason },
        )
    }
}

#[async_trait]
impl SystemService for RealtimeSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, configured = self.provider.is_some(), "realtime service provider started");
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = Self::trace(&command)?;
        info!(service_id = %self.descriptor.id, command = %command.name, trace_id = %trace.trace_id, "realtime service command accepted");
        let Some(provider) = &self.provider else {
            return self.unavailable_result(trace);
        };
        let response = match command.name.as_str() {
            START_COMMAND => provider.start(decode(command.payload)?).await?,
            APPEND_TEXT_COMMAND => provider.append_text(decode(command.payload)?).await?,
            APPEND_AUDIO_COMMAND => provider.append_audio(decode(command.payload)?).await?,
            STOP_COMMAND => provider.stop(decode(command.payload)?).await?,
            HEALTH_COMMAND | "service.snapshot" => provider.health().await?,
            other => return Err(ServiceError::UnsupportedCommand(other.into())),
        };
        Self::result(response, trace, WorkbenchCommandStatus::Completed)
    }

    async fn stop(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, "realtime service provider stopped");
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, "realtime service cleanup completed");
        Ok(())
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        if let Some(reason) = &self.unavailable_reason {
            Ok(ServiceHealth::Unavailable {
                reason: reason.clone(),
            })
        } else {
            Ok(ServiceHealth::Healthy)
        }
    }
}

fn decode<T: serde::de::DeserializeOwned>(value: serde_json::Value) -> ServiceResult<T> {
    serde_json::from_value(value).map_err(|error| ServiceError::InvalidArgument(error.to_string()))
}

#[allow(dead_code)]
fn unavailable_session(reason: String) -> RealtimeSessionRecord {
    RealtimeSessionRecord {
        session_ref: "realtime-unavailable".into(),
        modality: RealtimeModality::TextAndAudio,
        transport: RealtimeTransportKind::Other("unavailable".into()),
        status: RealtimeSessionStatus::Unavailable,
        connection_state: BoundedSummary {
            text: reason,
            truncated: false,
            original_byte_len: None,
            artifact_ref: None,
        },
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }
}
