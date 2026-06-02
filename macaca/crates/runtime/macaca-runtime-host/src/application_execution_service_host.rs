//! Service-backed Application ABI host for application execution.
//!
//! This Adapter connects the generic `MacacaHosted` application-execution
//! provider to the already service-owned Application Host dispatch command.
//! It deliberately does not know about WASM bytes, app names, UI state,
//! workflows, or product behavior.  The adapter only translates an
//! `ApplicationHostCommand` into the provider-neutral
//! `service.application/application.host.dispatch` command so every dispatch
//! goes through the same trace-required ServiceRuntime, policy decorators,
//! service bus, and audit placeholders as other OS service calls.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_persist::EventLog;
use macaca_proto::{
    ApplicationAbiError, ApplicationHostCommand, ApplicationHostCommandResult,
    ApplicationHostDispatchServiceCommand, ApplicationId, ApplicationServiceScope, KernelServiceId,
    MacacaError, PackageRuntimeKind, ServiceBusSource, ServiceCommand, ServiceCommandName,
    APPLICATION_HOST_DISPATCH_COMMAND, APPLICATION_SERVICE_ID,
};
use tracing::{info, warn};
use uuid::Uuid;

use crate::application_execution_event_store::ApplicationExecutionEventStore;
use crate::application_execution_hosted::{
    ApplicationAbiHostedExecutionAdapter, MacacaHostedApplicationExecutionProvider,
};
use crate::application_execution_provider_registry::ApplicationExecutionProviderRegistry;
use crate::application_hosts::ApplicationHostRuntime;
use crate::ServiceRuntime;

/// Runtime-host owned Bridge from Application Execution to Application Service.
///
/// The type is intentionally small and cloneable.  It holds only the host
/// `ServiceRuntime` plus a source label used for service-bus audit records.
/// Concrete application runtime selection remains inside Application Service,
/// while this adapter provides the Strategy required by
/// `MacacaHostedApplicationExecutionProvider`.
#[derive(Clone)]
pub struct ServiceBackedApplicationHostRuntime {
    runtime: Arc<ServiceRuntime>,
    source: ServiceBusSource,
}

impl ServiceBackedApplicationHostRuntime {
    /// Build a service-backed host runtime adapter.
    ///
    /// The `source` string is a stable service-bus audit label, not an
    /// application or provider name.  It lets policy/audit decorators explain
    /// that the call originated from the application-execution service.
    pub fn new(runtime: Arc<ServiceRuntime>, source: impl Into<String>) -> Self {
        Self {
            runtime,
            source: ServiceBusSource::new(source),
        }
    }
}

/// Build the default provider registry for `service.application_execution`.
///
/// Runtime-host owns this Abstract Factory because provider strategy assembly
/// is an OS composition concern.  Presentation shells pass only generic host
/// infrastructure (`ServiceRuntime` and EventLog), while this helper wires the
/// existing `MacacaHosted` provider to Application Service through the
/// service-backed host adapter.  The function is app-agnostic and keeps
/// provider selection extensible for future external-backend or remote-agent
/// additions.
pub(crate) fn default_application_execution_provider_registry(
    runtime: Arc<ServiceRuntime>,
    event_log: Arc<EventLog>,
) -> Result<ApplicationExecutionProviderRegistry, MacacaError> {
    let event_store = ApplicationExecutionEventStore::new(event_log);
    let host_runtime = Arc::new(ServiceBackedApplicationHostRuntime::new(
        runtime,
        "service.application_execution",
    ));
    let hosted_adapter = Arc::new(ApplicationAbiHostedExecutionAdapter::new(host_runtime));
    let hosted_provider = Arc::new(MacacaHostedApplicationExecutionProvider::new(
        event_store,
        hosted_adapter,
        vec![macaca_proto::CapabilityId::new(
            "capability.application_execution",
        )],
    ));
    ApplicationExecutionProviderRegistry::new()
        .register(hosted_provider)
        .map_err(|error| MacacaError::Config(error.to_string()))
}

#[async_trait]
impl ApplicationHostRuntime for ServiceBackedApplicationHostRuntime {
    fn runtime_kind(&self) -> PackageRuntimeKind {
        PackageRuntimeKind::Custom("service_backed_application_host".into())
    }

    async fn dispatch(
        &self,
        command: ApplicationHostCommand,
    ) -> Result<ApplicationHostCommandResult, ApplicationAbiError> {
        let trace = command
            .trace
            .clone()
            .ok_or(ApplicationAbiError::MissingTraceContext)?;
        let app_id = command
            .payload
            .get("application_id")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                ApplicationAbiError::InvalidDeclaration(
                    "application.host.dispatch requires application_id in host payload".into(),
                )
            })?
            .parse::<Uuid>()
            .map(ApplicationId)
            .map_err(|error| {
                ApplicationAbiError::InvalidDeclaration(format!(
                    "application.host.dispatch application_id is invalid: {error}"
                ))
            })?;
        let session_id = command
            .payload
            .get("session_id")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                ApplicationAbiError::InvalidDeclaration(
                    "application.host.dispatch requires session_id in host payload".into(),
                )
            })?;
        let scope = ApplicationServiceScope::session(app_id, session_id).map_err(|error| {
            ApplicationAbiError::InvalidDeclaration(format!(
                "application.host.dispatch scope is invalid: {error}"
            ))
        })?;
        let service_payload = serde_json::to_value(ApplicationHostDispatchServiceCommand {
            trace: trace.clone(),
            scope,
            host_command: command,
        })
        .map_err(|error| {
            ApplicationAbiError::InvalidDeclaration(format!(
                "application.host.dispatch payload encoding failed: {error}"
            ))
        })?;
        let service_command = ServiceCommand::with_trace(
            ServiceCommandName::new(APPLICATION_HOST_DISPATCH_COMMAND),
            service_payload,
            trace.clone(),
        );

        info!(
            trace_id = %trace.trace_id,
            service_id = APPLICATION_SERVICE_ID,
            command = APPLICATION_HOST_DISPATCH_COMMAND,
            "application execution dispatching hosted command through Application Service"
        );
        let reply = self
            .runtime
            .call(
                &KernelServiceId::new(APPLICATION_SERVICE_ID),
                self.source.clone(),
                service_command,
            )
            .await
            .map_err(|error| {
                ApplicationAbiError::RuntimeUnavailable(format!(
                    "Application Service dispatch failed: {error}"
                ))
            })?;
        if !reply.success {
            warn!(
                trace_id = %trace.trace_id,
                status = %reply.status,
                "Application Service host dispatch returned an unsuccessful reply"
            );
            return Err(ApplicationAbiError::RuntimeUnavailable(format!(
                "Application Service host dispatch was unsuccessful: {}",
                reply.status
            )));
        }
        let output = reply.output.unwrap_or(serde_json::Value::Null);
        serde_json::from_value(output).map_err(|error| {
            ApplicationAbiError::InvalidDeclaration(format!(
                "Application Service host dispatch result decoding failed: {error}"
            ))
        })
    }
}
