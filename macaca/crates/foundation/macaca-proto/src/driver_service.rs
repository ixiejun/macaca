//! Provider-neutral Driver service contract.
//!
//! Driver lifecycle, inventory, catalog, invocation, and cleanup commands are
//! protocol contracts, not provider implementation details.  Keeping these DTOs
//! in `macaca-proto` lets SDK clients and runtime-host providers communicate
//! without the SDK depending on the concrete driver service crate.

use std::collections::BTreeMap;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    ApplicationId, CapabilityId, CapabilityToolDescriptor, CapabilityToolInvocation,
    CapabilityToolInvocationResult, CleanupPolicy, KernelServiceId, MacacaError, MacacaResult,
    ServiceCapability, ServiceDescriptor, ServiceHealth, ServiceScope, ServiceType, TraceContext,
    TraceSchemaRef,
};

/// Stable service id used by runtime-host registration and SDK clients.
pub const DRIVER_SERVICE_ID: &str = "service.driver";

/// Command names accepted by the Driver service provider adapter.
pub const DRIVER_LOAD_COMMAND: &str = "driver.load";
pub const DRIVER_RELOAD_COMMAND: &str = "driver.reload";
pub const DRIVER_INVENTORY_COMMAND: &str = "driver.inventory";
pub const DRIVER_TOOL_CATALOG_COMMAND: &str = "driver.tool.catalog";
pub const DRIVER_TOOL_INVOKE_COMMAND: &str = "driver.tool.invoke";
pub const DRIVER_STATUS_COMMAND: &str = "driver.status";
pub const DRIVER_SNAPSHOT_COMMAND: &str = "driver.service.snapshot";
pub const DRIVER_CLEANUP_COMMAND: &str = "driver.cleanup";

/// Driver runtime load intent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DriverLoadCommand {
    LoadAll,
    Reload,
}

/// Per-driver load status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DriverLoadStatus {
    Loaded,
    Failed,
}

/// Per-driver load report entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverLoadEntry {
    pub name: String,
    pub path: PathBuf,
    pub status: DriverLoadStatus,
    pub tool_count: Option<usize>,
    pub error: Option<String>,
}

/// Aggregate report for a driver load command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverLoadReport {
    pub command: DriverLoadCommand,
    pub loaded: usize,
    pub failed: usize,
    pub entries: Vec<DriverLoadEntry>,
}

/// Explicit scope for Driver service commands.
///
/// Inventory/status may operate at application scope, while tool invocation
/// additionally carries session/agent scope through `CapabilityToolInvocation`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DriverServiceScope {
    pub application_id: Option<ApplicationId>,
    pub session_id: Option<String>,
    pub agent_name: Option<String>,
}

impl DriverServiceScope {
    /// Build application-scoped metadata for inventory/status commands.
    pub fn application(application_id: ApplicationId) -> Self {
        Self {
            application_id: Some(application_id),
            session_id: None,
            agent_name: None,
        }
    }

    /// Build session-scoped metadata for cleanup and audit commands.
    pub fn session(
        application_id: ApplicationId,
        session_id: impl Into<String>,
        agent_name: impl Into<String>,
    ) -> MacacaResult<Self> {
        Ok(Self {
            application_id: Some(application_id),
            session_id: Some(non_empty(
                session_id.into(),
                "driver service requires session_id",
            )?),
            agent_name: Some(non_empty(
                agent_name.into(),
                "driver service requires agent_name",
            )?),
        })
    }
}

/// Policy hints interpreted by runtime decorators or driver provider strategy.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DriverServicePolicyHints {
    pub required_permissions: Vec<String>,
    pub metadata: BTreeMap<String, String>,
}

/// Command for initial driver loading.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverLoadServiceCommand {
    pub trace: TraceContext,
    pub scope: DriverServiceScope,
    pub clear_existing: bool,
    pub policy: DriverServicePolicyHints,
}

impl DriverLoadServiceCommand {
    /// Build a traced load command and reject untraceable dispatch.
    pub fn new(trace: TraceContext, clear_existing: bool) -> MacacaResult<Self> {
        validate_trace(&trace, "driver load command requires trace_id")?;
        Ok(Self {
            trace,
            scope: DriverServiceScope::default(),
            clear_existing,
            policy: DriverServicePolicyHints::default(),
        })
    }
}

/// Command for reading driver inventory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverInventoryCommand {
    pub trace: TraceContext,
    pub scope: DriverServiceScope,
    pub include_tools: bool,
}

/// Command for reading sanitized driver tool descriptors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverToolCatalogCommand {
    pub trace: TraceContext,
    pub scope: DriverServiceScope,
    pub include_disabled: bool,
}

/// Command for invoking one driver-owned tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverToolInvokeCommand {
    pub invocation: CapabilityToolInvocation,
}

/// Command for lightweight driver status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverStatusCommand {
    pub trace: TraceContext,
    pub scope: DriverServiceScope,
}

/// Command for deterministic Driver service snapshots.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverServiceSnapshotCommand {
    pub trace: TraceContext,
    pub scope: DriverServiceScope,
    pub include_inventory: bool,
}

/// Command for driver resource cleanup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverCleanupCommand {
    pub trace: TraceContext,
    pub scope: DriverServiceScope,
}

/// Sanitized inventory row for one loaded driver.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DriverInventoryEntry {
    pub name: String,
    pub version: Option<String>,
    pub driver_type: String,
    pub description: String,
    pub capabilities: Vec<String>,
    pub tool_count: usize,
    pub metadata: BTreeMap<String, String>,
}

/// Driver inventory result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DriverInventoryResult {
    pub entries: Vec<DriverInventoryEntry>,
    pub captured_at: DateTime<Utc>,
}

impl DriverInventoryResult {
    /// Wrap inventory rows with deterministic capture metadata.
    pub fn new(entries: Vec<DriverInventoryEntry>) -> Self {
        Self {
            entries,
            captured_at: Utc::now(),
        }
    }
}

/// Driver tool catalog result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DriverToolCatalogResult {
    pub tools: Vec<CapabilityToolDescriptor>,
    pub captured_at: DateTime<Utc>,
}

impl DriverToolCatalogResult {
    /// Build a sanitized catalog result.
    pub fn new(tools: Vec<CapabilityToolDescriptor>) -> Self {
        Self {
            tools,
            captured_at: Utc::now(),
        }
    }
}

/// Driver status result for routes and CLI status surfaces.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DriverStatusResult {
    pub service_id: String,
    pub healthy: bool,
    pub loaded: usize,
    pub failed: usize,
    pub captured_at: DateTime<Utc>,
}

/// Deterministic Driver service snapshot.
///
/// This Memento exposes counts and inventory metadata only; it must not include
/// credentials, environment, command secrets, or raw tool input/output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DriverServiceSnapshot {
    pub service_id: String,
    pub healthy: bool,
    pub inventory: Vec<DriverInventoryEntry>,
    pub tool_count: usize,
    pub last_audit_ids: Vec<String>,
    pub captured_at: DateTime<Utc>,
}

impl DriverServiceSnapshot {
    /// Build a healthy snapshot from current inventory.
    pub fn healthy(inventory: Vec<DriverInventoryEntry>, tool_count: usize) -> Self {
        Self {
            service_id: DRIVER_SERVICE_ID.into(),
            healthy: true,
            inventory,
            tool_count,
            last_audit_ids: Vec::new(),
            captured_at: Utc::now(),
        }
    }

    /// Build a structured unavailable snapshot for hosts without drivers.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            service_id: DRIVER_SERVICE_ID.into(),
            healthy: false,
            inventory: Vec::new(),
            tool_count: 0,
            last_audit_ids: vec![reason.into()],
            captured_at: Utc::now(),
        }
    }
}

/// Result returned by driver tool invocation.
pub type DriverToolInvokeResult = CapabilityToolInvocationResult;

/// Result returned by driver load/reload commands.
pub type DriverLoadServiceResult = DriverLoadReport;

/// Build the provider-neutral descriptor for the driver system service.
///
/// The descriptor belongs to the protocol contract because SDK clients and
/// runtime-host providers both need the same service identity, permissions, and
/// lifecycle vocabulary without depending on each other's implementation crate.
pub fn driver_service_descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new(DRIVER_SERVICE_ID),
        ServiceType::new("driver"),
        TraceSchemaRef::new("trace.system_service.driver.v1"),
    );
    descriptor.capabilities = vec![
        ServiceCapability::new(
            CapabilityId::new("capability.driver.execute"),
            "Executes external software operations through a traced driver boundary.",
        ),
        ServiceCapability::new(
            CapabilityId::new("capability.driver.inventory"),
            "Reports sanitized driver inventory without exposing provider internals.",
        ),
        ServiceCapability::new(
            CapabilityId::new("capability.driver.reload"),
            "Reloads driver providers through the service lifecycle boundary.",
        ),
        ServiceCapability::new(
            CapabilityId::new("capability.driver.status"),
            "Reports driver service health and sanitized provider status.",
        ),
        ServiceCapability::new(
            CapabilityId::new("capability.driver.cleanup"),
            "Cleans driver-owned resources through explicit lifecycle commands.",
        ),
    ];
    descriptor.health = ServiceHealth::Healthy;
    descriptor.supported_scopes = vec![
        ServiceScope::Application("*".into()),
        ServiceScope::Session("*".into()),
    ];
    descriptor.required_permissions = vec![
        "driver.execute".into(),
        "driver.inventory".into(),
        "driver.reload".into(),
        "driver.status".into(),
    ];
    descriptor.cleanup_policy = CleanupPolicy::Always;
    descriptor
}

fn validate_trace(trace: &TraceContext, message: &'static str) -> MacacaResult<()> {
    if trace.trace_id.trim().is_empty() {
        return Err(MacacaError::Config(message.into()));
    }
    Ok(())
}

fn non_empty(value: String, message: &'static str) -> MacacaResult<String> {
    let trimmed = value.trim().to_string();
    if trimmed.is_empty() {
        return Err(MacacaError::Config(message.into()));
    }
    Ok(trimmed)
}
