//! Application-scoped agent delegation command and result DTOs (Adapter pattern).
//!
//! Delegation crosses from Application Service into Agent Execution Service via
//! `into_service_command`, preserving a single audited entrypoint for WASM,
//! Web, CLI, and workflow hosts without exposing executor internals.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{ApplicationId, MacacaResult, TraceContext};

use super::constants::APPLICATION_AGENT_DELEGATE_COMMAND;
use super::scope::ApplicationServiceScope;

/// Request app-scoped agent work through the Application Service.
///
/// This command is intentionally provider-neutral.  It carries only app,
/// session, agent, prompt, trace, and bounded JSON context so WASM runtimes,
/// Web, CLI, or future remote hosts can request delegated work without holding
/// `ApplicationExecutor`, Web state, Toolkit, or concrete provider handles.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationAgentDelegateCommand {
    pub trace: TraceContext,
    pub scope: ApplicationServiceScope,
    pub target_agent: String,
    pub prompt: String,
    pub context: serde_json::Value,
    pub metadata: BTreeMap<String, String>,
}

impl ApplicationAgentDelegateCommand {
    /// Convert this typed delegation command into a traced service envelope.
    ///
    /// Callers use the Application Service boundary (`service.application`) so
    /// YAML workflow, WASM host imports, and future remote transports share the
    /// same audited entrypoint before reaching `service.agent_execution`.
    pub fn into_service_command(self) -> MacacaResult<crate::ServiceCommand> {
        let trace = self.trace.clone();
        Ok(crate::ServiceCommand::with_trace(
            crate::ServiceCommandName::new(APPLICATION_AGENT_DELEGATE_COMMAND),
            serde_json::to_value(self)?,
            trace,
        ))
    }
}

/// Provider-neutral result for app-scoped agent delegation.
///
/// The result is a Memento-friendly summary rather than a raw worker runtime
/// object.  Hosts can persist or replay it alongside trace/audit events without
/// exposing agent internals, tool payload secrets, or presentation-shell state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationAgentDelegateResult {
    pub application_id: ApplicationId,
    pub session_id: String,
    pub target_agent: String,
    pub task_id: Option<String>,
    pub success: bool,
    pub output: serde_json::Value,
    pub status: String,
    pub metadata: BTreeMap<String, String>,
}
