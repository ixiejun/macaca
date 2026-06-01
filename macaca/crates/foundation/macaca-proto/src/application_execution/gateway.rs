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
///
/// This command is the low-level Adapter path for participants that already
/// know how to build a sanitized protocol event.  The gateway still validates
/// callback identity, optional lease binding, event schema version, payload
/// bounds, and idempotency before the EventLog append occurs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppendExecutionEventCommand {
    /// Optional lease issued by the provider strategy for this callback scope.
    pub lease_id: Option<String>,
    /// Reference to the scoped callback identity; it is never a raw secret.
    pub callback_identity_ref: String,
    /// Fully formed, sanitized event envelope requested by the participant.
    pub event: ApplicationExecutionEventEnvelope,
}

/// Gateway ingress command for provider heartbeat.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportExecutionHeartbeatCommand {
    /// Optional lease used to bind this callback to one provider/session/run.
    pub lease_id: Option<String>,
    /// Callback identity reference compared with the lease or gateway policy.
    pub callback_identity_ref: String,
    /// Application/session/run scope that the heartbeat claims to update.
    pub scope: ApplicationExecutionScope,
    /// Provider identity used for audit and optional lease binding checks.
    pub provider_id: String,
    /// Provider strategy kind carried as data, not a branch selector.
    pub provider_kind: ApplicationExecutionProviderKind,
    /// Participant-reported time, used only as bounded diagnostic metadata.
    pub reported_at: DateTime<Utc>,
    /// Trace context for audit correlation across transport boundaries.
    pub trace: TraceContext,
    /// Callback-declared event schema version that the gateway validates.
    pub event_schema_version: String,
    /// Retry-safe key for heartbeat callback de-duplication.
    pub idempotency_key: String,
}

/// Gateway ingress command for bounded provider snapshot reporting.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReportExecutionSnapshotCommand {
    /// Optional lease used to bind this callback to one provider/session/run.
    pub lease_id: Option<String>,
    /// Callback identity reference compared with the lease or gateway policy.
    pub callback_identity_ref: String,
    /// Bounded Memento-style snapshot; raw provider state must stay outside it.
    pub snapshot: ApplicationExecutionSnapshot,
    /// Trace context for audit correlation across transport boundaries.
    pub trace: TraceContext,
    /// Callback-declared event schema version that the gateway validates.
    pub event_schema_version: String,
    /// Retry-safe key for snapshot callback de-duplication.
    pub idempotency_key: String,
}

/// Gateway ingress command for provider-owned approval requests.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequestExecutionApprovalCommand {
    /// Optional lease used to bind this callback to one provider/session/run.
    pub lease_id: Option<String>,
    /// Callback identity reference compared with the lease or gateway policy.
    pub callback_identity_ref: String,
    /// Application/session/run scope that the approval request belongs to.
    pub scope: ApplicationExecutionScope,
    /// Provider identity used for audit and optional lease binding checks.
    pub provider_id: String,
    /// Provider strategy kind carried as data, not a branch selector.
    pub provider_kind: ApplicationExecutionProviderKind,
    /// Stable approval reference that a control command can answer later.
    pub approval_ref: String,
    /// Sanitized approval prompt summary or payload reference.
    pub prompt: ApplicationExecutionPayload,
    /// Trace context for audit correlation across transport boundaries.
    pub trace: TraceContext,
    /// Callback-declared event schema version that the gateway validates.
    pub event_schema_version: String,
    /// Retry-safe key for approval-request callback de-duplication.
    pub idempotency_key: String,
}

/// Gateway ingress command for terminal success.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReportExecutionCompletionCommand {
    /// Optional lease used to bind this callback to one provider/session/run.
    pub lease_id: Option<String>,
    /// Callback identity reference compared with the lease or gateway policy.
    pub callback_identity_ref: String,
    /// Application/session/run scope that reached terminal success.
    pub scope: ApplicationExecutionScope,
    /// Provider identity used for audit and optional lease binding checks.
    pub provider_id: String,
    /// Provider strategy kind carried as data, not a branch selector.
    pub provider_kind: ApplicationExecutionProviderKind,
    /// Sanitized terminal result summary or payload reference.
    pub result: ApplicationExecutionPayload,
    /// Trace context for audit correlation across transport boundaries.
    pub trace: TraceContext,
    /// Callback-declared event schema version that the gateway validates.
    pub event_schema_version: String,
    /// Retry-safe key for completion callback de-duplication.
    pub idempotency_key: String,
}

/// Gateway ingress command for terminal failure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReportExecutionFailureCommand {
    /// Optional lease used to bind this callback to one provider/session/run.
    pub lease_id: Option<String>,
    /// Callback identity reference compared with the lease or gateway policy.
    pub callback_identity_ref: String,
    /// Application/session/run scope that reached terminal failure.
    pub scope: ApplicationExecutionScope,
    /// Provider identity used for audit and optional lease binding checks.
    pub provider_id: String,
    /// Provider strategy kind carried as data, not a branch selector.
    pub provider_kind: ApplicationExecutionProviderKind,
    /// Structured, sanitized terminal error reported by the participant.
    pub error: ApplicationExecutionError,
    /// Trace context for audit correlation across transport boundaries.
    pub trace: TraceContext,
    /// Callback-declared event schema version that the gateway validates.
    pub event_schema_version: String,
    /// Retry-safe key for failure callback de-duplication.
    pub idempotency_key: String,
}
