//! SDK service client boundary for Route C system service inspection and calls.
//!
//! S3 defines the command/client shape before all concrete services are
//! migrated. Local compatibility implementations may report unavailable rather
//! than constructing providers. Later phases can bridge these commands to
//! `ServiceRuntime` or `ServiceBus` without changing Web/CLI contracts.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::{info, warn};

use macaca_proto::{MacacaError, MacacaResult};

/// Read-only service inspection command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceInspectionCommand {
    pub scope: String,
}

impl ServiceInspectionCommand {
    /// Create a scoped service inspection command.
    pub fn new(scope: impl Into<String>) -> MacacaResult<Self> {
        let scope = scope.into().trim().to_string();
        if scope.is_empty() {
            return Err(MacacaError::Config(
                "service inspection requires non-empty scope".into(),
            ));
        }
        Ok(Self { scope })
    }
}

/// Provider-neutral command for a future system service call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServiceCallCommand {
    pub service_id: String,
    pub command_name: String,
    pub payload: Value,
}

impl ServiceCallCommand {
    /// Create a service call command with explicit target and operation names.
    pub fn new(
        service_id: impl Into<String>,
        command_name: impl Into<String>,
        payload: Value,
    ) -> MacacaResult<Self> {
        let service_id = service_id.into().trim().to_string();
        let command_name = command_name.into().trim().to_string();
        if service_id.is_empty() || command_name.is_empty() {
            return Err(MacacaError::Config(
                "service call requires non-empty service_id and command_name".into(),
            ));
        }
        Ok(Self {
            service_id,
            command_name,
            payload,
        })
    }
}

/// Minimal inspection result for SDK-facing service views.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceInspectionResult {
    pub scope: String,
    pub services: Vec<String>,
}

/// Minimal service call result for future runtime-backed dispatch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServiceCallResult {
    pub service_id: String,
    pub output: Value,
}

/// Replaceable system service client.
#[async_trait]
pub trait SystemServiceClient: Send + Sync {
    /// Inspect service availability for a scope.
    async fn inspect_services(
        &self,
        command: &ServiceInspectionCommand,
    ) -> MacacaResult<ServiceInspectionResult>;

    /// Call a system service. Implementations may return structured unavailable.
    async fn call_service(&self, command: &ServiceCallCommand) -> MacacaResult<ServiceCallResult>;
}

/// Local placeholder client used before concrete services migrate.
#[derive(Debug, Default, Clone)]
pub struct UnavailableSystemServiceClient;

#[async_trait]
impl SystemServiceClient for UnavailableSystemServiceClient {
    async fn inspect_services(
        &self,
        command: &ServiceInspectionCommand,
    ) -> MacacaResult<ServiceInspectionResult> {
        info!(
            scope = %command.scope,
            "sdk service client returning empty local service inspection"
        );
        Ok(ServiceInspectionResult {
            scope: command.scope.clone(),
            services: Vec::new(),
        })
    }

    async fn call_service(&self, command: &ServiceCallCommand) -> MacacaResult<ServiceCallResult> {
        warn!(
            service_id = %command.service_id,
            command = %command.command_name,
            "sdk service client has no runtime-backed service dispatcher"
        );
        Err(MacacaError::Config(format!(
            "service '{}' is unavailable through the S3 SDK compatibility client",
            command.service_id
        )))
    }
}
