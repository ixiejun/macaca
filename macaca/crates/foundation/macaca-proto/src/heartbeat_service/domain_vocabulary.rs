//! Heartbeat profile, gate, and run-state vocabulary.
//!
//! **Pattern:** Value Object cluster — durable identity, cadence policy, gate decisions, and run
//! lifecycle enums stay separate from command envelopes so providers can evolve native profile
//! semantics without touching service-runtime transport DTOs.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    autonomy_service::non_empty,
    AutonomyScope, MacacaResult, ServiceCommandName,
};

/// Stable identity for a native heartbeat profile.
///
/// Profiles are the Heartbeat service's native cadence contract. They are not
/// Scheduler jobs and therefore can continue to tick even when Scheduler has no
/// due runs or is degraded.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct HeartbeatProfileId(String);

impl HeartbeatProfileId {
    /// Create a profile id from provider-assigned durable state.
    pub fn new(value: impl Into<String>) -> MacacaResult<Self> {
        Ok(Self(non_empty(
            value.into(),
            "heartbeat profile id is required",
        )?))
    }

    /// Return the raw identifier string for persistence adapters.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Stable identity for a heartbeat run.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct HeartbeatRunId(String);

impl HeartbeatRunId {
    /// Create a run id from provider-assigned durable state.
    pub fn new(value: impl Into<String>) -> MacacaResult<Self> {
        Ok(Self(non_empty(
            value.into(),
            "heartbeat run id is required",
        )?))
    }

    /// Return the raw identifier string for persistence adapters.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Generic wake intents accepted by the Heartbeat service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HeartbeatWakeIntent {
    ScheduledTick,
    NativeCadence { profile_id: HeartbeatProfileId },
    EventSignal { event_kind: String },
    Immediate,
    Manual,
    Recovery { reason_code: String },
    Extension { intent_id: String },
}

/// Provider-neutral identity for the scope that owns a heartbeat profile.
///
/// Scope identity is intentionally data, not control flow. Providers and
/// supervisors must not branch on application names, workflow names, agent role
/// names, provider names, model names, or business-domain strings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeartbeatScopeIdentity {
    pub scope: AutonomyScope,
    pub scope_key: String,
}

impl HeartbeatScopeIdentity {
    /// Build a typed heartbeat scope with a non-empty routing key.
    pub fn new(scope: AutonomyScope, scope_key: impl Into<String>) -> MacacaResult<Self> {
        Ok(Self {
            scope,
            scope_key: non_empty(scope_key.into(), "heartbeat scope key is required")?,
        })
    }
}

/// Native cadence policy for a Heartbeat profile.
///
/// The initial policy is deliberately simple and provider-neutral. More
/// advanced strategies can be added as new enum variants without making
/// Scheduler own heartbeat timing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HeartbeatCadencePolicy {
    FixedInterval {
        interval_ms: u64,
        anchor: Option<DateTime<Utc>>,
    },
}

impl HeartbeatCadencePolicy {
    /// Validate cadence bounds before a provider stores a profile.
    pub fn validate(&self) -> MacacaResult<()> {
        match self {
            Self::FixedInterval { interval_ms, .. } if *interval_ms > 0 => Ok(()),
            Self::FixedInterval { .. } => Err(crate::MacacaError::Config(
                "heartbeat interval must be positive".into(),
            )),
        }
    }

    /// Return the configured interval for providers that need eligibility math.
    pub fn interval_ms(&self) -> i64 {
        match self {
            Self::FixedInterval { interval_ms, .. } => (*interval_ms).min(i64::MAX as u64) as i64,
        }
    }
}

/// Safe declaration of a generic service action triggered by heartbeat.
///
/// Heartbeat records and dispatches action declarations only through service
/// boundaries. It never implements memory, task, execution, notification, or
/// application-specific behavior directly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeartbeatActionDeclaration {
    pub service_id: crate::KernelServiceId,
    pub command_name: ServiceCommandName,
    pub metadata: BTreeMap<String, String>,
}

/// Durable native heartbeat profile managed by `service.heartbeat`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeartbeatProfile {
    pub profile_id: HeartbeatProfileId,
    pub scope_identity: HeartbeatScopeIdentity,
    pub cadence: HeartbeatCadencePolicy,
    /// Optional profile-specific cooldown override.
    ///
    /// `None` means the provider Strategy uses its default cooldown. This keeps
    /// old profiles compatible while allowing per-agent profile policy to be
    /// edited without hiding cooldown in untyped metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cooldown_ms: Option<u64>,
    pub actions: Vec<HeartbeatActionDeclaration>,
    pub enabled: bool,
    pub metadata: BTreeMap<String, String>,
}

impl HeartbeatProfile {
    /// Build a native profile and validate its cadence before storage.
    pub fn new(
        profile_id: HeartbeatProfileId,
        scope_identity: HeartbeatScopeIdentity,
        cadence: HeartbeatCadencePolicy,
    ) -> MacacaResult<Self> {
        cadence.validate()?;
        Ok(Self {
            profile_id,
            scope_identity,
            cadence,
            cooldown_ms: None,
            actions: Vec::new(),
            enabled: true,
            metadata: BTreeMap::new(),
        })
    }
}

/// Bounded native profile summary returned in Heartbeat snapshots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeartbeatProfileSummary {
    pub profile_id: HeartbeatProfileId,
    pub scope_key: String,
    pub enabled: bool,
    /// Effective fixed interval for this profile in milliseconds.
    pub fixed_interval_ms: u64,
    /// Profile-specific cooldown override in milliseconds, if configured.
    pub cooldown_ms: Option<u64>,
    pub next_eligible_at: Option<DateTime<Utc>>,
    pub last_tick_at: Option<DateTime<Utc>>,
    pub last_run_id: Option<HeartbeatRunId>,
    pub safe_status: String,
    pub metadata: BTreeMap<String, String>,
}

/// Gate categories evaluated before heartbeat side effects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HeartbeatGateKind {
    ActiveHours,
    Cooldown,
    Busy,
    Resource,
    Budget,
    Policy,
    ProviderHealth,
    SchedulerActive,
    Extension { gate_id: String },
}

/// One safe gate decision captured for diagnostics and audit replay.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeartbeatGateDecision {
    pub gate: HeartbeatGateKind,
    pub allowed: bool,
    pub reason_code: String,
    pub next_eligible_at: Option<DateTime<Utc>>,
    pub metadata: BTreeMap<String, String>,
}

/// Explicit lifecycle state for heartbeat wake processing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HeartbeatRunState {
    Requested,
    Coalesced,
    Gated,
    Running,
    Succeeded,
    Failed,
    Skipped,
}

/// Result of accepting, coalescing, gating, or skipping a wake request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HeartbeatWakeDisposition {
    Accepted,
    Coalesced,
    Gated,
    Skipped,
}
