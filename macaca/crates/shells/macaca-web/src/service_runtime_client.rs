//! Web-local adapter from SDK service calls to `ServiceRuntime`.
//!
//! The SDK must not depend on `macaca-runtime-host`, otherwise Route C creates
//! a dependency cycle through framework/runtime-host integration.  This adapter
//! therefore lives in the host shell that actually owns a `ServiceRuntime`
//! instance.  It implements the SDK `SystemServiceClient` trait so focused SDK
//! clients can remain provider-neutral.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::{
    KernelServiceId, MacacaError, MacacaResult, ServiceBusSource, ServiceCommand,
    ServiceCommandName,
};
use macaca_sdk::runtime_host::ServiceRuntime;
use macaca_sdk::{
    ServiceCallCommand, ServiceCallResult, ServiceInspectionCommand, ServiceInspectionResult,
    SystemServiceClient,
};

/// Runtime-backed generic service client for the Web host.
#[derive(Clone)]
pub struct WebRuntimeSystemServiceClient {
    runtime: Arc<ServiceRuntime>,
    source: ServiceBusSource,
}

impl WebRuntimeSystemServiceClient {
    /// Create a Web service client bound to the host-owned service runtime.
    pub fn new(runtime: Arc<ServiceRuntime>, source: impl Into<String>) -> Self {
        Self {
            runtime,
            source: ServiceBusSource::new(source),
        }
    }
}

#[async_trait]
impl SystemServiceClient for WebRuntimeSystemServiceClient {
    async fn inspect_services(
        &self,
        command: &ServiceInspectionCommand,
    ) -> MacacaResult<ServiceInspectionResult> {
        let snapshot = self
            .runtime
            .snapshot()
            .map_err(|err| MacacaError::Config(err.to_string()))?;
        let services = snapshot
            .services
            .into_iter()
            .map(|service| service.descriptor.id.to_string())
            .collect();
        Ok(ServiceInspectionResult {
            scope: command.scope.clone(),
            services,
        })
    }

    async fn call_service(&self, command: &ServiceCallCommand) -> MacacaResult<ServiceCallResult> {
        let trace = command.trace.clone().ok_or_else(|| {
            MacacaError::Config("web runtime service client requires trace context".into())
        })?;
        tracing::info!(
            service_id = %command.service_id,
            command = %command.command_name,
            trace_id = %trace.trace_id,
            "web runtime service client dispatching service call"
        );
        let service_command = ServiceCommand::with_trace(
            ServiceCommandName::new(command.command_name.clone()),
            command.payload.clone(),
            trace,
        );
        let reply = self
            .runtime
            .call(
                &KernelServiceId::new(command.service_id.clone()),
                self.source.clone(),
                service_command,
            )
            .await
            .map_err(|err| MacacaError::Config(err.to_string()))?;
        if !reply.success {
            return Err(MacacaError::Config(format!(
                "service '{}' call '{}' failed: {}",
                command.service_id, command.command_name, reply.status
            )));
        }
        Ok(ServiceCallResult {
            service_id: command.service_id.clone(),
            output: reply.output.unwrap_or(serde_json::Value::Null),
        })
    }
}
