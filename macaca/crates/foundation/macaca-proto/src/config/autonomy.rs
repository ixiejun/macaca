//! Host-level autonomy runtime activation knobs.
//!
//! Provider-neutral serde types consumed by `macaca-runtime-host` composition roots.
//! Defaults stay fail-closed so tests, SDK users, and embedded hosts do not
//! accidentally start long-running loops.

use serde::{Deserialize, Serialize};

/// Host-level autonomy runtime activation knobs.
///
/// This configuration intentionally lives as provider-neutral data in
/// `macaca-proto`. Concrete provider construction still belongs to
/// `macaca-runtime-host`; shells only translate these bounded values into the
/// runtime-host factory config at startup.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AutonomyConfig {
    /// Provider mode label understood by approved host composition roots.
    ///
    /// Supported stable labels are `unavailable` and `local`. Unknown labels
    /// must be mapped to unavailable by hosts instead of guessing a provider.
    #[serde(default = "default_autonomy_provider_mode")]
    pub provider_mode: String,
    /// Whether the host-owned supervisor loop should run continuously.
    #[serde(default)]
    pub supervisor_enabled: bool,
    /// Bounded interval for Scheduler due-run ticks.
    #[serde(default = "default_autonomy_scheduler_tick_interval_ms")]
    pub scheduler_tick_interval_ms: u64,
    /// Bounded interval for Heartbeat scheduled wake ticks.
    #[serde(default = "default_autonomy_heartbeat_tick_interval_ms")]
    pub heartbeat_tick_interval_ms: u64,
    /// Maximum Scheduler leases a supervisor may claim per tick.
    #[serde(default = "default_autonomy_max_leases_per_tick")]
    pub max_leases_per_tick: usize,
    /// Maximum time allowed for one dispatch strategy call.
    #[serde(default = "default_autonomy_dispatch_timeout_ms")]
    pub dispatch_timeout_ms: u64,
    /// Grace period used when stopping the supervisor loop.
    #[serde(default = "default_autonomy_shutdown_grace_ms")]
    pub shutdown_grace_ms: u64,
    /// Whether startup should emit a generic recovery wake through Heartbeat.
    #[serde(default)]
    pub recovery_wake_enabled: bool,
    /// Host-visible retention ceiling for safe snapshots.
    #[serde(default = "default_autonomy_safe_retention_limit")]
    pub safe_retention_limit: usize,
}

fn default_autonomy_provider_mode() -> String {
    "unavailable".into()
}

fn default_autonomy_scheduler_tick_interval_ms() -> u64 {
    60_000
}

fn default_autonomy_heartbeat_tick_interval_ms() -> u64 {
    60_000
}

fn default_autonomy_max_leases_per_tick() -> usize {
    8
}

fn default_autonomy_dispatch_timeout_ms() -> u64 {
    30_000
}

fn default_autonomy_shutdown_grace_ms() -> u64 {
    5_000
}

fn default_autonomy_safe_retention_limit() -> usize {
    128
}

impl Default for AutonomyConfig {
    fn default() -> Self {
        Self {
            provider_mode: default_autonomy_provider_mode(),
            supervisor_enabled: false,
            scheduler_tick_interval_ms: default_autonomy_scheduler_tick_interval_ms(),
            heartbeat_tick_interval_ms: default_autonomy_heartbeat_tick_interval_ms(),
            max_leases_per_tick: default_autonomy_max_leases_per_tick(),
            dispatch_timeout_ms: default_autonomy_dispatch_timeout_ms(),
            shutdown_grace_ms: default_autonomy_shutdown_grace_ms(),
            recovery_wake_enabled: false,
            safe_retention_limit: default_autonomy_safe_retention_limit(),
        }
    }
}
