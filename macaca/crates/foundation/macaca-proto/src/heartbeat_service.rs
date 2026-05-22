//! Provider-neutral Heartbeat service contracts.
//!
//! The Heartbeat service coordinates wake-loop intent, coalescing, gate
//! evaluation, and run evidence.  It is not a task runner and does not own
//! application-specific workflows.  Providers may dispatch generic service
//! commands after gates pass, but task planning, execution, review, delivery,
//! and business logic remain behind their own service boundaries.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    autonomy_service::{non_empty, validate_trace},
    AutonomyPayloadRef, AutonomyScope, AutonomyStructuredError, MacacaResult, ServiceCommand,
    ServiceCommandName, TraceContext,
};

/// Stable service id used by runtime-host registration and SDK clients.
pub const HEARTBEAT_SERVICE_ID: &str = "service.heartbeat";

/// Command names accepted by Heartbeat service providers.
pub const HEARTBEAT_WAKE_COMMAND: &str = "heartbeat.wake";
pub const HEARTBEAT_CANCEL_WAKE_COMMAND: &str = "heartbeat.wake.cancel";
pub const HEARTBEAT_GET_RUN_COMMAND: &str = "heartbeat.run.get";
pub const HEARTBEAT_LIST_RUNS_COMMAND: &str = "heartbeat.run.list";
pub const HEARTBEAT_COMPLETE_RUN_COMMAND: &str = "heartbeat.run.complete";
pub const HEARTBEAT_UPDATE_PROFILE_COMMAND: &str = "heartbeat.profile.update";
pub const HEARTBEAT_HEALTH_COMMAND: &str = "heartbeat.health";
pub const HEARTBEAT_SNAPSHOT_COMMAND: &str = "heartbeat.snapshot";

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

/// Provider-neutral command accepted by `service.heartbeat`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeartbeatWakeCommand {
    pub trace: TraceContext,
    pub scope: AutonomyScope,
    pub wake_scope_key: String,
    pub intent: HeartbeatWakeIntent,
    pub payload_ref: Option<AutonomyPayloadRef>,
    pub metadata: BTreeMap<String, String>,
}

impl HeartbeatWakeCommand {
    /// Build a traced wake request and reject anonymous autonomous loops.
    pub fn new(
        trace: TraceContext,
        scope: AutonomyScope,
        wake_scope_key: impl Into<String>,
        intent: HeartbeatWakeIntent,
    ) -> MacacaResult<Self> {
        validate_trace(&trace, "heartbeat wake command requires trace_id")?;
        Ok(Self {
            trace,
            scope,
            wake_scope_key: non_empty(
                wake_scope_key.into(),
                "heartbeat wake_scope_key is required",
            )?,
            intent,
            payload_ref: None,
            metadata: BTreeMap::new(),
        })
    }

    /// Build the canonical wake command emitted by `service.scheduler`.
    ///
    /// This helper keeps recurring heartbeat integration provider-neutral: the
    /// scheduler only creates a typed heartbeat wake command with
    /// `ScheduledTick` intent.  It does not know how heartbeat coalesces,
    /// gates, dispatches, or reports wake runs.
    pub fn scheduled_tick(
        trace: TraceContext,
        scope: AutonomyScope,
        wake_scope_key: impl Into<String>,
    ) -> MacacaResult<Self> {
        Self::new(
            trace,
            scope,
            wake_scope_key,
            HeartbeatWakeIntent::ScheduledTick,
        )
    }

    /// Convert this typed DTO into the generic service runtime command shape.
    pub fn into_service_command(self) -> MacacaResult<ServiceCommand> {
        let trace = self.trace.clone();
        Ok(ServiceCommand::with_trace(
            ServiceCommandName::new(HEARTBEAT_WAKE_COMMAND),
            serde_json::to_value(self)?,
            trace,
        ))
    }
}

/// Command for cancelling a pending heartbeat wake.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeartbeatCancelWakeCommand {
    pub trace: TraceContext,
    pub run_id: HeartbeatRunId,
    pub reason_code: String,
    pub metadata: BTreeMap<String, String>,
}

impl HeartbeatCancelWakeCommand {
    /// Build a traced cancellation command for audit-visible intervention.
    pub fn new(
        trace: TraceContext,
        run_id: HeartbeatRunId,
        reason_code: impl Into<String>,
    ) -> MacacaResult<Self> {
        validate_trace(&trace, "heartbeat cancel command requires trace_id")?;
        Ok(Self {
            trace,
            run_id,
            reason_code: non_empty(reason_code.into(), "heartbeat reason_code is required")?,
            metadata: BTreeMap::new(),
        })
    }
}

/// Command for recording the terminal outcome of work delegated from a wake.
///
/// Heartbeat owns wake mementos but does not execute application agents. Runtime
/// Host therefore reports the observed dispatch result back through this typed
/// command after Agent Execution, task execution, or another generic runtime
/// boundary finishes. The command accepts only terminal states and bounded
/// metadata so run history remains replayable without storing raw prompts,
/// provider output, manifests, package bytes, credentials, or unbounded payloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeartbeatCompleteRunCommand {
    pub trace: TraceContext,
    pub run_id: HeartbeatRunId,
    pub state: HeartbeatRunState,
    pub reason_code: String,
    pub metadata: BTreeMap<String, String>,
}

impl HeartbeatCompleteRunCommand {
    /// Build a traced completion command for a terminal heartbeat run state.
    pub fn new(
        trace: TraceContext,
        run_id: HeartbeatRunId,
        state: HeartbeatRunState,
        reason_code: impl Into<String>,
    ) -> MacacaResult<Self> {
        validate_trace(&trace, "heartbeat run completion requires trace_id")?;
        match state {
            HeartbeatRunState::Succeeded
            | HeartbeatRunState::Failed
            | HeartbeatRunState::Skipped => {}
            _ => {
                return Err(crate::MacacaError::Config(
                    "heartbeat run completion requires a terminal state".into(),
                ));
            }
        }
        Ok(Self {
            trace,
            run_id,
            state,
            reason_code: non_empty(
                reason_code.into(),
                "heartbeat completion reason_code is required",
            )?,
            metadata: BTreeMap::new(),
        })
    }

    /// Convert this typed DTO into the generic service runtime command shape.
    pub fn into_service_command(self) -> MacacaResult<ServiceCommand> {
        let trace = self.trace.clone();
        Ok(ServiceCommand::with_trace(
            ServiceCommandName::new(HEARTBEAT_COMPLETE_RUN_COMMAND),
            serde_json::to_value(self)?,
            trace,
        ))
    }
}

/// Command for reading heartbeat diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeartbeatQueryCommand {
    pub trace: TraceContext,
    pub scope: AutonomyScope,
    pub run_id: Option<HeartbeatRunId>,
    pub wake_scope_key: Option<String>,
    pub limit: Option<usize>,
}

/// Trace-bearing command for editing native Heartbeat profile policy.
///
/// Profile policy is Heartbeat-owned runtime state. Application manifests keep
/// declaring which agents participate in heartbeat execution; this command only
/// updates the native profile that gates cadence and operator metadata for a
/// provider-neutral scope. That split prevents Web shells from becoming raw
/// manifest editors while still giving operators auditable runtime control.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeartbeatUpdateProfileCommand {
    pub trace: TraceContext,
    pub scope: AutonomyScope,
    pub profile_id: HeartbeatProfileId,
    pub enabled: Option<bool>,
    pub fixed_interval_ms: Option<u64>,
    pub cooldown_ms: Option<u64>,
    pub metadata: BTreeMap<String, String>,
    pub reason_code: String,
}

impl HeartbeatUpdateProfileCommand {
    /// Build a profile update command with trace and scope validation.
    pub fn new(
        trace: TraceContext,
        scope: AutonomyScope,
        profile_id: HeartbeatProfileId,
        reason_code: impl Into<String>,
    ) -> MacacaResult<Self> {
        validate_trace(&trace, "heartbeat profile update requires trace_id")?;
        Ok(Self {
            trace,
            scope,
            profile_id,
            enabled: None,
            fixed_interval_ms: None,
            cooldown_ms: None,
            metadata: BTreeMap::new(),
            reason_code: non_empty(reason_code.into(), "heartbeat reason_code is required")?,
        })
    }

    /// Attach an optional fixed interval update.
    pub fn with_fixed_interval_ms(mut self, interval_ms: Option<u64>) -> MacacaResult<Self> {
        if matches!(interval_ms, Some(0)) {
            return Err(crate::MacacaError::Config(
                "heartbeat interval must be positive".into(),
            ));
        }
        self.fixed_interval_ms = interval_ms;
        Ok(self)
    }

    /// Attach an optional cooldown update.
    pub fn with_cooldown_ms(mut self, cooldown_ms: Option<u64>) -> MacacaResult<Self> {
        if matches!(cooldown_ms, Some(0)) {
            return Err(crate::MacacaError::Config(
                "heartbeat cooldown must be positive".into(),
            ));
        }
        self.cooldown_ms = cooldown_ms;
        Ok(self)
    }
}

/// Bounded run summary returned by heartbeat history and snapshots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeartbeatRunSummary {
    pub run_id: HeartbeatRunId,
    pub wake_scope_key: String,
    pub intent: HeartbeatWakeIntent,
    pub state: HeartbeatRunState,
    pub disposition: HeartbeatWakeDisposition,
    pub requested_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub gates: Vec<HeartbeatGateDecision>,
    pub trace_id: String,
    pub audit_id: Option<String>,
    pub safe_status: String,
    pub metadata: BTreeMap<String, String>,
}

/// Typed result returned by Heartbeat service commands.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeartbeatCommandResult {
    pub run_id: Option<HeartbeatRunId>,
    pub state: Option<HeartbeatRunState>,
    pub disposition: HeartbeatWakeDisposition,
    pub gates: Vec<HeartbeatGateDecision>,
    pub accepted: bool,
    pub error: Option<AutonomyStructuredError>,
    pub trace: TraceContext,
    pub audit_id: Option<String>,
    pub metadata: BTreeMap<String, String>,
}

/// Result returned after a native Heartbeat profile mutation.
///
/// The result carries a sanitized profile summary instead of the full profile
/// declaration. It is safe for Web, CLI, trace, and audit surfaces because it
/// contains only identifiers, cadence timing, lifecycle state, metadata, and
/// audit correlation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeartbeatProfileMutationResult {
    pub accepted: bool,
    pub profile: Option<HeartbeatProfileSummary>,
    pub trace: TraceContext,
    pub audit_id: Option<String>,
    pub error: Option<AutonomyStructuredError>,
    pub metadata: BTreeMap<String, String>,
}

/// Provider-neutral heartbeat service snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeartbeatServiceSnapshot {
    pub service_id: String,
    pub provider_id: String,
    pub healthy: bool,
    pub lifecycle_state: String,
    pub pending_wakes: usize,
    pub active_runs: usize,
    pub scheduler_ticks_active: bool,
    pub native_profiles_active: usize,
    pub native_profiles: Vec<HeartbeatProfileSummary>,
    pub recent_runs: Vec<HeartbeatRunSummary>,
    pub last_gate_decisions: Vec<HeartbeatGateDecision>,
    pub last_audit_ids: Vec<String>,
    pub captured_at: DateTime<Utc>,
}

impl HeartbeatServiceSnapshot {
    /// Build a fail-closed snapshot for hosts without heartbeat providers.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            service_id: HEARTBEAT_SERVICE_ID.into(),
            provider_id: "unavailable".into(),
            healthy: false,
            lifecycle_state: "unavailable".into(),
            pending_wakes: 0,
            active_runs: 0,
            scheduler_ticks_active: false,
            native_profiles_active: 0,
            native_profiles: Vec::new(),
            recent_runs: Vec::new(),
            last_gate_decisions: Vec::new(),
            last_audit_ids: vec![reason.into()],
            captured_at: Utc::now(),
        }
    }
}
