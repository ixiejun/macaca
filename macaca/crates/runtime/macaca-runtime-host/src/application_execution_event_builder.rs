//! Builder helpers for application execution EventLog envelopes.
//!
//! Service providers use this tiny factory to create protocol events with the
//! same schema version and scope mapping.  It contains no business workflow
//! logic; callers choose the protocol event type and already-sanitized payload.

use macaca_proto::{
    ApplicationExecutionEventEnvelope, ApplicationExecutionEventType, ApplicationExecutionPayload,
    ApplicationExecutionProviderKind, ApplicationExecutionScope, TraceContext,
};

/// Build one unsaved application execution event envelope.
pub(crate) fn build_event(
    scope: &ApplicationExecutionScope,
    event_type: ApplicationExecutionEventType,
    provider_id: &str,
    provider_kind: ApplicationExecutionProviderKind,
    trace: TraceContext,
    payload: ApplicationExecutionPayload,
    idempotency_key: String,
) -> ApplicationExecutionEventEnvelope {
    ApplicationExecutionEventEnvelope {
        application_id: scope.application_id,
        session_id: scope.session_id.clone(),
        run_id: scope.run_id.clone(),
        seq: None,
        timestamp: chrono::Utc::now(),
        event_type,
        trace,
        actor: scope.actor.clone(),
        provider_id: provider_id.into(),
        provider_kind,
        visibility: "session".into(),
        causality: Vec::new(),
        sanitized_payload: payload,
        payload_ref: None,
        schema_version: "application-execution.v1".into(),
        idempotency_key,
    }
}
