//! External application backend transport adapters.
//!
//! This module holds the Bridge layer below the external backend provider
//! strategy.  The provider owns protocol validation and lease construction,
//! while this file owns delivery envelopes and concrete transport mechanics.
//! Keeping the split explicit prevents HTTP details from becoming application
//! execution semantics and gives tests/plugins a narrow replacement seam.

use async_trait::async_trait;
use macaca_proto::{
    ApplicationExecutionControlKind, ApplicationExecutionEventType, ApplicationExecutionPayload,
    ApplicationExecutionScope, ApplicationId, CapabilityId, ServiceError, TraceContext,
};
use serde::{Deserialize, Serialize};

/// Transport-neutral request sent to an external application backend start endpoint.
///
/// The DTO deliberately contains only bounded protocol metadata and sanitized
/// task input. It passes callback identity by reference, never as raw secret
/// material, so logs/tests can inspect routing without leaking credentials.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExternalApplicationBackendStartRequest {
    pub endpoint: String,
    pub application_id: ApplicationId,
    pub session_id: Option<String>,
    pub run_id: Option<String>,
    pub workspace_ref: Option<String>,
    pub task_input: ApplicationExecutionPayload,
    pub requested_capabilities: Vec<CapabilityId>,
    pub callback_gateway_ref: Option<String>,
    pub callback_identity_ref: String,
    pub lease_id: String,
    pub allowed_event_types: Vec<ApplicationExecutionEventType>,
    pub allowed_controls: Vec<ApplicationExecutionControlKind>,
    pub heartbeat_policy: macaca_proto::ApplicationExecutionHeartbeatPolicy,
    pub event_schema_version: String,
    pub trace: TraceContext,
    pub idempotency_key: String,
}

/// Transport-neutral control delivery request for external backends.
///
/// The provider creates this envelope after service-level policy and control
/// idempotency have run. Transport implementations may use HTTP, IPC, or a
/// plugin bridge, but all variants receive the same auditable command shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExternalApplicationBackendControlRequest {
    pub endpoint: String,
    pub scope: ApplicationExecutionScope,
    pub command: ApplicationExecutionControlKind,
    pub control_id: String,
    pub reason_code: String,
    pub trace: TraceContext,
    pub payload: Option<ApplicationExecutionPayload>,
    pub lease_id: Option<String>,
    pub idempotency_key: String,
}

/// Replaceable transport Adapter used by the external backend provider.
///
/// The trait is intentionally small: the provider owns protocol validation,
/// lease construction, and result shaping, while the transport only delivers a
/// sanitized request to the configured endpoint. Tests use a fake transport and
/// production composition uses the HTTP implementation without changing the
/// provider strategy.
#[async_trait]
pub trait ExternalApplicationBackendTransport: Send + Sync {
    async fn start(
        &self,
        request: ExternalApplicationBackendStartRequest,
    ) -> Result<(), ServiceError>;

    async fn control(
        &self,
        request: ExternalApplicationBackendControlRequest,
    ) -> Result<(), ServiceError>;
}

/// Default HTTP transport for manifest-declared external backend endpoints.
///
/// This is a generic protocol Adapter, not an application backend. It posts the
/// already-sanitized request envelope as JSON and maps HTTP failures into
/// structured service errors without logging response bodies or raw provider
/// payloads.
#[derive(Debug, Default)]
pub struct HttpExternalApplicationBackendTransport {
    client: reqwest::Client,
}

impl HttpExternalApplicationBackendTransport {
    /// Build the transport with reqwest's default client configuration.
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl ExternalApplicationBackendTransport for HttpExternalApplicationBackendTransport {
    async fn start(
        &self,
        request: ExternalApplicationBackendStartRequest,
    ) -> Result<(), ServiceError> {
        post_json(&self.client, &request.endpoint, &request).await
    }

    async fn control(
        &self,
        request: ExternalApplicationBackendControlRequest,
    ) -> Result<(), ServiceError> {
        post_json(&self.client, &request.endpoint, &request).await
    }
}

async fn post_json<T: Serialize>(
    client: &reqwest::Client,
    endpoint: &str,
    body: &T,
) -> Result<(), ServiceError> {
    let response = client
        .post(endpoint)
        .json(body)
        .send()
        .await
        .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?;
    let status = response.status();
    if !status.is_success() {
        return Err(ServiceError::AdapterFailure(format!(
            "external backend endpoint returned status {status}"
        )));
    }
    Ok(())
}
