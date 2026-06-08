//! Application Service query command DTOs (Command pattern).
//!
//! Query commands fetch sanitized metadata, heartbeat declarations, and GenUI
//! surfaces through the same traced service boundary as lifecycle operations.

use serde::{Deserialize, Serialize};

use crate::{ApplicationId, MacacaResult, TraceContext};

use super::constants::APPLICATION_HEARTBEAT_AGENTS_QUERY_COMMAND;
use super::scope::ApplicationServiceScope;
use super::validation::validate_trace;

/// Query one application-owned GenUI surface.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationGenUiSurfaceCommand {
    pub trace: TraceContext,
    pub scope: ApplicationServiceScope,
    pub surface_id: Option<String>,
}

/// Query sanitized application-owned metadata through Application Service.
///
/// The command uses the Command pattern so shells can ask for metadata through
/// the same traced service boundary used by lifecycle operations.  It carries
/// only scope and view-selection flags; providers must not attach raw manifest
/// bodies, raw agent configs, prompt templates, secrets, environment values, or
/// host payloads to the payload or logs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationMetadataQueryCommand {
    pub trace: TraceContext,
    pub scope: ApplicationServiceScope,
    pub include_abilities: bool,
    pub include_policy: bool,
    pub include_overlay: bool,
    pub include_digest: bool,
}

impl ApplicationMetadataQueryCommand {
    /// Build a fail-closed application metadata query.
    pub fn application(trace: TraceContext, application_id: ApplicationId) -> MacacaResult<Self> {
        validate_trace(&trace, "application metadata query requires trace_id")?;
        Ok(Self {
            trace,
            scope: ApplicationServiceScope::application(application_id),
            include_abilities: true,
            include_policy: true,
            include_overlay: true,
            include_digest: true,
        })
    }
}

/// Query manifest-declared heartbeat agents through Application Service.
///
/// The command uses the Command pattern instead of exposing manifests directly.
/// Runtime-host callers provide only trace and application scope; providers own
/// manifest lookup, validation, sanitization, and bounded diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationHeartbeatAgentsQueryCommand {
    pub trace: TraceContext,
    pub scope: ApplicationServiceScope,
}

impl ApplicationHeartbeatAgentsQueryCommand {
    /// Build an application-scoped heartbeat declaration query.
    pub fn application(trace: TraceContext, application_id: ApplicationId) -> MacacaResult<Self> {
        validate_trace(
            &trace,
            "application heartbeat agents query requires trace_id",
        )?;
        Ok(Self {
            trace,
            scope: ApplicationServiceScope::application(application_id),
        })
    }

    /// Convert this typed query into the generic service command envelope.
    pub fn into_service_command(self) -> MacacaResult<crate::ServiceCommand> {
        let trace = self.trace.clone();
        Ok(crate::ServiceCommand::with_trace(
            crate::ServiceCommandName::new(APPLICATION_HEARTBEAT_AGENTS_QUERY_COMMAND),
            serde_json::to_value(self)?,
            trace,
        ))
    }
}
