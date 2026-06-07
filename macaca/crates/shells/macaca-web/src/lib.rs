//! `macaca-web` — Simple web UI for Macaca OS.
//!
//! Provides an HTTP server with a single-page web interface for interacting
//! with Macaca OS applications. Uses axum for the HTTP layer.

mod agent_context_backend;
mod agent_execution_backend;
mod agent_execution_evidence;
pub mod agent_runner;
mod app_ui_csp;
mod app_ui_llm_bridge;
pub mod app_ui_routes;
mod app_ui_session_projection;
mod app_ui_workspace_scope;
mod app_workspace_bootstrap;
mod application_shell_adapter;
mod application_agent_delegate_bridge;
mod application_execution_agent_event_bridge;
mod application_execution_agent_event_display;
pub mod application_execution_gateway_routes;
#[cfg(test)]
mod application_execution_gateway_routes_tests;
#[cfg(test)]
mod application_execution_route_validation_tests;
pub mod application_execution_routes;
#[cfg(test)]
mod application_execution_routes_tests;
pub mod application_execution_stream_routes;
pub mod bootstrap;
mod capability_catalog;
pub mod chat_mediator;
pub mod chat_orchestrator;
mod composition_bootstrap;
mod context_memory_injection;
mod context_memory_tools;
mod context_message_codec;
mod context_report_events;
mod context_reporting_memory;
mod context_reporting_model;
mod delegated_task_dispatcher;
pub mod event_persistence;
mod external_context_adapter;
pub mod framework_runner;
pub mod framework_toolkit;
pub mod genui_routes;
pub mod heartbeat_operations_routes;
mod fork_join_shell_adapter;
mod framework_agent_construction_shell_adapter;
mod goal_lifecycle_shell_adapter;
mod session_loop_shell_adapter;
mod llm_route_shell_adapter;
mod mcp_shell_adapter;
#[cfg(test)]
mod unified_delegation_path_tests;
#[cfg(test)]
mod unified_agent_execution_provider_tests;
#[cfg(test)]
mod unified_workflow_application_abi_tests;
#[cfg(test)]
mod unified_audit_replay_convergence_tests;
pub mod hook_consumer;
pub mod loop_manager;
mod memory_runtime;
pub mod metrics;
pub mod orchestration_tools;
mod persistence_adapter;
pub mod plugin_routes;
pub mod proto_event_visitors;
pub mod route_command;
pub mod routes;
pub mod run_trace;
pub mod runtime_event_bridge;
pub mod runtime_resume;
mod scheduled_agent_task_tool;
mod service_runtime_client;
mod service_tool_adapter;
pub mod session;
mod session_memory_capture;
pub mod session_replay;
pub mod shell;
mod shell_composition_bundle;
pub mod skill_mcp;
pub mod skill_operations_routes;
pub mod skill_self_evolution_audit;
mod skill_self_evolution_execution_observer;
pub mod skill_self_evolution_observer;
mod skill_usage_telemetry;
mod source_artifact;
pub mod sse;
pub mod state;
pub mod tool_routes;
pub mod trace_events;
mod wasm_orchestration_backend;
pub mod web3_status;
mod web_agent_execution_adapters;
pub mod workbench_routes;
pub mod workspace;
mod workspace_knowledge_digest_capability;
mod workspace_memory_recall_source;

use macaca_proto::MacacaResult;

pub use crate::bootstrap::{WebRuntimeFacade, WebServerBuilder};
pub(crate) use crate::composition_bootstrap::serve_web_server;

/// Start the Macaca OS web server.
#[deprecated(note = "Use WebServerBuilder::new().port(port).serve() instead")]
pub async fn start_server(port: u16) -> MacacaResult<()> {
    WebServerBuilder::new().port(port).serve().await
}
