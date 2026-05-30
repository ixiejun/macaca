//! EventLog adapter for the application execution protocol.
//!
//! Macaca already has a durable append-only session EventLog.  This adapter
//! reuses that infrastructure instead of introducing a parallel store.  The
//! service writes one sanitized `ApplicationExecutionEventEnvelope` per EventLog
//! row, assigns the final sequence, and reconstructs replay/current-state views
//! from the persisted rows.

use std::sync::Arc;

use macaca_persist::{AppendEventCommand, EventLog};
use macaca_proto::{
    ApplicationExecutionCurrentState, ApplicationExecutionEventEnvelope,
    ApplicationExecutionReplayRequest, ApplicationExecutionReplayResult, ApplicationExecutionScope,
    ServiceError,
};
use tracing::{info, warn};

use crate::application_execution_projection::project_application_execution_state;

const EVENT_SOURCE: &str = "service.application_execution";
const MAX_INLINE_PAYLOAD_BYTES: usize = 32 * 1024;

/// EventLog-backed repository for application execution facts.
#[derive(Clone)]
pub struct ApplicationExecutionEventStore {
    event_log: Arc<EventLog>,
    max_inline_payload_bytes: usize,
}

impl ApplicationExecutionEventStore {
    /// Build a repository around the shared host EventLog.
    ///
    /// The EventLog remains the durable owner.  This type is a small Adapter
    /// that validates protocol-specific invariants before delegating append and
    /// replay to the existing persistence service.
    pub fn new(event_log: Arc<EventLog>) -> Self {
        Self {
            event_log,
            max_inline_payload_bytes: MAX_INLINE_PAYLOAD_BYTES,
        }
    }

    /// Append one protocol event idempotently and return the persisted envelope.
    ///
    /// Idempotency is scoped by application/session/run/idempotency key.  When a
    /// duplicate is observed, the previously persisted row is returned without a
    /// second side effect.  This lets external backends and remote agents retry
    /// callbacks safely after network loss.
    pub async fn append_idempotent(
        &self,
        mut event: ApplicationExecutionEventEnvelope,
    ) -> Result<ApplicationExecutionEventEnvelope, ServiceError> {
        self.validate_event(&event)?;
        if let Some(existing) = self.find_duplicate(&event).await {
            info!(
                application_id = %event.application_id,
                session_id = %event.session_id,
                run_id = %event.run_id,
                idempotency_key = %event.idempotency_key,
                "application execution event append deduplicated"
            );
            return Ok(existing);
        }

        let event_type = format!("{:?}", event.event_type);
        let app_id = event.application_id.to_string();
        let payload = serde_json::to_value(&event).map_err(adapter_error)?;
        let mut command =
            AppendEventCommand::new(&event.session_id, event_type, EVENT_SOURCE, payload);
        command = command.with_app_id(app_id);
        let seq = self.event_log.append_command(command).await;
        event.seq = Some(seq);

        // EventLog assigns the sequence after payload serialization.  Append a
        // second immutable protocol row is unacceptable, so replay conversion
        // treats EventLog.seq as authoritative even if the stored envelope had
        // `seq = None`.  Returning the assigned value here gives callers the
        // immediate cursor they need for replay/control references.
        info!(
            application_id = %event.application_id,
            session_id = %event.session_id,
            run_id = %event.run_id,
            seq,
            trace_id = %event.trace.trace_id,
            "application execution event appended to EventLog"
        );
        Ok(event)
    }

    /// Replay persisted protocol events and return the deterministic projection.
    pub async fn replay(
        &self,
        request: ApplicationExecutionReplayRequest,
    ) -> Result<ApplicationExecutionReplayResult, ServiceError> {
        let since_seq = request
            .from_cursor
            .as_deref()
            .and_then(parse_cursor)
            .unwrap_or(0);
        let page_size = request.page_size.clamp(1, 500);
        let mut events = Vec::new();
        for row in self
            .event_log
            .query(&request.session_id, since_seq, page_size)
            .await
        {
            let Some(event) = self.row_to_event(row.seq, row.payload) else {
                continue;
            };
            if event.application_id != request.application_id {
                continue;
            }
            if let Some(run_id) = &request.run_id {
                if &event.run_id != run_id {
                    continue;
                }
            }
            if let Some(visibility) = &request.visibility {
                if &event.visibility != visibility {
                    continue;
                }
            }
            if !request.event_types.is_empty()
                && !request
                    .event_types
                    .iter()
                    .any(|kind| kind == &event.event_type)
            {
                continue;
            }
            events.push(event);
        }
        let next_cursor = events.last().and_then(|event| event.seq).map(cursor);
        let current_state = events.first().map(|first| {
            let scope = ApplicationExecutionScope {
                application_id: first.application_id,
                session_id: first.session_id.clone(),
                run_id: first.run_id.clone(),
                tenant_id: None,
                actor: first.actor.clone(),
            };
            project_application_execution_state(scope, &events, next_cursor.clone())
        });
        Ok(ApplicationExecutionReplayResult {
            events,
            next_cursor,
            current_state,
        })
    }

    /// Return the latest current-state projection for one scope.
    pub async fn current_state(
        &self,
        scope: ApplicationExecutionScope,
    ) -> Result<ApplicationExecutionCurrentState, ServiceError> {
        let request = ApplicationExecutionReplayRequest {
            application_id: scope.application_id,
            session_id: scope.session_id.clone(),
            run_id: Some(scope.run_id.clone()),
            from_cursor: None,
            page_size: 500,
            event_types: Vec::new(),
            visibility: None,
            trace: macaca_proto::TraceContext::new("application-execution-current-state"),
        };
        let replay = self.replay(request).await?;
        Ok(project_application_execution_state(
            scope,
            &replay.events,
            replay.next_cursor,
        ))
    }

    fn validate_event(
        &self,
        event: &ApplicationExecutionEventEnvelope,
    ) -> Result<(), ServiceError> {
        if event.session_id.trim().is_empty()
            || event.run_id.trim().is_empty()
            || event.actor.trim().is_empty()
            || event.provider_id.trim().is_empty()
            || event.idempotency_key.trim().is_empty()
            || event.schema_version.trim().is_empty()
        {
            return Err(ServiceError::InvalidArgument(
                "application execution event is missing required scope fields".into(),
            ));
        }
        let payload_size = serde_json::to_vec(&event.sanitized_payload)
            .map_err(adapter_error)?
            .len();
        if payload_size > self.max_inline_payload_bytes && event.payload_ref.is_none() {
            return Err(ServiceError::InvalidArgument(
                "application execution event payload exceeds inline limit without payload_ref"
                    .into(),
            ));
        }
        Ok(())
    }

    async fn find_duplicate(
        &self,
        event: &ApplicationExecutionEventEnvelope,
    ) -> Option<ApplicationExecutionEventEnvelope> {
        let latest = self.event_log.latest_seq(&event.session_id).await;
        for row in self
            .event_log
            .query(&event.session_id, 0, latest as usize + 1)
            .await
        {
            let candidate = self.row_to_event(row.seq, row.payload)?;
            if candidate.application_id == event.application_id
                && candidate.run_id == event.run_id
                && candidate.idempotency_key == event.idempotency_key
            {
                return Some(candidate);
            }
        }
        None
    }

    fn row_to_event(
        &self,
        seq: u64,
        payload: serde_json::Value,
    ) -> Option<ApplicationExecutionEventEnvelope> {
        let mut event: ApplicationExecutionEventEnvelope = match serde_json::from_value(payload) {
            Ok(event) => event,
            Err(error) => {
                warn!(
                    seq,
                    error = %error,
                    "skipping non application execution EventLog row during replay"
                );
                return None;
            }
        };
        event.seq = Some(seq);
        Some(event)
    }
}

fn parse_cursor(cursor: &str) -> Option<u64> {
    cursor.strip_prefix("event/")?.parse().ok()
}

fn cursor(seq: u64) -> String {
    format!("event/{seq}")
}

fn adapter_error(error: serde_json::Error) -> ServiceError {
    ServiceError::AdapterFailure(error.to_string())
}
