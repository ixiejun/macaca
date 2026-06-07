//! Chat orchestration: SSE streaming, agentic loop, workflow execution,
//! and process lifecycle (start/stop).
//!
//! Facade module following the Facade + Adapter pattern used by
//! `loop_manager/` and `framework_runner/`. Route handlers stay thin;
//! each adapter owns one integration boundary (SSE, executor events,
//! application service, WASM dispatch, session persistence).

mod agent_activity_adapter;
mod agent_execution_adapter;
mod application_service_adapter;
mod dto;
mod executor_adapter;
mod executor_event_adapter;
mod framework_chat_path;
mod route_chat_v2;
mod route_legacy;
mod route_stop;
mod session_persistence_adapter;
mod sse_adapter;
mod wasm_chat_path;
mod wasm_dispatch_adapter;

#[cfg(test)]
pub(crate) mod contract_source;
#[cfg(test)]
mod tests;

pub(crate) use route_chat_v2::post_chat_v2;
pub(crate) use route_legacy::post_chat;
pub(crate) use route_stop::post_chat_stop;
