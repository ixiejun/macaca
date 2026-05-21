//! Shared protocol primitives for autonomous Macaca OS services.
//!
//! Scheduler and heartbeat providers both need the same safe vocabulary for
//! scope, payload references, structured failures, and audit correlation.  The
//! types in this module are deliberately data-only.  They can cross SDK,
//! service-runtime, plugin, IPC, and future remote-provider boundaries without
//! pulling concrete cron loops, queues, storage engines, shell state, or
//! application-specific workflow logic into `macaca-proto`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{ApplicationId, MacacaError, MacacaResult, TaskId, TraceContext};

/// Provider-neutral scope shared by autonomy service commands.
///
/// The scope records generic OS identities only.  Service providers and policy
/// engines may use these facts for authorization and isolation, but they must
/// not branch on application names, provider names, workflow names, or business
/// domains.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutonomyScope {
    pub application_id: Option<ApplicationId>,
    pub session_id: Option<String>,
    pub task_id: Option<TaskId>,
    pub tenant_id: Option<String>,
    pub metadata: BTreeMap<String, String>,
}

impl AutonomyScope {
    /// Build a global scope for OS-level diagnostics or unavailable providers.
    pub fn global() -> Self {
        Self::default()
    }

    /// Build an application-scoped command boundary without encoding product
    /// behavior in the OS contract.
    pub fn application(application_id: ApplicationId) -> Self {
        Self {
            application_id: Some(application_id),
            ..Self::default()
        }
    }

    /// Build a session-scoped command boundary and reject empty session ids so
    /// audit replay never has to guess which autonomous loop emitted evidence.
    pub fn session(
        application_id: ApplicationId,
        session_id: impl Into<String>,
    ) -> MacacaResult<Self> {
        Ok(Self {
            application_id: Some(application_id),
            session_id: Some(non_empty(session_id.into(), "session_id is required")?),
            ..Self::default()
        })
    }
}

/// Bounded reference to command payload material.
///
/// Autonomous services must be traceable, but their snapshots and audit records
/// must not embed raw prompts, secrets, manifests, WASM bytes, package bytes, or
/// unbounded provider payloads.  This value object stores a reference, an
/// optional digest, and a safe human summary instead of the payload itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutonomyPayloadRef {
    pub reference: String,
    pub content_digest: Option<String>,
    pub redacted_summary: String,
    pub metadata: BTreeMap<String, String>,
}

impl AutonomyPayloadRef {
    /// Create a sanitized payload reference.
    pub fn new(
        reference: impl Into<String>,
        redacted_summary: impl Into<String>,
    ) -> MacacaResult<Self> {
        Ok(Self {
            reference: non_empty(reference.into(), "payload reference is required")?,
            content_digest: None,
            redacted_summary: non_empty(
                redacted_summary.into(),
                "payload redacted summary is required",
            )?,
            metadata: BTreeMap::new(),
        })
    }
}

/// Correlation identifiers returned by service providers after audit writes.
///
/// The trace context proves the execution chain.  The optional audit id points
/// to sanitized durable evidence when a provider has already appended an audit
/// record.  Keeping both fields together makes facade results replay-friendly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutonomyAuditCorrelation {
    pub trace: TraceContext,
    pub audit_id: Option<String>,
    pub metadata: BTreeMap<String, String>,
}

impl AutonomyAuditCorrelation {
    /// Build correlation metadata from a validated trace context.
    pub fn from_trace(trace: TraceContext) -> MacacaResult<Self> {
        validate_trace(&trace, "autonomy audit correlation requires trace_id")?;
        Ok(Self {
            trace,
            audit_id: None,
            metadata: BTreeMap::new(),
        })
    }
}

/// Cross-service structured error categories for autonomy service boundaries.
///
/// These categories mirror the serviceization constitution.  Providers return
/// data instead of panicking, silently falling back, or pretending work was
/// scheduled when a capability is unavailable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AutonomyServiceErrorKind {
    Unavailable,
    Unsupported,
    Denied,
    InvalidRequest,
    Conflict,
    ProviderFailure,
    Timeout,
}

/// Safe error envelope for Scheduler and Heartbeat service results.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutonomyStructuredError {
    pub kind: AutonomyServiceErrorKind,
    pub reason_code: String,
    pub safe_message: String,
    pub correlation: AutonomyAuditCorrelation,
    pub metadata: BTreeMap<String, String>,
}

impl AutonomyStructuredError {
    /// Build a fail-closed unavailable error for Null Object providers.
    pub fn unavailable(trace: TraceContext, safe_message: impl Into<String>) -> MacacaResult<Self> {
        Ok(Self {
            kind: AutonomyServiceErrorKind::Unavailable,
            reason_code: "provider_unavailable".into(),
            safe_message: non_empty(
                safe_message.into(),
                "autonomy unavailable message is required",
            )?,
            correlation: AutonomyAuditCorrelation::from_trace(trace)?,
            metadata: BTreeMap::new(),
        })
    }
}

/// Reject empty trace ids before commands leave facade or service boundaries.
pub(crate) fn validate_trace(trace: &TraceContext, message: &'static str) -> MacacaResult<()> {
    if trace.trace_id.trim().is_empty() {
        return Err(MacacaError::Config(message.into()));
    }
    Ok(())
}

/// Normalize user-facing labels while keeping validation in one place.
pub(crate) fn non_empty(value: String, message: &'static str) -> MacacaResult<String> {
    let value = value.trim().to_string();
    if value.is_empty() {
        return Err(MacacaError::Config(message.into()));
    }
    Ok(value)
}
