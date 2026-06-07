//! Session management: types, CRUD handlers, persistence, and SSE streaming (Facade module).
//!
//! This module follows the Facade + Adapter pattern used by [`crate::routes`] and
//! [`crate::loop_manager`]. Submodules own one boundary each:
//!
//! - `types` — serde DTOs shared with chat orchestration and EventLog reconstruction
//! - `trace_mapping` — Visitor mapping from proto execution events to trace steps
//! - `trace_collector` — in-memory Observer for delegated-agent traces during SSE
//! - `turn_model` — turn list helpers (running assistant turn, executor status projection)
//! - `persistence` — redb snapshot writes with per-session locking
//! - `list_handlers` / `crud_handlers` / `sse_handlers` — thin Axum route adapters
//!
//! `bootstrap.rs` registers handlers via `session::` paths; external crates import only
//! the re-exported symbols below to keep the facade stable across file splits.

mod crud_handlers;
mod list_handlers;
mod persistence;
mod sse_handlers;
mod trace_collector;
mod trace_mapping;
mod turn_model;
mod types;

#[cfg(test)]
pub(crate) mod contract_source;
#[cfg(test)]
mod tests;

// Types and storage keys
pub(crate) use persistence::{
    load_agent_traces, persist_session_snapshot, save_agent_traces, update_session_realtime,
    AGENT_TRACES_PREFIX, APP_SESSIONS_PREFIX, SESSION_PREFIX,
};
pub(crate) use trace_collector::AgentTraceCollector;
pub(crate) use turn_model::{
    build_turns_from_messages, ensure_running_assistant_turn, session_status_from_executor_event,
    stored_turns_or_messages,
};
pub(crate) use types::{
    AgentTrace, AgentTraceStep, AssistantExecutionMeta, SessionMeta, StoredSession,
    StoredTraceStep, StoredTurn,
};

// HTTP handlers (registered from bootstrap)
pub(crate) use crud_handlers::{delete_session, get_session, get_session_by_id};
pub(crate) use list_handlers::{list_app_sessions, list_sessions};
pub(crate) use sse_handlers::stream_session_events;
