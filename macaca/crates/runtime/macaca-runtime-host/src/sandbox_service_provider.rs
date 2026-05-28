//! Runtime-host provider for `service.sandbox`.
//!
//! This provider owns service lifecycle, command dispatch, structured
//! unavailable behavior, trace-scoped result envelopes, and the tool bridge.
//! Concrete environment mechanics are delegated to a replaceable
//! [`SandboxProvider`] Strategy so Docker, SSH, browser, WASM, remote, mock, and
//! unavailable providers can enter through the same service contract.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::workbench::sandbox::*;
use macaca_proto::{
    BoundedSummary, CapabilityToolInvocationResult, CapabilityToolOriginKind, CleanupPolicy,
    ServiceCallResult, ServiceCommand, ServiceDescriptor, ServiceError, ServiceHealth,
    ServiceResult, ToolInvokeCommand, TraceContext, WorkbenchCommandResult, WorkbenchCommandStatus,
};
use tracing::{info, warn};

use crate::sandbox_service_local::{LocalSandboxProvider, SandboxProvider};
use crate::{
    ServiceProviderFactoryContext, ServiceProviderInstance, ServiceRuntime,
    StaticServiceProviderFactory,
};

pub struct SandboxSystemServiceProvider {
    descriptor: ServiceDescriptor,
    provider: Option<Arc<dyn SandboxProvider>>,
    unavailable_reason: Option<String>,
}

/// Register and start the built-in local `service.sandbox` provider.
///
/// Runtime-host is the approved composition root for service providers.  This
/// helper keeps sandbox provider construction out of kernel, SDK, shell, and
/// application code while still exposing a replaceable service boundary.
pub async fn bootstrap_local_sandbox_service(
    runtime: Arc<ServiceRuntime>,
    trace_id: impl Into<String>,
) -> macaca_proto::MacacaResult<macaca_proto::KernelServiceId> {
    let service: Arc<dyn SystemService> = Arc::new(SandboxSystemServiceProvider::local());
    let descriptor = service.descriptor();
    let service_id = descriptor.id.clone();
    let trace = TraceContext::new(trace_id);
    info!(
        service_id = %service_id,
        trace_id = %trace.trace_id,
        "sandbox service registering local provider"
    );
    runtime
        .register_provider(
            &StaticServiceProviderFactory::new(ServiceProviderInstance::new(descriptor, service)),
            ServiceProviderFactoryContext::new(),
        )
        .await
        .map_err(runtime_error)?;
    runtime
        .start(&service_id, trace.clone())
        .await
        .map_err(runtime_error)?;
    Ok(service_id)
}

impl SandboxSystemServiceProvider {
    /// Build the built-in local provider.  The provider records local workspace
    /// and local sandbox environments and returns structured unavailable for
    /// optional runtimes that are not installed.
    pub fn local() -> Self {
        Self::new(Arc::new(LocalSandboxProvider::default()))
    }

    /// Build a service around an injected provider Strategy.
    pub fn new(provider: Arc<dyn SandboxProvider>) -> Self {
        Self {
            descriptor: descriptor().base,
            provider: Some(provider),
            unavailable_reason: None,
        }
    }

    /// Build a Null Object provider for hosts without sandbox capabilities.
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            descriptor: descriptor().base,
            provider: None,
            unavailable_reason: Some(reason.into()),
        }
    }

    fn provider(&self) -> ServiceResult<Arc<dyn SandboxProvider>> {
        self.provider.clone().ok_or_else(|| {
            ServiceError::ServiceUnavailable(
                self.unavailable_reason
                    .clone()
                    .unwrap_or_else(|| "sandbox provider is not configured".into()),
            )
        })
    }

    fn trace(command: &ServiceCommand) -> ServiceResult<TraceContext> {
        command
            .trace
            .clone()
            .ok_or(ServiceError::MissingTraceContext)
    }

    fn result(
        response: SandboxServiceResponse,
        trace: TraceContext,
        status: WorkbenchCommandStatus,
        summary: Option<BoundedSummary>,
    ) -> ServiceResult<ServiceCallResult> {
        let result = WorkbenchCommandResult {
            trace: trace.clone(),
            status,
            output: Some(response),
            summary,
            audit_ref: Some(format!("sandbox:{}", trace.trace_id)),
            completed_at: chrono::Utc::now(),
        };
        Ok(ServiceCallResult {
            output: serde_json::to_value(result)
                .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?,
            trace,
            status: "ok".into(),
            metadata: BTreeMap::new(),
            cleanup_hint: Some(CleanupPolicy::OnStop),
        })
    }

    async fn unavailable_result(&self, trace: TraceContext) -> ServiceResult<ServiceCallResult> {
        Self::result(
            SandboxServiceResponse::EnvironmentHealth {
                records: Vec::new(),
            },
            trace,
            WorkbenchCommandStatus::Unavailable {
                reason: self
                    .unavailable_reason
                    .clone()
                    .unwrap_or_else(|| "sandbox provider is not configured".into()),
            },
            None,
        )
    }
}

#[async_trait]
impl SystemService for SandboxSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        info!(
            service_id = %self.descriptor.id,
            configured = self.provider.is_some(),
            "sandbox service provider started"
        );
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = Self::trace(&command)?;
        info!(
            service_id = %self.descriptor.id,
            command = %command.name,
            trace_id = %trace.trace_id,
            "sandbox service command accepted"
        );
        if self.provider.is_none() {
            warn!(
                service_id = %self.descriptor.id,
                trace_id = %trace.trace_id,
                "sandbox service unavailable provider returned structured unavailable"
            );
            return self.unavailable_result(trace).await;
        }
        match command.name.as_str() {
            PROFILE_LIST_COMMAND => Self::result(
                SandboxServiceResponse::ProfileList {
                    profiles: self
                        .provider()?
                        .list_profiles(decode(command.payload)?)
                        .await?,
                },
                trace,
                WorkbenchCommandStatus::Completed,
                None,
            ),
            PROFILE_RESOLVE_COMMAND => Self::result(
                SandboxServiceResponse::ProfileResolved {
                    profile: self
                        .provider()?
                        .resolve_profile(decode(command.payload)?)
                        .await?,
                },
                trace,
                WorkbenchCommandStatus::Completed,
                None,
            ),
            ENVIRONMENT_PREPARE_COMMAND => {
                let (record, status) = self
                    .provider()?
                    .prepare_environment(decode(command.payload)?, trace.trace_id.clone())
                    .await?;
                Self::result(
                    SandboxServiceResponse::EnvironmentPrepared { record },
                    trace,
                    status,
                    None,
                )
            }
            ENVIRONMENT_HEALTH_COMMAND => Self::result(
                SandboxServiceResponse::EnvironmentHealth {
                    records: self
                        .provider()?
                        .environment_health(decode(command.payload)?)
                        .await?,
                },
                trace,
                WorkbenchCommandStatus::Completed,
                None,
            ),
            ENVIRONMENT_CLEANUP_COMMAND => {
                let (record, released_lease_refs) = self
                    .provider()?
                    .cleanup_environment(decode(command.payload)?, trace.trace_id.clone())
                    .await?;
                Self::result(
                    SandboxServiceResponse::EnvironmentCleaned {
                        record,
                        released_lease_refs,
                    },
                    trace,
                    WorkbenchCommandStatus::Completed,
                    None,
                )
            }
            POLICY_EXPLAIN_COMMAND => Self::result(
                SandboxServiceResponse::PolicyExplanation {
                    decisions: self
                        .provider()?
                        .explain_policy(decode(command.payload)?)
                        .await?,
                },
                trace,
                WorkbenchCommandStatus::Completed,
                None,
            ),
            TOOL_INVOKE_COMMAND => self.tool_invoke(decode(command.payload)?).await,
            "service.snapshot" => Self::result(
                SandboxServiceResponse::EnvironmentHealth {
                    records: self
                        .provider()?
                        .environment_health(SandboxEnvironmentHealthRequest {
                            environment_id: None,
                        })
                        .await?,
                },
                trace,
                WorkbenchCommandStatus::Completed,
                None,
            ),
            other => Err(ServiceError::UnsupportedCommand(other.into())),
        }
    }

    async fn stop(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, "sandbox service provider stopped");
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        info!(service_id = %self.descriptor.id, "sandbox service cleanup completed");
        Ok(())
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        if let Some(reason) = &self.unavailable_reason {
            Ok(ServiceHealth::Unavailable {
                reason: reason.clone(),
            })
        } else {
            Ok(ServiceHealth::Healthy)
        }
    }
}

impl SandboxSystemServiceProvider {
    async fn tool_invoke(&self, command: ToolInvokeCommand) -> ServiceResult<ServiceCallResult> {
        let trace = command.trace.clone();
        let operation = command
            .input
            .get("operation")
            .and_then(|value| value.as_str())
            .unwrap_or("policy_explain");
        let result = match operation {
            "policy_explain" => {
                let request: SandboxPolicyExplainRequest =
                    serde_json::from_value(command.input.clone())
                        .map_err(|error| ServiceError::InvalidArgument(error.to_string()))?;
                let decisions = self.provider()?.explain_policy(request).await?;
                CapabilityToolInvocationResult::ok(
                    SERVICE_ID,
                    CapabilityToolOriginKind::Mcp,
                    command.tool_id,
                    serde_json::to_value(decisions)
                        .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?,
                    trace.clone(),
                )
            }
            _ => CapabilityToolInvocationResult::failed(
                SERVICE_ID,
                CapabilityToolOriginKind::Mcp,
                command.tool_id,
                "unsupported service.sandbox tool operation",
                trace.clone(),
            ),
        };
        Ok(ServiceCallResult {
            output: serde_json::to_value(result)
                .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?,
            trace,
            status: "ok".into(),
            metadata: BTreeMap::new(),
            cleanup_hint: Some(CleanupPolicy::None),
        })
    }
}

fn decode<T: serde::de::DeserializeOwned>(value: serde_json::Value) -> ServiceResult<T> {
    serde_json::from_value(value).map_err(|error| ServiceError::InvalidArgument(error.to_string()))
}

fn runtime_error(error: crate::ServiceRuntimeError) -> macaca_proto::MacacaError {
    macaca_proto::MacacaError::Config(error.to_string())
}
