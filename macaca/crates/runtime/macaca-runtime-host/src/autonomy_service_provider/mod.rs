//! Runtime-host adapters for Scheduler and Heartbeat autonomy services.
//!
//! This module is the approved composition-root bridge for autonomy services.
//! It wraps service-contract providers behind the kernel-visible
//! [`macaca_kernel::SystemService`] boundary and registers fail-closed
//! unavailable providers when no concrete scheduler or heartbeat implementation
//! is installed.  The adapter never parses cron expressions, evaluates
//! application workflows, or branches on provider/application/business names.
//!
//! **Pattern:** Facade module tree — per-service Adapter modules decode typed
//! commands, `bootstrap` owns Abstract Factory registration, and `support`
//! centralizes trace/decode/encode helpers shared across adapters.
//!
//! Module layout:
//! - `support.rs` — trace extraction, payload decode, result shaping
//! - `scheduler_adapter.rs` — `HostSchedulerServiceAdapter` Command Router
//! - `scheduled_agent_task_adapter.rs` — scheduled task intent Bridge
//! - `heartbeat_adapter.rs` — `HostHeartbeatServiceAdapter` Command Router
//! - `bootstrap.rs` — `AutonomyRuntimeBundle` + composition-root bootstrap

mod bootstrap;
mod heartbeat_adapter;
mod scheduled_agent_task_adapter;
mod scheduler_adapter;
mod support;

pub use bootstrap::{
    bootstrap_autonomy_local_services, bootstrap_autonomy_services,
    bootstrap_autonomy_unavailable_services, AutonomyRuntimeBundle,
};
pub use heartbeat_adapter::HostHeartbeatServiceAdapter;
pub use scheduled_agent_task_adapter::ScheduledAgentTaskSystemServiceProvider;
pub use scheduler_adapter::HostSchedulerServiceAdapter;
