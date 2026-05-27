//! Runtime-host provider for the Tool Capability Planning service.
//!
//! This provider owns planning orchestration only.  It decodes typed
//! `service.tool` commands, delegates deterministic plan construction to
//! `ToolPlanningService`, stores bounded snapshots, and routes invocation
//! commands through owning services without stealing provider lifecycle.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::{
    tool_service_descriptor, CleanupPolicy, KernelServiceId, MacacaError, MacacaResult,
    ServiceCallResult, ServiceCommand, ServiceError, ServiceHealth, ServiceResult,
    ToolCatalogPlanCommand, ToolGenericTraceCommand, TraceContext, TOOL_ARTIFACT_OPEN_COMMAND,
    TOOL_AUDIT_QUERY_COMMAND, TOOL_CATALOG_PLAN_COMMAND, TOOL_CATALOG_SNAPSHOT_COMMAND,
    TOOL_INVOCATION_STATUS_COMMAND, TOOL_INVOKE_CANCEL_COMMAND, TOOL_INVOKE_COMMAND,
    TOOL_POLICY_EXPLAIN_COMMAND, TOOL_PROVIDER_HEALTH_COMMAND, TOOL_PROVIDER_STATUS_COMMAND,
    TOOL_RESULT_GET_COMMAND, TOOL_SERVICE_ID, TOOL_TOOLSET_RESOLVE_COMMAND,
};

use crate::tool_service_invocation::ToolInvocationService;
use crate::tool_service_planning::ToolPlanningService;
use crate::tool_service_provider_state::ToolServiceProviderState;
use crate::{
    ServiceProviderFactoryContext, ServiceProviderInstance, ServiceRuntime,
    StaticServiceProviderFactory,
};

pub struct ToolSystemServiceProvider {
    planner: Arc<ToolPlanningService>,
    state: Arc<ToolServiceProviderState>,
    invocation: ToolInvocationService,
}

impl ToolSystemServiceProvider {
    pub fn new(planner: Arc<ToolPlanningService>, runtime: Arc<ServiceRuntime>) -> Self {
        let state = Arc::new(ToolServiceProviderState::default());
        state.set_provider_count(planner.contributor_count());
        Self {
            planner,
            invocation: ToolInvocationService::new(runtime, Arc::clone(&state)),
            state,
        }
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

/// Register and start `service.tool` inside the host-owned ServiceRuntime.
///
/// This helper is the approved Abstract Factory composition point for the
/// planning provider.  Callers provide an already-assembled planner so concrete
/// provider discovery remains outside the microkernel, SDK, Web shell, and
/// applications.
pub async fn bootstrap_tool_planning_service(
    runtime: Arc<ServiceRuntime>,
    planner: Arc<ToolPlanningService>,
    trace_id: impl Into<String>,
) -> MacacaResult<KernelServiceId> {
    let service: Arc<dyn SystemService> = Arc::new(ToolSystemServiceProvider::new(
        planner,
        Arc::clone(&runtime),
    ));
    let descriptor = service.descriptor();
    let service_id = descriptor.id.clone();
    let trace = TraceContext::new(trace_id);
    tracing::info!(
        service_id = %service_id,
        trace_id = %trace.trace_id,
        "tool planning service registering provider"
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
    tracing::info!(
        service_id = %service_id,
        trace_id = %trace.trace_id,
        "tool planning service provider started"
    );
    Ok(service_id)
}

#[async_trait]
impl SystemService for ToolSystemServiceProvider {
    fn descriptor(&self) -> macaca_proto::ServiceDescriptor {
        tool_service_descriptor()
    }

    async fn start(&self) -> ServiceResult<()> {
        tracing::info!(
            service_id = TOOL_SERVICE_ID,
            "tool service provider started"
        );
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = Self::trace(&command)?;
        tracing::info!(
            service_id = TOOL_SERVICE_ID,
            command = %command.name,
            trace_id = %trace.trace_id,
            "tool service command accepted"
        );
        match command.name.as_str() {
            TOOL_CATALOG_PLAN_COMMAND => {
                let typed: ToolCatalogPlanCommand = decode(command.payload)?;
                let result = self
                    .planner
                    .plan(typed.clone())
                    .await
                    .map_err(|err| ServiceError::AdapterFailure(err.to_string()))?;
                let provider_count = result
                    .metadata
                    .get("tool.contributor_count")
                    .and_then(|value| value.parse::<usize>().ok())
                    .unwrap_or_default();
                self.state.record_plan(result.clone(), provider_count);
                Ok(Self::service_result(to_value(result)?, typed.trace))
            }
            TOOL_CATALOG_SNAPSHOT_COMMAND | TOOL_PROVIDER_STATUS_COMMAND => {
                let typed: ToolGenericTraceCommand = decode(command.payload)?;
                Ok(Self::service_result(
                    to_value(self.state.snapshot(typed.trace.clone()))?,
                    typed.trace,
                ))
            }
            TOOL_TOOLSET_RESOLVE_COMMAND => {
                let typed: ToolGenericTraceCommand = decode(command.payload)?;
                let mut plan_command = ToolCatalogPlanCommand::new(typed.trace.clone())
                    .map_err(|err| ServiceError::InvalidArgument(err.to_string()))?;
                plan_command.include_hidden = true;
                if let Some(toolsets) = typed.metadata.get("toolsets") {
                    plan_command.requested_toolsets = toolsets
                        .split(',')
                        .map(str::trim)
                        .filter(|item| !item.is_empty())
                        .map(macaca_proto::ToolsetRef::new)
                        .collect::<MacacaResult<Vec<_>>>()
                        .map_err(|err| ServiceError::InvalidArgument(err.to_string()))?;
                }
                if let Some(families) = typed.metadata.get("families") {
                    plan_command.requested_families = families
                        .split(',')
                        .map(str::trim)
                        .filter(|item| !item.is_empty())
                        .map(macaca_proto::ToolFamilyRef::new)
                        .collect::<MacacaResult<Vec<_>>>()
                        .map_err(|err| ServiceError::InvalidArgument(err.to_string()))?;
                }
                let result = self
                    .planner
                    .plan(plan_command)
                    .await
                    .map_err(|err| ServiceError::AdapterFailure(err.to_string()))?;
                let provider_count = result
                    .metadata
                    .get("tool.contributor_count")
                    .and_then(|value| value.parse::<usize>().ok())
                    .unwrap_or_else(|| self.planner.contributor_count());
                self.state.record_plan(result.clone(), provider_count);
                Ok(Self::service_result(to_value(result)?, typed.trace))
            }
            TOOL_PROVIDER_HEALTH_COMMAND => {
                let typed: ToolGenericTraceCommand = decode(command.payload)?;
                let provider_count = self.state.provider_count();
                let health = self.state.provider_health(
                    typed.trace.clone(),
                    if provider_count == 0 {
                        ServiceHealth::Unavailable {
                            reason: "no tool providers registered".into(),
                        }
                    } else {
                        ServiceHealth::Healthy
                    },
                    provider_count,
                );
                Ok(Self::service_result(to_value(health)?, typed.trace))
            }
            TOOL_AUDIT_QUERY_COMMAND => {
                let typed: ToolGenericTraceCommand = decode(command.payload)?;
                Ok(Self::service_result(
                    to_value(self.state.audit_query(typed.trace.clone()))?,
                    typed.trace,
                ))
            }
            TOOL_POLICY_EXPLAIN_COMMAND => {
                let typed: ToolGenericTraceCommand = decode(command.payload)?;
                Ok(Self::service_result(
                    to_value(self.state.policy_explain(typed.trace.clone()))?,
                    typed.trace,
                ))
            }
            TOOL_INVOKE_COMMAND => {
                let typed: macaca_proto::ToolInvokeCommand = decode(command.payload)?;
                let trace = typed.trace.clone();
                let result = self
                    .invocation
                    .invoke(typed)
                    .await
                    .map_err(|err| ServiceError::AdapterFailure(err.to_string()))?;
                Ok(Self::service_result(to_value(result)?, trace))
            }
            TOOL_INVOKE_CANCEL_COMMAND => {
                let typed: macaca_proto::ToolInvokeCancelCommand = decode(command.payload)?;
                Ok(Self::service_result(
                    to_value(
                        self.invocation
                            .cancel(typed.trace.clone(), typed.invocation_ref),
                    )?,
                    typed.trace,
                ))
            }
            TOOL_INVOCATION_STATUS_COMMAND => {
                let typed: macaca_proto::ToolInvocationStatusCommand = decode(command.payload)?;
                Ok(Self::service_result(
                    to_value(
                        self.invocation
                            .status(typed.trace.clone(), typed.invocation_ref),
                    )?,
                    typed.trace,
                ))
            }
            TOOL_RESULT_GET_COMMAND => {
                let typed: macaca_proto::ToolResultGetCommand = decode(command.payload)?;
                Ok(Self::service_result(
                    to_value(
                        self.invocation
                            .result_get(typed.trace.clone(), typed.invocation_ref),
                    )?,
                    typed.trace,
                ))
            }
            TOOL_ARTIFACT_OPEN_COMMAND => {
                let typed: macaca_proto::ToolArtifactOpenCommand = decode(command.payload)?;
                Ok(Self::service_result(
                    to_value(
                        self.invocation
                            .artifact_open(typed.trace.clone(), typed.artifact_ref),
                    )?,
                    typed.trace,
                ))
            }
            other => Err(ServiceError::UnsupportedCommand(other.into())),
        }
    }

    async fn stop(&self) -> ServiceResult<()> {
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        Ok(())
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        Ok(ServiceHealth::Healthy)
    }
}

fn runtime_error(error: crate::ServiceRuntimeError) -> MacacaError {
    MacacaError::Config(error.to_string())
}

fn decode<T: serde::de::DeserializeOwned>(value: serde_json::Value) -> ServiceResult<T> {
    serde_json::from_value(value).map_err(|err| ServiceError::InvalidArgument(err.to_string()))
}

fn to_value<T: serde::Serialize>(value: T) -> ServiceResult<serde_json::Value> {
    serde_json::to_value(value).map_err(|err| ServiceError::AdapterFailure(err.to_string()))
}
