//! Host composition facade for process bootstrap surfaces.
//!
//! This crate is the approved composition-root boundary for host-owned
//! providers, runtime handles, and application/framework bootstrap contracts.
//! SDK clients stay protocol-backed; process shells that still assemble the
//! local host import these contracts from here instead of from `macaca-sdk`.

extern crate self as macaca_host_composition;

#[path = "../../../shells/macaca-web/src/agent_context_backend.rs"]
mod agent_context_backend;
#[path = "../../../shells/macaca-web/src/agent_execution_backend/mod.rs"]
mod agent_execution_backend;
#[path = "../../../shells/macaca-web/src/agent_execution_evidence.rs"]
mod agent_execution_evidence;
#[path = "../../../shells/macaca-web/src/agent_runner.rs"]
pub mod agent_runner;
#[path = "../../../shells/macaca-web/src/app_skill_status_facade.rs"]
mod app_skill_status_facade;
#[path = "../../../shells/macaca-web/src/app_ui_csp.rs"]
mod app_ui_csp;
#[path = "../../../shells/macaca-web/src/app_ui_llm_bridge.rs"]
mod app_ui_llm_bridge;
#[path = "../../../shells/macaca-web/src/app_ui_routes/mod.rs"]
pub mod app_ui_routes;
#[path = "../../../shells/macaca-web/src/app_ui_session_projection.rs"]
mod app_ui_session_projection;
#[path = "../../../shells/macaca-web/src/app_ui_workspace_scope.rs"]
mod app_ui_workspace_scope;
#[path = "../../../shells/macaca-web/src/app_workspace_bootstrap.rs"]
mod app_workspace_bootstrap;
#[path = "../../../shells/macaca-web/src/application_execution_agent_event_bridge.rs"]
mod application_execution_agent_event_bridge;
#[path = "../../../shells/macaca-web/src/application_execution_agent_event_display.rs"]
mod application_execution_agent_event_display;
#[path = "../../../shells/macaca-web/src/application_execution_event_log.rs"]
mod application_execution_event_log;
#[path = "../../../shells/macaca-web/src/application_execution_gateway_routes.rs"]
pub mod application_execution_gateway_routes;
#[cfg(test)]
#[path = "../../../shells/macaca-web/src/application_execution_gateway_routes_tests.rs"]
mod application_execution_gateway_routes_tests;
#[cfg(test)]
#[path = "../../../shells/macaca-web/src/application_execution_route_validation_tests.rs"]
mod application_execution_route_validation_tests;
#[path = "../../../shells/macaca-web/src/application_execution_routes.rs"]
pub mod application_execution_routes;
#[cfg(test)]
#[path = "../../../shells/macaca-web/src/application_execution_routes_tests.rs"]
mod application_execution_routes_tests;
#[path = "../../../shells/macaca-web/src/application_execution_stream_routes.rs"]
pub mod application_execution_stream_routes;
#[path = "../../../shells/macaca-web/src/application_shell_adapter.rs"]
mod application_shell_adapter;
#[path = "../../../shells/macaca-web/src/bootstrap.rs"]
pub mod bootstrap;
#[path = "../../../shells/macaca-web/src/capability_catalog.rs"]
mod capability_catalog;
#[path = "../../../shells/macaca-web/src/chat_mediator.rs"]
pub mod chat_mediator;
#[path = "../../../shells/macaca-web/src/chat_orchestrator/mod.rs"]
pub mod chat_orchestrator;
#[path = "../../../shells/macaca-web/src/composition_bootstrap/mod.rs"]
mod composition_bootstrap;
#[path = "../../../shells/macaca-web/src/context_memory_injection/mod.rs"]
mod context_memory_injection;
#[path = "../../../shells/macaca-web/src/context_memory_tools.rs"]
mod context_memory_tools;
#[path = "../../../shells/macaca-web/src/context_message_codec.rs"]
mod context_message_codec;
#[path = "../../../shells/macaca-web/src/context_report_events.rs"]
mod context_report_events;
#[path = "../../../shells/macaca-web/src/context_reporting_memory.rs"]
mod context_reporting_memory;
#[path = "../../../shells/macaca-web/src/context_reporting_model/mod.rs"]
mod context_reporting_model;
#[path = "../../../shells/macaca-web/src/context_runtime_facade.rs"]
pub(crate) mod context_runtime_facade;
#[path = "../../../shells/macaca-web/src/event_persistence.rs"]
pub mod event_persistence;
#[path = "../../../shells/macaca-web/src/external_context_adapter.rs"]
mod external_context_adapter;
#[path = "../../../shells/macaca-web/src/fork_join_shell_adapter.rs"]
mod fork_join_shell_adapter;
#[path = "../../../shells/macaca-web/src/framework_adapter.rs"]
mod framework_adapter;
#[path = "../../../shells/macaca-web/src/framework_agent_construction_shell_adapter.rs"]
mod framework_agent_construction_shell_adapter;
#[path = "../../../shells/macaca-web/src/framework_runner/mod.rs"]
pub mod framework_runner;
#[path = "../../../shells/macaca-web/src/framework_state_memento.rs"]
mod framework_state_memento;
#[path = "../../../shells/macaca-web/src/framework_toolkit/mod.rs"]
pub mod framework_toolkit;
#[path = "../../../shells/macaca-web/src/genui_routes.rs"]
pub mod genui_routes;
#[path = "../../../shells/macaca-web/src/goal_lifecycle_shell_adapter.rs"]
mod goal_lifecycle_shell_adapter;
#[path = "../../../shells/macaca-web/src/heartbeat_operations_routes.rs"]
pub mod heartbeat_operations_routes;
#[path = "../../../shells/macaca-web/src/hook_consumer.rs"]
pub mod hook_consumer;
#[path = "../../../shells/macaca-web/src/llm_route_shell_adapter.rs"]
mod llm_route_shell_adapter;
#[path = "../../../shells/macaca-web/src/loop_manager/mod.rs"]
pub mod loop_manager;
#[path = "../../../shells/macaca-web/src/mcp_shell_adapter.rs"]
mod mcp_shell_adapter;
#[path = "../../../shells/macaca-web/src/metrics.rs"]
pub mod metrics;
#[path = "../../../shells/macaca-web/src/orchestration_tools.rs"]
pub mod orchestration_tools;
#[path = "../../../shells/macaca-web/src/plugin_routes.rs"]
pub mod plugin_routes;
#[path = "../../../shells/macaca-web/src/proto_event_visitors.rs"]
pub mod proto_event_visitors;
#[path = "../../../shells/macaca-web/src/route_command.rs"]
pub mod route_command;
#[path = "../../../shells/macaca-web/src/routes/mod.rs"]
pub mod routes;
#[path = "../../../shells/macaca-web/src/run_trace.rs"]
pub mod run_trace;
#[path = "../../../shells/macaca-web/src/runtime_event_bridge.rs"]
pub mod runtime_event_bridge;
#[path = "../../../shells/macaca-web/src/scheduled_agent_task_tool.rs"]
mod scheduled_agent_task_tool;
#[path = "../../../shells/macaca-web/src/service_tool_adapter.rs"]
mod service_tool_adapter;
#[path = "../../../shells/macaca-web/src/session/mod.rs"]
pub mod session;
#[path = "../../../shells/macaca-web/src/session_inspection_facade.rs"]
mod session_inspection_facade;
#[path = "../../../shells/macaca-web/src/session_loop_shell_adapter.rs"]
mod session_loop_shell_adapter;
#[path = "../../../shells/macaca-web/src/session_memory_capture.rs"]
mod session_memory_capture;
#[path = "../../../shells/macaca-web/src/session_replay.rs"]
pub mod session_replay;
#[path = "../../../shells/macaca-web/src/shell.rs"]
pub mod shell;
#[path = "../../../shells/macaca-web/src/shell_composition_bundle.rs"]
mod shell_composition_bundle;
#[path = "../../../shells/macaca-web/src/skill_mcp/mod.rs"]
pub mod skill_mcp;
#[path = "../../../shells/macaca-web/src/skill_operations_routes/mod.rs"]
pub mod skill_operations_routes;
#[path = "../../../shells/macaca-web/src/skill_self_evolution_audit.rs"]
pub mod skill_self_evolution_audit;
#[path = "../../../shells/macaca-web/src/skill_self_evolution_execution_observer.rs"]
mod skill_self_evolution_execution_observer;
#[path = "../../../shells/macaca-web/src/skill_self_evolution_observer/mod.rs"]
pub mod skill_self_evolution_observer;
#[path = "../../../shells/macaca-web/src/skill_usage_telemetry.rs"]
mod skill_usage_telemetry;
#[path = "../../../shells/macaca-web/src/source_artifact.rs"]
mod source_artifact;
#[path = "../../../shells/macaca-web/src/sse.rs"]
pub mod sse;
#[path = "../../../shells/macaca-web/src/state.rs"]
pub mod state;
#[path = "../../../shells/macaca-web/src/system_status_adapter.rs"]
mod system_status_adapter;
#[path = "../../../shells/macaca-web/src/tool_routes.rs"]
pub mod tool_routes;
#[path = "../../../shells/macaca-web/src/trace_events.rs"]
pub mod trace_events;
#[cfg(test)]
#[path = "../../../shells/macaca-web/src/unified_agent_execution_provider_tests.rs"]
mod unified_agent_execution_provider_tests;
#[cfg(test)]
#[path = "../../../shells/macaca-web/src/unified_audit_replay_convergence_tests.rs"]
mod unified_audit_replay_convergence_tests;
#[cfg(test)]
#[path = "../../../shells/macaca-web/src/unified_delegation_path_tests.rs"]
mod unified_delegation_path_tests;
#[cfg(test)]
#[path = "../../../shells/macaca-web/src/unified_workflow_application_abi_tests.rs"]
mod unified_workflow_application_abi_tests;
#[path = "../../../shells/macaca-web/src/wasm_orchestration_backend.rs"]
mod wasm_orchestration_backend;
#[path = "../../../shells/macaca-web/src/web3_status.rs"]
pub mod web3_status;
#[path = "../../../shells/macaca-web/src/web_agent_execution_adapters.rs"]
mod web_agent_execution_adapters;
#[path = "../../../shells/macaca-web/src/workbench_routes.rs"]
pub mod workbench_routes;
#[path = "../../../shells/macaca-web/src/workspace.rs"]
pub mod workspace;
#[path = "../../../shells/macaca-web/src/workspace_knowledge_digest_capability.rs"]
mod workspace_knowledge_digest_capability;
#[path = "../../../shells/macaca-web/src/workspace_memory_recall_source.rs"]
mod workspace_memory_recall_source;

pub mod application_agent_delegate_bridge;
pub mod context_client;
pub mod optional_service_bootstrap;
pub mod persistence_adapter;
pub mod runtime_host;
pub mod skill_client;
pub mod skill_operator_client;
pub mod system_service_client;
pub mod workbench_service_bootstrap;

mod focused_runtime_surfaces;

pub use crate::bootstrap::{WebRuntimeFacade, WebServerBuilder};
pub use application_agent_delegate_bridge::{
    build_execution_command_from_delegate, delegate_result_from_execution_reply,
    dispatch_agent_execution_via_service, queued_delegate_result, DEFAULT_AGENT_DELEGATE_WAIT_MS,
    ORCHESTRATION_AGENT_EXECUTION_SOURCE,
};
pub use context_client::{
    ServiceBackedContextClient, SystemContextClient, UnavailableSystemContextClient,
};
pub use focused_runtime_surfaces::{
    agent_execution, app, application_bootstrap, autonomy_runtime, context, execution_control,
    executor, framework, kernel, llm, mcp_runtime, memory, persist, service_bootstrap,
    service_runtime, task, tool_bootstrap, tools,
};
pub use optional_service_bootstrap::{
    bootstrap_host_optional_services, HostOptionalServiceBootstrapReport,
};
pub use persistence_adapter::RedbKernelPersistenceAdapter;
pub use skill_client::{ServiceBackedSkillClient, SystemSkillClient, UnavailableSystemSkillClient};
pub use skill_operator_client::SystemSkillOperatorClient;
pub use system_service_client::HostRuntimeSystemServiceClient;
pub use workbench_service_bootstrap::{
    bootstrap_workbench_family_services, WorkbenchServiceBootstrapReport,
};
