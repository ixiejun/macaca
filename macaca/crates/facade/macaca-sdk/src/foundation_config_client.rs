//! SDK Facade helpers for `pack.foundation.config.v1`.
//!
//! These helpers only create canonical traced service calls after descriptor
//! admission. They never read environment variables, resolve secret values, or
//! instantiate package, workspace, tenant, or remote config providers.

use macaca_proto::{ConfigAdmissionFailure, ConfigResourceReservation, MacacaResult, TraceContext};
use tracing::{info, warn};

use crate::domain_pack_client::{DomainPackResolveResult, DomainPackServiceCallBuilder};
use crate::service_client::ServiceCallCommand;

const SERVICE_ID: &str = "service.foundation.config";

/// Result of configuration preflight and canonical command construction.
///
/// A rejected outcome contains only the stable policy reason. It intentionally
/// does not construct a service command, so SDK callers cannot bypass approval,
/// quota, secret-reference, or provider-availability decisions.
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigDomainPackCommandBuildOutcome {
    Ready(ServiceCallCommand),
    Rejected(ConfigAdmissionFailure),
}

/// Provider-neutral builder for declared foundation config commands.
#[derive(Debug, Clone, PartialEq)]
pub struct ConfigDomainPackCommandBuilder {
    command_name: String,
    payload: serde_json::Value,
    trace: TraceContext,
}

impl ConfigDomainPackCommandBuilder {
    /// Capture a validated DTO payload and its required trace context.
    pub fn new(
        command_name: impl Into<String>,
        payload: serde_json::Value,
        trace: TraceContext,
    ) -> Self {
        Self {
            command_name: command_name.into(),
            payload,
            trace,
        }
    }

    /// Build an admitted generic service call without exposing a provider handle.
    pub fn build(self, resolved: &DomainPackResolveResult) -> MacacaResult<ServiceCallCommand> {
        info!(service_id = SERVICE_ID, command = %self.command_name, trace_id = %self.trace.trace_id,
            "foundation_config_sdk_command_built");
        DomainPackServiceCallBuilder::new(SERVICE_ID, self.command_name, self.payload, self.trace)?
            .build(resolved)
    }

    /// Build only after a policy decorator has reserved resources and approved it.
    ///
    /// This additive helper preserves the original builder API for compatibility
    /// while giving new callers a fail-closed path that cannot reach the service
    /// runtime when configuration admission rejected the request.
    pub fn build_after_preflight(
        self,
        resolved: &DomainPackResolveResult,
        decision: Result<ConfigResourceReservation, ConfigAdmissionFailure>,
    ) -> MacacaResult<ConfigDomainPackCommandBuildOutcome> {
        match decision {
            Ok(_) => Ok(ConfigDomainPackCommandBuildOutcome::Ready(
                self.build(resolved)?,
            )),
            Err(reason) => {
                warn!(service_id = SERVICE_ID, trace_id = %self.trace.trace_id,
                    status = ?reason, "foundation_config_sdk_preflight_rejected");
                Ok(ConfigDomainPackCommandBuildOutcome::Rejected(reason))
            }
        }
    }
}

/// Build `config.get` for typed configuration lookup through the service runtime.
pub fn config_get_command(
    payload: serde_json::Value,
    trace: TraceContext,
) -> ConfigDomainPackCommandBuilder {
    command("config.get", payload, trace)
}
/// Build `config.describe_schema` without exposing provider-native schemas.
pub fn config_describe_schema_command(
    payload: serde_json::Value,
    trace: TraceContext,
) -> ConfigDomainPackCommandBuilder {
    command("config.describe_schema", payload, trace)
}
/// Build `config.get_many` for a bounded batch lookup through the service runtime.
pub fn config_get_many_command(
    payload: serde_json::Value,
    trace: TraceContext,
) -> ConfigDomainPackCommandBuilder {
    command("config.get_many", payload, trace)
}
/// Build `config.list_keys` for a policy-bounded metadata listing.
pub fn config_list_keys_command(
    payload: serde_json::Value,
    trace: TraceContext,
) -> ConfigDomainPackCommandBuilder {
    command("config.list_keys", payload, trace)
}
/// Build `config.resolve_effective` for layered, provider-neutral resolution.
pub fn config_effective_command(
    payload: serde_json::Value,
    trace: TraceContext,
) -> ConfigDomainPackCommandBuilder {
    command("config.resolve_effective", payload, trace)
}
/// Build `config.validate` for schema validation without placing candidate values in SDK logs.
pub fn config_validate_command(
    payload: serde_json::Value,
    trace: TraceContext,
) -> ConfigDomainPackCommandBuilder {
    command("config.validate", payload, trace)
}
/// Build `config.explain_provenance` for redacted source and precedence evidence.
pub fn config_provenance_command(
    payload: serde_json::Value,
    trace: TraceContext,
) -> ConfigDomainPackCommandBuilder {
    command("config.explain_provenance", payload, trace)
}
/// Build the lifecycle-owned `config.watch` stream request; runtime cleanup cancels the stream.
pub fn config_watch_command(
    payload: serde_json::Value,
    trace: TraceContext,
) -> ConfigDomainPackCommandBuilder {
    command("config.watch", payload, trace)
}
/// Build `config.reload`; a caller must supply an approved preflight decision.
pub fn config_reload_command(
    payload: serde_json::Value,
    trace: TraceContext,
) -> ConfigDomainPackCommandBuilder {
    command("config.reload", payload, trace)
}
/// Build `config.snapshot` for bounded, redacted replay evidence.
pub fn config_snapshot_command(
    payload: serde_json::Value,
    trace: TraceContext,
) -> ConfigDomainPackCommandBuilder {
    command("config.snapshot", payload, trace)
}
/// Build `config.export_redacted` for diagnostics that intentionally omit raw values.
pub fn config_export_redacted_command(
    payload: serde_json::Value,
    trace: TraceContext,
) -> ConfigDomainPackCommandBuilder {
    command("config.export_redacted", payload, trace)
}
/// Build a schema query that exposes structured unavailable diagnostics through the service.
pub fn config_unavailable_diagnostics_command(
    payload: serde_json::Value,
    trace: TraceContext,
) -> ConfigDomainPackCommandBuilder {
    config_describe_schema_command(payload, trace)
}

fn command(
    command_name: &str,
    payload: serde_json::Value,
    trace: TraceContext,
) -> ConfigDomainPackCommandBuilder {
    ConfigDomainPackCommandBuilder::new(command_name, payload, trace)
}

#[cfg(test)]
#[path = "foundation_config_client_tests.rs"]
mod tests;
