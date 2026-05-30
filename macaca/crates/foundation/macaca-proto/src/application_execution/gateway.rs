//! Gateway ingress commands for external execution participants.
//!
//! External application backends and remote agents cannot write authoritative
//! state directly to shells.  They report facts through these command DTOs so a
//! Macaca-owned gateway can validate identity, lease, schema, policy, and
//! sanitization before appending durable events.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::TraceContext;

use super::{
    ApplicationExecutionError, ApplicationExecutionEventEnvelope, ApplicationExecutionPayload,
    ApplicationExecutionProviderKind, ApplicationExecutionScope, ApplicationExecutionSnapshot,
};

/// Gateway ingress command for appending a provider event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppendExecutionEventCommand {
    pub lease_id: Option<String>,
    pub callback_identity_ref: String,
    pub event: ApplicationExecutionEventEnvelope,
}

/// Gateway ingress command for provider heartbeat.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportExecutionHeartbeatCommand {
    pub lease_id: Option<String>,
    pub callback_identity_ref: String,
    pub scope: ApplicationExecutionScope,
    pub provider_id: String,
    pub provider_kind: ApplicationExecutionProviderKind,
    pub reported_at: DateTime<Utc>,
    pub trace: TraceContext,
}

/// Gateway ingress command for bounded provider snapshot reporting.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReportExecutionSnapshotCommand {
    pub lease_id: Option<String>,
    pub callback_identity_ref: String,
    pub snapshot: ApplicationExecutionSnapshot,
    pub trace: TraceContext,
}

/// Gateway ingress command for provider-owned approval requests.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequestExecutionApprovalCommand {
    pub lease_id: Option<String>,
    pub callback_identity_ref: String,
    pub scope: ApplicationExecutionScope,
    pub approval_ref: String,
    pub prompt: ApplicationExecutionPayload,
    pub trace: TraceContext,
    pub idempotency_key: String,
}

/// Gateway ingress command for terminal success.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReportExecutionCompletionCommand {
    pub lease_id: Option<String>,
    pub callback_identity_ref: String,
    pub scope: ApplicationExecutionScope,
    pub result: ApplicationExecutionPayload,
    pub trace: TraceContext,
    pub idempotency_key: String,
}

/// Gateway ingress command for terminal failure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReportExecutionFailureCommand {
    pub lease_id: Option<String>,
    pub callback_identity_ref: String,
    pub scope: ApplicationExecutionScope,
    pub error: ApplicationExecutionError,
    pub trace: TraceContext,
    pub idempotency_key: String,
}
