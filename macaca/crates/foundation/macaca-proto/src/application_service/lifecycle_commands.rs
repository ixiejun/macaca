//! Application lifecycle and session command DTOs (Command pattern).
//!
//! Each struct is a typed service command envelope for discover/load/start/stop,
//! session management, and host dispatch.  Providers map these DTOs to runtime
//! operations while shells remain decoupled from `macaca-app` internals.

use serde::{Deserialize, Serialize};

use crate::{ApplicationHostCommand, MacacaResult, TraceContext};

use super::scope::{ApplicationServicePolicyHints, ApplicationServiceScope};
use super::validation::validate_trace;

/// Discover applications from configured application directories.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationDiscoverCommand {
    pub trace: TraceContext,
    pub include_manifest_metadata: bool,
    pub policy: ApplicationServicePolicyHints,
}

impl ApplicationDiscoverCommand {
    /// Build a traced discovery command.
    pub fn new(trace: TraceContext) -> MacacaResult<Self> {
        validate_trace(&trace, "application discover command requires trace_id")?;
        Ok(Self {
            trace,
            include_manifest_metadata: false,
            policy: ApplicationServicePolicyHints::default(),
        })
    }
}

/// Load/admit one application without necessarily starting it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationLoadCommand {
    pub trace: TraceContext,
    pub manifest_path: Option<String>,
    pub package_ref: Option<String>,
    pub policy: ApplicationServicePolicyHints,
}

/// Start one application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationStartCommand {
    pub trace: TraceContext,
    pub manifest_path: Option<String>,
    pub manifest: Option<serde_json::Value>,
    pub app_dir: Option<String>,
    pub policy: ApplicationServicePolicyHints,
}

/// Stop one running application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationStopCommand {
    pub trace: TraceContext,
    pub scope: ApplicationServiceScope,
}

/// Remove one stopped application from the runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationRemoveCommand {
    pub trace: TraceContext,
    pub scope: ApplicationServiceScope,
}

/// Read status for one application or all applications when scope is empty.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationStatusCommand {
    pub trace: TraceContext,
    pub scope: ApplicationServiceScope,
}

/// Read a deterministic, sanitized Application Service snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationSnapshotCommand {
    pub trace: TraceContext,
    pub include_discovered: bool,
    pub include_running: bool,
}

/// Start a session envelope for an application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationSessionStartCommand {
    pub trace: TraceContext,
    pub scope: ApplicationServiceScope,
}

/// Resume a session envelope for an application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationSessionResumeCommand {
    pub trace: TraceContext,
    pub scope: ApplicationServiceScope,
}

/// Stop a session envelope for an application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationSessionStopCommand {
    pub trace: TraceContext,
    pub scope: ApplicationServiceScope,
    pub reason: Option<String>,
}

/// Dispatch an ApplicationHost command through the service boundary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationHostDispatchServiceCommand {
    pub trace: TraceContext,
    pub scope: ApplicationServiceScope,
    pub host_command: ApplicationHostCommand,
}
