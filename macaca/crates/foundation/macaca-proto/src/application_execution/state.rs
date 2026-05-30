//! Replay, snapshot, and current-state projection DTOs.
//!
//! Application execution state is a Memento-style projection from durable
//! events.  Frontends may render these DTOs, but they must not become the
//! authority for execution progress.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{ApplicationId, TraceContext};

use super::{
    ApplicationExecutionError, ApplicationExecutionEventEnvelope, ApplicationExecutionEventType,
    ApplicationExecutionLifecycleState, ApplicationExecutionPayload,
    ApplicationExecutionProviderKind, ApplicationExecutionScope,
};

/// Point-in-time provider/session snapshot for diagnostics and recovery.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApplicationExecutionSnapshot {
    pub scope: ApplicationExecutionScope,
    pub lifecycle_state: ApplicationExecutionLifecycleState,
    pub provider_id: Option<String>,
    pub provider_kind: ApplicationExecutionProviderKind,
    pub latest_event_cursor: Option<String>,
    pub latest_checkpoint_ref: Option<String>,
    pub metadata: BTreeMap<String, String>,
}

/// Current-state projection derived from persisted events.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApplicationExecutionCurrentState {
    pub scope: ApplicationExecutionScope,
    pub lifecycle_state: ApplicationExecutionLifecycleState,
    pub provider_id: Option<String>,
    pub provider_kind: ApplicationExecutionProviderKind,
    pub latest_heartbeat_at: Option<DateTime<Utc>>,
    pub pending_approval_refs: Vec<String>,
    pub active_control_ids: Vec<String>,
    pub latest_checkpoint_ref: Option<String>,
    pub step_summaries: Vec<String>,
    pub terminal_result: Option<ApplicationExecutionPayload>,
    pub terminal_error: Option<ApplicationExecutionError>,
    pub replay_cursor: Option<String>,
}

/// Replay request for deterministic event recovery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplicationExecutionReplayRequest {
    pub application_id: ApplicationId,
    pub session_id: String,
    pub run_id: Option<String>,
    pub from_cursor: Option<String>,
    pub page_size: usize,
    pub event_types: Vec<ApplicationExecutionEventType>,
    pub visibility: Option<String>,
    pub trace: TraceContext,
}

/// Replay response containing persisted events and the next cursor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApplicationExecutionReplayResult {
    pub events: Vec<ApplicationExecutionEventEnvelope>,
    pub next_cursor: Option<String>,
    pub current_state: Option<ApplicationExecutionCurrentState>,
}
