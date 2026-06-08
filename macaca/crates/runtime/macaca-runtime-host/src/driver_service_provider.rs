//! Runtime-host adapter for the Route C Driver Service.
//!
//! This module uses the Adapter/Bridge pattern: `ServiceRuntime` dispatches
//! provider-neutral `ServiceCommand` values, while the adapter delegates driver
//! lifecycle and tool execution to an injected `DriverRuntime`.  The adapter is
//! intentionally host-owned because it coordinates lifecycle, trace, structured
//! results, and unavailable behavior without moving driver semantics into the
//! kernel or Web shell.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use macaca_driver::{
    driver_service_descriptor, DriverCleanupCommand, DriverInventoryCommand, DriverInventoryEntry,
    DriverInventoryResult, DriverLoadServiceCommand, DriverRuntime, DriverServiceSnapshot,
    DriverServiceSnapshotCommand, DriverStatusCommand, DriverStatusResult,
    DriverToolCatalogCommand, DriverToolCatalogResult, DriverToolInvokeCommand,
    DRIVER_CLEANUP_COMMAND, DRIVER_INVENTORY_COMMAND, DRIVER_LOAD_COMMAND, DRIVER_RELOAD_COMMAND,
    DRIVER_SERVICE_ID, DRIVER_SNAPSHOT_COMMAND, DRIVER_STATUS_COMMAND, DRIVER_TOOL_CATALOG_COMMAND,
    DRIVER_TOOL_INVOKE_COMMAND,
};
use macaca_kernel::SystemService;
use macaca_proto::{
    CapabilityToolDescriptor, CapabilityToolInvocationResult, CapabilityToolOriginKind,
    CapabilityToolResourceScope, CleanupPolicy, MacacaError, ServiceCallResult, ServiceCommand,
    ServiceDescriptor, ServiceError, ServiceHealth, ServiceResult, TraceContext,
};
use macaca_tools::{ToolCommand, ToolCommandExecutor, ToolSchemaProvider};

/// Host-owned Driver service provider backed by an optional runtime.
pub struct DriverSystemServiceProvider {
    descriptor: ServiceDescriptor,
    runtime: Option<Arc<DriverRuntime>>,
}

impl DriverSystemServiceProvider {
    /// Create a service provider backed by an existing driver runtime.
    pub fn new(runtime: Arc<DriverRuntime>) -> Self {
        Self {
            descriptor: driver_service_descriptor(),
            runtime: Some(runtime),
        }
    }

    /// Create a Null Object provider that returns structured unavailable.
    pub fn unavailable() -> Self {
        Self {
            descriptor: driver_service_descriptor(),
            runtime: None,
        }
    }

    fn runtime(&self) -> ServiceResult<Arc<DriverRuntime>> {
        self.runtime.clone().ok_or_else(|| {
            ServiceError::ServiceUnavailable("driver runtime is not configured".into())
        })
    }

    fn trace(command: &ServiceCommand) -> ServiceResult<TraceContext> {
        command
            .trace
            .clone()
            .ok_or(ServiceError::MissingTraceContext)
    }

    fn service_result(output: serde_json::Value, trace: TraceContext) -> ServiceCallResult {
        ServiceCallResult {
            output,
            trace,
            status: "ok".into(),
            metadata: BTreeMap::new(),
            cleanup_hint: Some(CleanupPolicy::None),
        }
    }
}

#[async_trait]
impl SystemService for DriverSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        tracing::info!(
            service_id = %self.descriptor.id,
            configured = self.runtime.is_some(),
            "driver service provider started"
        );
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = Self::trace(&command)?;
        tracing::info!(
            service_id = %self.descriptor.id,
            command = %command.name,
            trace_id = %trace.trace_id,
            "driver service command accepted"
        );
        match command.name.as_str() {
            DRIVER_LOAD_COMMAND => {
                let typed: DriverLoadServiceCommand = decode(command.payload)?;
                let runtime = self.runtime()?;
                tracing::info!(trace_id = %typed.trace.trace_id, "driver service loading drivers");
                let report = if typed.clear_existing {
                    runtime.reload().await
                } else {
                    runtime.load_all().await
                };
                Ok(Self::service_result(to_value(report)?, typed.trace))
            }
            DRIVER_RELOAD_COMMAND => {
                let typed: DriverLoadServiceCommand = decode(command.payload)?;
                let runtime = self.runtime()?;
                tracing::info!(trace_id = %typed.trace.trace_id, "driver service reloading drivers");
                let report = runtime.reload().await;
                Ok(Self::service_result(to_value(report)?, typed.trace))
            }
            DRIVER_INVENTORY_COMMAND => {
                let typed: DriverInventoryCommand = decode(command.payload)?;
                let runtime = self.runtime()?;
                let entries = inventory_entries(&runtime).await;
                tracing::info!(
                    trace_id = %typed.trace.trace_id,
                    count = entries.len(),
                    "driver service inventory emitted"
                );
                Ok(Self::service_result(
                    to_value(DriverInventoryResult::new(entries))?,
                    typed.trace,
                ))
            }
            DRIVER_TOOL_CATALOG_COMMAND => {
                let typed: DriverToolCatalogCommand = decode(command.payload)?;
                let runtime = self.runtime()?;
                let tools = runtime.snapshot_tool_catalog().await;
                let descriptors = tools
                    .iter()
                    .map(|tool| {
                        CapabilityToolDescriptor::new(
                            DRIVER_SERVICE_ID,
                            "driver-runtime",
                            format!("driver.tool.{}", tool.name()),
                            tool.name(),
                            tool.description(),
                            ToolSchemaProvider::tool_schema(tool.as_ref()),
                            CapabilityToolOriginKind::Driver,
                        )
                        .map(|descriptor| {
                            descriptor.with_policy_hints(
                                vec!["driver.execute".into()],
                                vec![CapabilityToolResourceScope::AgentSession],
                            )
                        })
                    })
                    .collect::<Result<Vec<_>, MacacaError>>()
                    .map_err(service_adapter_error)?;
                tracing::info!(
                    trace_id = %typed.trace.trace_id,
                    count = descriptors.len(),
                    "driver service tool catalog emitted"
                );
                Ok(Self::service_result(
                    to_value(DriverToolCatalogResult::new(descriptors))?,
                    typed.trace,
                ))
            }
            DRIVER_TOOL_INVOKE_COMMAND => {
                let typed: DriverToolInvokeCommand = decode(command.payload)?;
                let runtime = self.runtime()?;
                let tools = runtime.snapshot_tool_catalog().await;
                let tool = tools
                    .iter()
                    .find(|tool| tool.name() == typed.invocation.tool_name)
                    .ok_or_else(|| {
                        ServiceError::ServiceUnavailable(format!(
                            "driver tool '{}' is not available",
                            typed.invocation.tool_name
                        ))
                    })?;
                tracing::info!(
                    trace_id = %typed.invocation.trace.trace_id,
                    tool = %typed.invocation.tool_name,
                    "driver service invoking tool"
                );
                let output = ToolCommandExecutor::execute_command(
                    tool.as_ref(),
                    ToolCommand::new(typed.invocation.input.clone()),
                )
                .await
                .map_err(service_adapter_error)?;
                let result = CapabilityToolInvocationResult::ok(
                    DRIVER_SERVICE_ID,
                    CapabilityToolOriginKind::Driver,
                    typed.invocation.tool_name,
                    output,
                    typed.invocation.trace.clone(),
                );
                Ok(Self::service_result(
                    to_value(result)?,
                    typed.invocation.trace,
                ))
            }
            DRIVER_STATUS_COMMAND => {
                let typed: DriverStatusCommand = decode(command.payload)?;
                let runtime = self.runtime()?;
                let entries = runtime.list_inventory().await;
                let result = DriverStatusResult {
                    service_id: DRIVER_SERVICE_ID.into(),
                    healthy: true,
                    loaded: entries.len(),
                    failed: 0,
                    captured_at: chrono::Utc::now(),
                };
                Ok(Self::service_result(to_value(result)?, typed.trace))
            }
            DRIVER_SNAPSHOT_COMMAND => {
                let typed: DriverServiceSnapshotCommand = decode(command.payload)?;
                let runtime = self.runtime()?;
                let entries = inventory_entries(&runtime).await;
                let tool_count = entries.iter().map(|entry| entry.tool_count).sum();
                let snapshot = DriverServiceSnapshot::healthy(entries, tool_count);
                tracing::info!(trace_id = %typed.trace.trace_id, "driver service snapshot emitted");
                Ok(Self::service_result(to_value(snapshot)?, typed.trace))
            }
            DRIVER_CLEANUP_COMMAND => {
                let typed: DriverCleanupCommand = decode(command.payload)?;
                tracing::info!(trace_id = %typed.trace.trace_id, "driver service cleanup completed");
                Ok(Self::service_result(
                    serde_json::json!({"status": "cleaned"}),
                    typed.trace,
                ))
            }
            other => Err(ServiceError::UnsupportedCommand(format!(
                "unsupported Driver service command '{other}'"
            ))),
        }
    }

    async fn stop(&self) -> ServiceResult<()> {
        tracing::info!(service_id = %self.descriptor.id, "driver service provider stopped");
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        tracing::info!(service_id = %self.descriptor.id, "driver service provider cleanup completed");
        Ok(())
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        if self.runtime.is_some() {
            Ok(ServiceHealth::Healthy)
        } else {
            Ok(ServiceHealth::Unavailable {
                reason: "driver runtime is not configured".into(),
            })
        }
    }
}

async fn inventory_entries(runtime: &DriverRuntime) -> Vec<DriverInventoryEntry> {
    runtime
        .list_inventory()
        .await
        .into_iter()
        .map(|item| DriverInventoryEntry {
            name: item.manifest.name,
            version: Some(item.manifest.version),
            driver_type: format!("{:?}", item.manifest.driver_type),
            description: item.manifest.description,
            capabilities: item.manifest.capabilities,
            tool_count: item.tool_count,
            metadata: BTreeMap::new(),
        })
        .collect()
}

fn decode<T: serde::de::DeserializeOwned>(value: serde_json::Value) -> ServiceResult<T> {
    serde_json::from_value(value).map_err(|err| ServiceError::UnsupportedCommand(err.to_string()))
}

fn to_value<T: serde::Serialize>(value: T) -> ServiceResult<serde_json::Value> {
    serde_json::to_value(value).map_err(|err| ServiceError::AdapterFailure(err.to_string()))
}

fn service_adapter_error(err: MacacaError) -> ServiceError {
    ServiceError::AdapterFailure(err.to_string())
}
