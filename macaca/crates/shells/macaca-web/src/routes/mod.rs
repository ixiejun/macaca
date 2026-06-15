//! API route handlers for the Agent OS web server (thin Facade module).
//!
//! This module follows the Facade + Adapter pattern used by [`crate::chat_orchestrator`]
//! and [`crate::loop_manager`]. Each submodule owns one HTTP surface area (status,
//! applications, skills/MCP, todos, schedules, session inspection, drivers, context
//! runtime) while `mod.rs` re-exports the stable public API consumed by
//! `bootstrap.rs` and sibling shell adapters.
//!
//! Heavy-lifting modules:
//!
//! - [`crate::chat_orchestrator`] — SSE streaming, agentic loop, workflow
//! - [`crate::session`] — session CRUD, persistence, EventLog reconstruction
//! - [`crate::loop_manager`] — PlanLoop / WorkerLoop lifecycle
//! - [`crate::sse`] — SSE event conversion, broadcasting, plan decisions

mod apps;
mod apps_agents;
mod apps_reload;
mod autonomy_sanitizers;
mod autonomy_schedules;
mod context_runtime;
mod drivers;
mod session_inspect;
mod shared;
mod skills_mcp;
mod status;
mod todos;

#[cfg(test)]
pub(crate) mod contract_source;
#[cfg(test)]
mod tests;

// Shared utilities and error helpers
pub(crate) use shared::{app_entry_agent_name, default_model, err, proto_err};
pub use shared::{root_not_found, AgentStatusQuery, ErrorResponse};

// System status
pub use status::{get_status, StatusResponse};

// Application routes
pub use apps::{get_app, get_apps, AppInfo};
pub use apps_agents::{
    get_app_agents, stream_agent_status, AgentActivityInfo, AgentInfo, SimpleAgentStatus,
};
pub use apps_reload::{reload_apps, ReloadResponse};

// Skills / MCP
pub use skills_mcp::{
    get_app_skills, get_mcp_status, get_skills, AppFilteredSkillInfo, AppSkillInfo, AppSkillStatus,
    SkillInfo, SkillStatusQuery,
};

// Todo / goal board
pub use todos::{
    get_todo_claim_diagnostics, get_todo_progress, list_agent_todos, list_goals, list_todos,
    SessionQuery,
};

// Serviceized autonomy + scheduled agent tasks
pub use autonomy_sanitizers::AutonomyScheduleTargetRequest;
pub use autonomy_schedules::{
    cancel_scheduled_agent_task, create_autonomy_schedule, create_scheduled_agent_task,
    delete_autonomy_schedule, get_autonomy_schedule, get_scheduled_agent_task,
    list_autonomy_scheduler_runs, list_autonomy_schedules, list_scheduled_agent_tasks,
    transition_autonomy_schedule, update_autonomy_schedule, AutonomyRunsQuery,
    AutonomyScheduleLifecycleRequest, AutonomySchedulesQuery, CreateAutonomyScheduleRequest,
    CreateScheduledAgentTaskRequest, UpdateAutonomyScheduleRequest,
};

// Session inspection
pub use session_inspect::{
    get_session_context_reports, get_session_events, get_session_lineage, get_session_run_trace,
    get_session_source_artifact, manual_compact_session, EventsQuery, EventsResponse,
    ManualCompactRequest, ManualCompactResponse, SessionLineageResponse,
};

// Drivers
pub use drivers::{
    get_drivers, reload_drivers, DriverInfo, DriverReloadResponse, DriverReloadResult,
    DriversResponse,
};

// Context provider runtime operator snapshot
pub use context_runtime::get_context_provider_runtime;
