//! Configuration for explicitly activated local autonomy runtime.
//!
//! The types in this module are deliberately provider-neutral.  Runtime-host
//! uses them to decide whether to keep fail-closed unavailable providers or to
//! activate local Scheduler, Heartbeat, and supervisor wiring.  The
//! configuration never contains application, workflow, model, driver, gateway,
//! chain, payment, or business-domain names, so enabling autonomy cannot become
//! a hidden product-specific branch.

/// Provider mode selected by the runtime-host autonomy composition root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutonomyProviderMode {
    /// Register Null Object providers and do not start background autonomy.
    Unavailable,
    /// Register built-in local providers and optionally start the supervisor.
    Local,
}

impl Default for AutonomyProviderMode {
    fn default() -> Self {
        Self::Unavailable
    }
}

/// Provider-neutral local autonomy runtime controls.
///
/// Values are expressed as plain milliseconds and bounded counters so they can
/// later be loaded from TOML, environment, package policy, or remote control
/// planes without changing public service contracts.  Defaults are fail-closed:
/// unavailable provider mode and no supervisor side effects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutonomyRuntimeConfig {
    pub provider_mode: AutonomyProviderMode,
    pub supervisor_enabled: bool,
    pub scheduler_tick_interval_ms: u64,
    pub heartbeat_tick_interval_ms: u64,
    pub max_leases_per_tick: usize,
    pub dispatch_timeout_ms: u64,
    pub shutdown_grace_ms: u64,
    pub recovery_wake_enabled: bool,
    pub safe_retention_limit: usize,
}

impl AutonomyRuntimeConfig {
    /// Build explicit local mode while keeping conservative bounded defaults.
    pub fn local_enabled() -> Self {
        Self {
            provider_mode: AutonomyProviderMode::Local,
            supervisor_enabled: true,
            scheduler_tick_interval_ms: 60_000,
            heartbeat_tick_interval_ms: 60_000,
            max_leases_per_tick: 8,
            dispatch_timeout_ms: 30_000,
            shutdown_grace_ms: 5_000,
            recovery_wake_enabled: true,
            safe_retention_limit: 128,
        }
    }

    /// Build local mode without a spawned loop for deterministic tests.
    ///
    /// Manual ticks still exercise the same supervisor implementation, but no
    /// background task races the test body.  Production callers should use
    /// [`Self::local_enabled`] or set `supervisor_enabled` explicitly.
    pub fn local_manual() -> Self {
        Self {
            supervisor_enabled: false,
            ..Self::local_enabled()
        }
    }

    /// Return a sanitized mode label for logs and snapshots.
    pub fn mode_label(&self) -> &'static str {
        match self.provider_mode {
            AutonomyProviderMode::Unavailable => "unavailable",
            AutonomyProviderMode::Local => "local",
        }
    }

    /// Normalize bounds so runtime-host never starts an unbounded loop.
    pub fn normalized(mut self) -> Self {
        self.scheduler_tick_interval_ms = self.scheduler_tick_interval_ms.max(1_000);
        self.heartbeat_tick_interval_ms = self.heartbeat_tick_interval_ms.max(1_000);
        self.max_leases_per_tick = self.max_leases_per_tick.clamp(1, 128);
        self.dispatch_timeout_ms = self.dispatch_timeout_ms.max(1);
        self.shutdown_grace_ms = self.shutdown_grace_ms.max(1);
        self.safe_retention_limit = self.safe_retention_limit.clamp(1, 10_000);
        self
    }
}

impl Default for AutonomyRuntimeConfig {
    fn default() -> Self {
        Self {
            provider_mode: AutonomyProviderMode::Unavailable,
            supervisor_enabled: false,
            scheduler_tick_interval_ms: 60_000,
            heartbeat_tick_interval_ms: 60_000,
            max_leases_per_tick: 8,
            dispatch_timeout_ms: 30_000,
            shutdown_grace_ms: 5_000,
            recovery_wake_enabled: false,
            safe_retention_limit: 128,
        }
    }
}
