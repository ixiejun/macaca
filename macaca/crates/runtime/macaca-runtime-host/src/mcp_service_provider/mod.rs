//! Runtime-host adapter for the MCP Service (Facade module tree).
//!
//! The adapter owns protocol lifecycle dispatch for MCP service calls while
//! delegating actual runtime operations to `McpRuntimeFacade`.  It exposes
//! structured status, snapshots, operator lifecycle commands, and guarded tool
//! invocation through provider-neutral service contracts.
//!
//! **Pattern:** Facade + Command Router — `system_service` owns lifecycle hooks,
//! `command_dispatch` routes typed commands, and focused handler modules keep
//! each MCP syscall surface under the 500-line constitution.
//!
//! Module layout:
//! - `descriptor.rs` — provider-neutral service descriptor factory
//! - `support.rs` — DTO shaping, policy translation, admission validation
//! - `system_service.rs` — `SystemService` lifecycle syscall surface
//! - `command_dispatch.rs` — Command Router match table
//! - `runtime_facade_commands.rs` — register/probe/catalog/attach/status/snapshot/cleanup
//! - `tool_invoke_commands.rs` — guarded tool invocation with policy admission
//! - `operator_commands.rs` — reload/oauth/resources/diagnostics operator lifecycle

mod command_dispatch;
mod descriptor;
mod operator_commands;
mod runtime_facade_commands;
mod support;
mod system_service;
mod tool_invoke_commands;

#[cfg(test)]
mod tests;

pub use descriptor::mcp_service_descriptor;

use std::collections::BTreeMap;
use std::sync::Arc;

use macaca_proto::{
    CleanupPolicy, ServiceCallResult, ServiceCommand, ServiceDescriptor, ServiceError,
    ServiceResult, TraceContext,
};

use crate::mcp_operator_lifecycle::{McpOperatorLifecycle, McpOperatorState};
use crate::mcp_runtime::McpRuntimeFacade;

/// Host-owned MCP service provider backed by an optional facade.
pub struct McpSystemServiceProvider {
    pub(crate) descriptor: ServiceDescriptor,
    pub(crate) facade: Option<Arc<McpRuntimeFacade>>,
    pub(crate) operator_state: Arc<McpOperatorState>,
}

impl McpSystemServiceProvider {
    /// Create a service provider backed by an existing MCP runtime facade.
    pub fn new(facade: Arc<McpRuntimeFacade>) -> Self {
        Self {
            descriptor: mcp_service_descriptor(),
            facade: Some(facade),
            operator_state: Arc::new(McpOperatorState::default()),
        }
    }

    /// Create a Null Object provider that returns structured unavailable.
    pub fn unavailable() -> Self {
        Self {
            descriptor: mcp_service_descriptor(),
            facade: None,
            operator_state: Arc::new(McpOperatorState::default()),
        }
    }

    pub(crate) fn facade(&self) -> ServiceResult<Arc<McpRuntimeFacade>> {
        self.facade
            .clone()
            .ok_or_else(|| ServiceError::ServiceUnavailable("MCP runtime is not configured".into()))
    }

    pub(crate) fn operator(&self) -> ServiceResult<McpOperatorLifecycle> {
        Ok(McpOperatorLifecycle::new(
            self.facade()?,
            self.operator_state.clone(),
        ))
    }

    pub(crate) fn trace(command: &ServiceCommand) -> ServiceResult<TraceContext> {
        command
            .trace
            .clone()
            .ok_or(ServiceError::MissingTraceContext)
    }

    pub(crate) fn service_result(
        output: serde_json::Value,
        trace: TraceContext,
    ) -> ServiceCallResult {
        ServiceCallResult {
            output,
            trace,
            status: "ok".into(),
            metadata: BTreeMap::new(),
            cleanup_hint: Some(CleanupPolicy::None),
        }
    }
}
