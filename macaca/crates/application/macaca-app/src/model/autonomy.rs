//! Application-owned autonomy and heartbeat declaration types.
//!
//! These blocks are **declarative only**: they describe which generic autonomy
//! surfaces an application opts into. Runtime-host and system services still own
//! policy, trace, scheduling, heartbeat evaluation, and execution semantics.

use serde::{Deserialize, Serialize};

/// Default enable flag for heartbeat blocks and per-agent switches.
pub(super) fn default_true() -> bool {
    true
}

/// Provider-neutral heartbeat profile selector used when manifests omit `profile_id`.
pub(super) fn default_profile_id() -> String {
    "default".into()
}

/// Application-owned autonomy declarations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppAutonomyConfig {
    /// Optional heartbeat agent participation contract.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heartbeat: Option<AppHeartbeatConfig>,
}

/// Application-owned heartbeat participation contract.
///
/// The presence of this block does not execute anything by itself. Runtime-host
/// must query the sanitized Application Service projection and then dispatch
/// through Agent Execution, so app packages cannot bypass policy or service
/// audit boundaries.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppHeartbeatConfig {
    /// Disable all heartbeat agent dispatch while preserving declarations.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Manifest-declared agents that may receive heartbeat execution intent.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub agents: Vec<AppHeartbeatAgentConfig>,
}

/// One manifest-declared heartbeat agent.
///
/// `profile_id` is a provider-neutral extension point. The first runtime bridge
/// records it for traceability without adding application-specific profile
/// behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppHeartbeatAgentConfig {
    /// Agent name as declared in this application's manifest.
    pub name: String,
    /// Per-agent enable switch for staged rollout and package defaults.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Provider-neutral profile selector recorded in sanitized dispatch views.
    #[serde(default = "default_profile_id")]
    pub profile_id: String,
    /// Optional per-agent cadence policy.
    ///
    /// This policy is declarative only. Runtime-host translates it into a
    /// Heartbeat-owned native profile, while service.heartbeat remains the
    /// owner of cadence evaluation, gate decisions, and run mementos.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cadence: Option<AppHeartbeatCadenceConfig>,
    /// Optional per-agent gate policy.
    ///
    /// The first gate exposed to manifests is cooldown because it determines
    /// how often an accepted wake may run after the fixed cadence becomes due.
    /// Missing values preserve the provider default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gates: Option<AppHeartbeatGateConfig>,
    /// Bounded declaration metadata. Projection code sanitizes values before
    /// they cross service boundaries.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub metadata: std::collections::BTreeMap<String, String>,
}

/// Per-agent heartbeat cadence policy declared by an application.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppHeartbeatCadenceConfig {
    /// Fixed interval in seconds for this agent's native Heartbeat profile.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixed_interval_secs: Option<u64>,
}

/// Per-agent heartbeat gate policy declared by an application.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppHeartbeatGateConfig {
    /// Optional cooldown in seconds after an accepted wake for this agent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cooldown_secs: Option<u64>,
}
