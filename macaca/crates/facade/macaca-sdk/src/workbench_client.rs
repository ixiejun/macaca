//! Focused SDK facades for interactive workbench services.
//!
//! This module applies the Facade and Null Object patterns to the foundation
//! contracts in `macaca-proto::workbench`.  A focused client is bound to exactly
//! one service id and command catalog, but it still dispatches through the
//! generic `SystemServiceClient` boundary.  The SDK therefore remains a caller
//! facade: it never constructs filesystem providers, process runners, protocol
//! gateways, plugin managers, or application-specific workflows.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::{
    workbench, MacacaError, MacacaResult, TraceContext, WorkbenchCommand, WorkbenchCommandResult,
    WorkbenchCommandStatus, WorkbenchServiceDescriptor,
};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use tracing::{info, warn};

use crate::service_client::{ServiceCallCommand, SystemServiceClient};
use crate::system_facade::SystemFacade;

/// Focused SDK client for one workbench-family service.
#[async_trait]
pub trait SystemWorkbenchClient: Send + Sync {
    /// Return the service descriptor visible to shells and application code.
    fn descriptor(&self) -> &WorkbenchServiceDescriptor;

    /// Dispatch a trace-scoped command to the owning service.
    async fn call(
        &self,
        command_name: &'static str,
        command: WorkbenchCommand<Value>,
    ) -> MacacaResult<WorkbenchCommandResult<Value>>;

    /// Return a bounded health/snapshot view.
    async fn snapshot(&self, trace: TraceContext) -> MacacaResult<WorkbenchCommandResult<Value>>;
}

/// Null Object client for absent workbench providers.
#[derive(Debug, Clone)]
pub struct UnavailableWorkbenchServiceClient {
    descriptor: WorkbenchServiceDescriptor,
}

impl UnavailableWorkbenchServiceClient {
    /// Create an unavailable focused client for one service descriptor.
    pub fn new(descriptor: WorkbenchServiceDescriptor) -> Self {
        Self { descriptor }
    }

    fn unavailable_result(&self, trace: TraceContext) -> WorkbenchCommandResult<Value> {
        WorkbenchCommandResult::unavailable(
            trace,
            format!(
                "{} is unavailable through the SDK Null Object client",
                self.descriptor.base.id.as_str()
            ),
        )
    }
}

#[async_trait]
impl SystemWorkbenchClient for UnavailableWorkbenchServiceClient {
    fn descriptor(&self) -> &WorkbenchServiceDescriptor {
        &self.descriptor
    }

    async fn call(
        &self,
        command_name: &'static str,
        command: WorkbenchCommand<Value>,
    ) -> MacacaResult<WorkbenchCommandResult<Value>> {
        warn!(
            service_id = %self.descriptor.base.id.as_str(),
            command = %command_name,
            trace_id = %command.trace.trace_id,
            "sdk workbench client returning structured unavailable command result"
        );
        Ok(self.unavailable_result(command.trace))
    }

    async fn snapshot(&self, trace: TraceContext) -> MacacaResult<WorkbenchCommandResult<Value>> {
        info!(
            service_id = %self.descriptor.base.id.as_str(),
            trace_id = %trace.trace_id,
            "sdk workbench client returning structured unavailable snapshot"
        );
        Ok(self.unavailable_result(trace))
    }
}

/// Runtime-backed focused client implemented over the generic service facade.
#[derive(Clone)]
pub struct ServiceBackedWorkbenchClient {
    descriptor: WorkbenchServiceDescriptor,
    service: Arc<dyn SystemServiceClient>,
}

impl ServiceBackedWorkbenchClient {
    /// Create a focused service-backed client without constructing providers.
    pub fn new(
        descriptor: WorkbenchServiceDescriptor,
        service: Arc<dyn SystemServiceClient>,
    ) -> Self {
        Self {
            descriptor,
            service,
        }
    }
}

#[async_trait]
impl SystemWorkbenchClient for ServiceBackedWorkbenchClient {
    fn descriptor(&self) -> &WorkbenchServiceDescriptor {
        &self.descriptor
    }

    async fn call(
        &self,
        command_name: &'static str,
        command: WorkbenchCommand<Value>,
    ) -> MacacaResult<WorkbenchCommandResult<Value>> {
        call_service(
            &self.service,
            self.descriptor.base.id.as_str(),
            command_name,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn snapshot(&self, trace: TraceContext) -> MacacaResult<WorkbenchCommandResult<Value>> {
        let command = WorkbenchCommand::new(
            workbench::WorkbenchCommandScope::new(macaca_proto::ApplicationId::new(), "system")?,
            trace.clone(),
            "snapshot",
            Value::Null,
        )?;
        call_service(
            &self.service,
            self.descriptor.base.id.as_str(),
            "service.snapshot",
            trace,
            command,
        )
        .await
    }
}

/// Catalog of focused clients for every foundation workbench service.
///
/// Keeping the catalog data-driven avoids application-specific branching while
/// still giving shells and application framework code named entrypoints.
#[derive(Clone)]
pub struct WorkbenchClientCatalog<C> {
    pub interaction: C,
    pub app_protocol: C,
    pub file: C,
    pub process: C,
    pub sandbox: C,
    pub approval: C,
    pub hook: C,
    pub config: C,
    pub plugin_marketplace: C,
    pub code_intelligence: C,
    pub git: C,
    pub review: C,
    pub diagnostics: C,
    pub realtime: C,
    pub remote_environment: C,
}

impl WorkbenchClientCatalog<UnavailableWorkbenchServiceClient> {
    /// Build a full Null Object catalog for hosts that have not installed
    /// workbench providers.  Every client returns structured unavailable.
    pub fn unavailable() -> Self {
        Self::from_descriptors(UnavailableWorkbenchServiceClient::new)
    }
}

impl WorkbenchClientCatalog<ServiceBackedWorkbenchClient> {
    /// Build focused clients over a caller-provided generic service client.
    ///
    /// The SDK receives a dispatcher as a Strategy object.  Runtime-host
    /// composition roots remain responsible for provider construction.
    pub fn service_backed(service: Arc<dyn SystemServiceClient>) -> Self {
        Self::from_descriptors(|descriptor| {
            ServiceBackedWorkbenchClient::new(descriptor, Arc::clone(&service))
        })
    }
}

impl<C> WorkbenchClientCatalog<C> {
    fn from_descriptors(mut build: impl FnMut(WorkbenchServiceDescriptor) -> C) -> Self {
        Self {
            interaction: build(workbench::interaction::descriptor()),
            app_protocol: build(workbench::app_protocol::descriptor()),
            file: build(workbench::file::descriptor()),
            process: build(workbench::process::descriptor()),
            sandbox: build(workbench::sandbox::descriptor()),
            approval: build(workbench::approval::descriptor()),
            hook: build(workbench::hook::descriptor()),
            config: build(workbench::config_service::descriptor()),
            plugin_marketplace: build(workbench::plugin_marketplace::descriptor()),
            code_intelligence: build(workbench::code_intelligence::descriptor()),
            git: build(workbench::git::descriptor()),
            review: build(workbench::review::descriptor()),
            diagnostics: build(workbench::diagnostics::descriptor()),
            realtime: build(workbench::realtime::descriptor()),
            remote_environment: build(workbench::remote_environment::descriptor()),
        }
    }
}

async fn call_service<T, R>(
    service: &Arc<dyn SystemServiceClient>,
    service_id: &str,
    command_name: &str,
    trace: TraceContext,
    payload: T,
) -> MacacaResult<R>
where
    T: Serialize,
    R: DeserializeOwned,
{
    let command =
        ServiceCallCommand::new(service_id, command_name, serde_json::to_value(payload)?)?
            .with_trace(trace);
    let result = service.call_service(&command).await?;
    serde_json::from_value(result.output).map_err(MacacaError::from)
}

/// Return true when a result is the explicit Null Object unavailable state.
pub fn is_structured_unavailable(result: &WorkbenchCommandResult<Value>) -> bool {
    matches!(result.status, WorkbenchCommandStatus::Unavailable { .. })
}

/// Extension entrypoints that attach workbench clients to `SystemFacade`
/// without increasing the already large facade implementation file.
pub trait SystemWorkbenchFacadeExt {
    /// Build Null Object focused clients for all workbench services.
    fn unavailable_workbench_clients(
        &self,
    ) -> WorkbenchClientCatalog<UnavailableWorkbenchServiceClient>;

    /// Build service-backed focused workbench clients from an injected generic
    /// dispatcher.  The dispatcher is supplied by runtime-host composition
    /// roots; this SDK method only wraps it in focused facades.
    fn workbench_clients_from_service(
        service: Arc<dyn SystemServiceClient>,
    ) -> WorkbenchClientCatalog<ServiceBackedWorkbenchClient>;
}

impl<T, S, SV, TR, P, L, M, C, D, SK, MCP, A, ST, E, PMT, W3, EVM, SCH, HB> SystemWorkbenchFacadeExt
    for SystemFacade<T, S, SV, TR, P, L, M, C, D, SK, MCP, A, ST, E, PMT, W3, EVM, SCH, HB>
{
    fn unavailable_workbench_clients(
        &self,
    ) -> WorkbenchClientCatalog<UnavailableWorkbenchServiceClient> {
        WorkbenchClientCatalog::unavailable()
    }

    fn workbench_clients_from_service(
        service: Arc<dyn SystemServiceClient>,
    ) -> WorkbenchClientCatalog<ServiceBackedWorkbenchClient> {
        WorkbenchClientCatalog::service_backed(service)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_proto::{ApplicationId, WorkbenchCommandScope};

    #[tokio::test]
    async fn unavailable_focused_client_returns_structured_unavailable() {
        let catalog = WorkbenchClientCatalog::unavailable();
        let scope =
            WorkbenchCommandScope::new(ApplicationId::from_name("generic-test-app"), "session")
                .unwrap();
        let command = WorkbenchCommand::new(
            scope,
            TraceContext::new("trace-sdk-workbench-null"),
            "cmd",
            Value::Null,
        )
        .unwrap();

        let result = catalog.file.call("file.read", command).await.unwrap();

        assert!(is_structured_unavailable(&result));
        assert_eq!(catalog.file.descriptor().base.id.as_str(), "service.file");
    }

    #[tokio::test]
    async fn unavailable_catalog_exposes_every_focused_descriptor() {
        let catalog = WorkbenchClientCatalog::unavailable();
        let descriptors = [
            catalog.interaction.descriptor(),
            catalog.app_protocol.descriptor(),
            catalog.file.descriptor(),
            catalog.process.descriptor(),
            catalog.sandbox.descriptor(),
            catalog.approval.descriptor(),
            catalog.hook.descriptor(),
            catalog.config.descriptor(),
            catalog.plugin_marketplace.descriptor(),
            catalog.code_intelligence.descriptor(),
            catalog.git.descriptor(),
            catalog.review.descriptor(),
            catalog.diagnostics.descriptor(),
            catalog.realtime.descriptor(),
            catalog.remote_environment.descriptor(),
        ];

        assert_eq!(descriptors.len(), 15);
        assert!(descriptors
            .iter()
            .all(|descriptor| !descriptor.commands.is_empty()));
    }
}
