//! Event-sourced projection helpers for application execution sessions.
//!
//! The application execution service treats persisted events as the source of
//! truth.  This module owns the deterministic reducer that turns those events
//! into the current state returned to SDKs, Web adapters, CLIs, and app-owned
//! UIs.  Keeping the reducer pure makes replay auditable and prevents browser
//! caches from becoming authoritative state.

use macaca_proto::{
    ApplicationExecutionCurrentState, ApplicationExecutionEventEnvelope,
    ApplicationExecutionEventType, ApplicationExecutionLifecycleState, ApplicationExecutionPayload,
    ApplicationExecutionProviderKind, ApplicationExecutionScope,
};

/// Build a current-state projection from ordered application execution events.
///
/// The reducer intentionally uses only protocol facts from each event envelope.
/// It does not inspect application ids, app names, workflow names, model names,
/// or provider-specific payload shapes.  Event payloads may carry generic hints
/// such as `approval_ref`, `checkpoint_ref`, `control_id`, or `summary`, but the
/// absence of those hints never changes provider-neutral lifecycle semantics.
pub fn project_application_execution_state(
    scope: ApplicationExecutionScope,
    events: &[ApplicationExecutionEventEnvelope],
    replay_cursor: Option<String>,
) -> ApplicationExecutionCurrentState {
    let mut state = ApplicationExecutionCurrentState {
        scope,
        lifecycle_state: ApplicationExecutionLifecycleState::Accepted,
        provider_id: None,
        provider_kind: ApplicationExecutionProviderKind::Unavailable,
        latest_heartbeat_at: None,
        pending_approval_refs: Vec::new(),
        active_control_ids: Vec::new(),
        latest_checkpoint_ref: None,
        step_summaries: Vec::new(),
        terminal_result: None,
        terminal_error: None,
        replay_cursor,
    };

    for event in events {
        state.provider_id = Some(event.provider_id.clone());
        state.provider_kind = event.provider_kind;
        match event.event_type {
            ApplicationExecutionEventType::SessionStarted
            | ApplicationExecutionEventType::ExecutionAccepted
            | ApplicationExecutionEventType::ProviderAssigned => {
                state.lifecycle_state = ApplicationExecutionLifecycleState::Running;
            }
            ApplicationExecutionEventType::ProviderHeartbeat => {
                state.latest_heartbeat_at = Some(event.timestamp);
            }
            ApplicationExecutionEventType::ApprovalRequested => {
                state.lifecycle_state = ApplicationExecutionLifecycleState::WaitingForApproval;
                if let Some(approval_ref) = payload_string(&event.sanitized_payload, "approval_ref")
                {
                    push_unique(&mut state.pending_approval_refs, approval_ref);
                }
            }
            ApplicationExecutionEventType::ApprovalResolved => {
                if let Some(approval_ref) = payload_string(&event.sanitized_payload, "approval_ref")
                {
                    state
                        .pending_approval_refs
                        .retain(|value| value != &approval_ref);
                }
                if state.pending_approval_refs.is_empty() && !state.lifecycle_state.is_terminal() {
                    state.lifecycle_state = ApplicationExecutionLifecycleState::Running;
                }
            }
            ApplicationExecutionEventType::CheckpointCreated => {
                state.latest_checkpoint_ref =
                    payload_string(&event.sanitized_payload, "checkpoint_ref");
            }
            ApplicationExecutionEventType::ControlRequested
            | ApplicationExecutionEventType::ControlDelivered => {
                if let Some(control_id) = payload_string(&event.sanitized_payload, "control_id") {
                    push_unique(&mut state.active_control_ids, control_id);
                }
            }
            ApplicationExecutionEventType::ControlCompleted => {
                if let Some(control_id) = payload_string(&event.sanitized_payload, "control_id") {
                    state
                        .active_control_ids
                        .retain(|value| value != &control_id);
                }
            }
            ApplicationExecutionEventType::LlmRequested
            | ApplicationExecutionEventType::LlmCompleted
            | ApplicationExecutionEventType::ToolCallRequested
            | ApplicationExecutionEventType::ToolCallDispatched
            | ApplicationExecutionEventType::ToolCallCompleted
            | ApplicationExecutionEventType::ProviderSnapshot => {
                if !event.sanitized_payload.summary.is_empty() {
                    state
                        .step_summaries
                        .push(event.sanitized_payload.summary.clone());
                }
            }
            ApplicationExecutionEventType::ExecutionCompleted => {
                state.lifecycle_state = ApplicationExecutionLifecycleState::Completed;
                state.terminal_result = Some(event.sanitized_payload.clone());
                state.active_control_ids.clear();
                state.pending_approval_refs.clear();
            }
            ApplicationExecutionEventType::ExecutionFailed => {
                state.lifecycle_state = ApplicationExecutionLifecycleState::Failed;
                state.terminal_error = serde_json::from_value(
                    event.sanitized_payload.data.clone().unwrap_or_default(),
                )
                .ok();
                state.active_control_ids.clear();
                state.pending_approval_refs.clear();
            }
            ApplicationExecutionEventType::ExecutionCancelled => {
                state.lifecycle_state = ApplicationExecutionLifecycleState::Cancelled;
                state.active_control_ids.clear();
                state.pending_approval_refs.clear();
            }
        }
    }

    state
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

fn payload_string(payload: &ApplicationExecutionPayload, key: &str) -> Option<String> {
    payload
        .data
        .as_ref()
        .and_then(|value| value.get(key))
        .and_then(|value| value.as_str())
        .map(str::to_string)
}
