//! Execution Control system service provider.
//!
//! This provider is the Stage 2 Adapter around the built-in
//! `ExecutionControlRuntimeCapability`. The runtime capability remains the
//! single State/Memento owner for pause, resume, checkpoint, and snapshot
//! semantics; this file only exposes that state machine through the generic
//! `SystemService` boundary so `ServiceRuntime` decorators can enforce trace,
//! policy, resource, entitlement, and metering checks before side effects.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::{
    CleanupPolicy, ExecutionControlAwaitResumeCommand, ExecutionControlCancelWaitCommand,
    ExecutionControlCheckpointCommand, ExecutionControlCommandResult, ExecutionControlEvent,
    ExecutionControlPauseCommand, ExecutionControlRegisterExecutionCommand,
    ExecutionControlResolvePolicyCommand, ExecutionControlResumeCommand, ExecutionControlScope,
    ExecutionControlSnapshotCommand, ExecutionControlState, ExecutionControlStateCommand,
    KernelServiceId, ServiceCallResult, ServiceCapability, ServiceCommand, ServiceDescriptor,
    ServiceError, ServiceHealth, ServiceResult, ServiceScope, ServiceType, TraceContext,
    TraceSchemaRef, EXECUTION_CONTROL_AWAIT_RESUME_COMMAND, EXECUTION_CONTROL_CANCEL_WAIT_COMMAND,
    EXECUTION_CONTROL_QUERY_STATE_COMMAND, EXECUTION_CONTROL_RECORD_CHECKPOINT_COMMAND,
    EXECUTION_CONTROL_REGISTER_EXECUTION_COMMAND, EXECUTION_CONTROL_REQUEST_PAUSE_COMMAND,
    EXECUTION_CONTROL_REQUEST_RESUME_COMMAND, EXECUTION_CONTROL_RESOLVE_POLICY_COMMAND,
    EXECUTION_CONTROL_SERVICE_ID, EXECUTION_CONTROL_SNAPSHOT_COMMAND,
};

use crate::execution_control::ExecutionControlPolicyResolver;
use crate::execution_control_runtime::{
    ExecutionControlRuntimeCapability, ExecutionControlRuntimeSnapshot,
};

/// ServiceRuntime-facing provider for `service.execution_control`.
pub struct ExecutionControlSystemServiceProvider {
    descriptor: ServiceDescriptor,
    capability: Option<Arc<ExecutionControlRuntimeCapability>>,
}

impl ExecutionControlSystemServiceProvider {
    /// Create a provider backed by the built-in runtime capability.
    pub fn new(capability: Arc<ExecutionControlRuntimeCapability>) -> Self {
        Self {
            descriptor: execution_control_service_descriptor(),
            capability: Some(capability),
        }
    }

    /// Create a Null Object provider for hosts where execution control is absent.
    pub fn unavailable() -> Self {
        let mut descriptor = execution_control_service_descriptor();
        descriptor.health = ServiceHealth::Unavailable {
            reason: "execution control runtime capability is not configured".into(),
        };
        Self {
            descriptor,
            capability: None,
        }
    }

    fn capability(&self) -> ServiceResult<Arc<ExecutionControlRuntimeCapability>> {
        self.capability.clone().ok_or_else(|| {
            ServiceError::ServiceUnavailable(
                "execution control runtime capability is not configured".into(),
            )
        })
    }

    fn trace(command: &ServiceCommand) -> ServiceResult<TraceContext> {
        command
            .trace
            .clone()
            .ok_or(ServiceError::MissingTraceContext)
    }

    fn service_result(
        output: serde_json::Value,
        status: impl Into<String>,
        trace: TraceContext,
    ) -> ServiceCallResult {
        ServiceCallResult {
            output,
            trace,
            status: status.into(),
            metadata: BTreeMap::new(),
            cleanup_hint: Some(CleanupPolicy::None),
        }
    }

    fn command_result(
        result: ExecutionControlCommandResult,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        Ok(Self::service_result(
            to_value(result.clone())?,
            format!("{:?}", result.status).to_lowercase(),
            trace,
        ))
    }

    fn snapshot_result(
        snapshot: ExecutionControlRuntimeSnapshot,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        Ok(Self::service_result(to_value(snapshot)?, "ok", trace))
    }
}

#[async_trait]
impl SystemService for ExecutionControlSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        tracing::info!(
            service_id = %self.descriptor.id,
            configured = self.capability.is_some(),
            "execution control service provider started"
        );
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = Self::trace(&command)?;
        tracing::info!(
            service_id = %self.descriptor.id,
            command = %command.name,
            trace_id = %trace.trace_id,
            "execution control service command accepted"
        );
        match command.name.as_str() {
            EXECUTION_CONTROL_RESOLVE_POLICY_COMMAND => {
                let typed: ExecutionControlResolvePolicyCommand = decode(command.payload)?;
                let mut resolution = ExecutionControlPolicyResolver::resolve(
                    typed.app_default.as_ref(),
                    typed.command_override.as_ref(),
                );
                // Policy resolution is a first-class admission decision.  The
                // provider attaches sanitized evidence to the typed result so
                // adapters can persist the decision before they expose live
                // diagnostics or install any local pause/resume channels.
                let policy_event = policy_resolution_event(
                    &typed.scope,
                    &resolution.reason_code,
                    &resolution.metadata,
                );
                resolution.events.push(policy_event);
                tracing::info!(
                    trace_id = %typed.scope.trace.trace_id,
                    status = ?resolution.status,
                    reason_code = %resolution.reason_code,
                    "execution control policy resolved"
                );
                Ok(Self::service_result(
                    to_value(resolution.clone())?,
                    format!("{:?}", resolution.status).to_lowercase(),
                    trace,
                ))
            }
            EXECUTION_CONTROL_REGISTER_EXECUTION_COMMAND => {
                let typed: ExecutionControlRegisterExecutionCommand = decode(command.payload)?;
                let result = self
                    .capability()?
                    .register_execution(typed)
                    .map_err(adapter_error)?;
                Self::command_result(result, trace)
            }
            EXECUTION_CONTROL_REQUEST_PAUSE_COMMAND => {
                let typed: ExecutionControlPauseCommand = decode(command.payload)?;
                let result = self
                    .capability()?
                    .request_pause(typed)
                    .map_err(adapter_error)?;
                Self::command_result(result, trace)
            }
            EXECUTION_CONTROL_RECORD_CHECKPOINT_COMMAND => {
                let typed: ExecutionControlCheckpointCommand = decode(command.payload)?;
                let result = self
                    .capability()?
                    .record_checkpoint(typed)
                    .map_err(adapter_error)?;
                Self::command_result(result, trace)
            }
            EXECUTION_CONTROL_AWAIT_RESUME_COMMAND => {
                let typed: ExecutionControlAwaitResumeCommand = decode(command.payload)?;
                let result = self
                    .capability()?
                    .await_resume(typed)
                    .map_err(adapter_error)?;
                Self::command_result(result, trace)
            }
            EXECUTION_CONTROL_REQUEST_RESUME_COMMAND => {
                let typed: ExecutionControlResumeCommand = decode(command.payload)?;
                let result = self
                    .capability()?
                    .request_resume(typed)
                    .map_err(adapter_error)?;
                Self::command_result(result, trace)
            }
            EXECUTION_CONTROL_CANCEL_WAIT_COMMAND => {
                let typed: ExecutionControlCancelWaitCommand = decode(command.payload)?;
                let result = self
                    .capability()?
                    .cancel_wait(typed)
                    .map_err(adapter_error)?;
                Self::command_result(result, trace)
            }
            EXECUTION_CONTROL_QUERY_STATE_COMMAND => {
                let typed: ExecutionControlStateCommand = decode(command.payload)?;
                let result = self
                    .capability()?
                    .query_state(typed)
                    .map_err(adapter_error)?;
                Self::command_result(result, trace)
            }
            EXECUTION_CONTROL_SNAPSHOT_COMMAND => {
                let _typed: ExecutionControlSnapshotCommand = decode(command.payload)?;
                Self::snapshot_result(self.capability()?.snapshot(), trace)
            }
            other => Err(ServiceError::UnsupportedCommand(format!(
                "unsupported execution control command {other}"
            ))),
        }
    }

    async fn stop(&self) -> ServiceResult<()> {
        tracing::info!(service_id = %self.descriptor.id, "execution control service stopped");
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        tracing::info!(service_id = %self.descriptor.id, "execution control cleanup completed");
        Ok(())
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        if self.capability.is_some() {
            Ok(ServiceHealth::Healthy)
        } else {
            Ok(ServiceHealth::Unavailable {
                reason: "execution control runtime capability is not configured".into(),
            })
        }
    }
}

/// Build the provider-neutral descriptor for the Execution Control service.
pub fn execution_control_service_descriptor() -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new(EXECUTION_CONTROL_SERVICE_ID),
        ServiceType::new("execution.control"),
        TraceSchemaRef::new("trace.system_service.execution_control.v1"),
    );
    descriptor.capabilities = vec![
        ServiceCapability::new(
            macaca_proto::CapabilityId::new("capability.execution_control.policy"),
            "Resolves application defaults and command overrides deterministically.",
        ),
        ServiceCapability::new(
            macaca_proto::CapabilityId::new("capability.execution_control.state"),
            "Registers execution-control state transitions and bounded snapshots.",
        ),
        ServiceCapability::new(
            macaca_proto::CapabilityId::new("capability.execution_control.pause_resume"),
            "Requests pause and resume transitions through typed strategies.",
        ),
        ServiceCapability::new(
            macaca_proto::CapabilityId::new("capability.execution_control.checkpoint"),
            "Records sanitized checkpoint references for replay and audit.",
        ),
    ];
    descriptor.health = ServiceHealth::Healthy;
    descriptor.supported_scopes = vec![
        ServiceScope::Application("*".into()),
        ServiceScope::Session("*".into()),
    ];
    descriptor.required_permissions = vec![
        "execution_control.resolve_policy".into(),
        "execution_control.register".into(),
        "execution_control.pause".into(),
        "execution_control.resume".into(),
        "execution_control.snapshot".into(),
    ];
    descriptor.cleanup_policy = CleanupPolicy::None;
    descriptor
}

fn decode<T>(payload: serde_json::Value) -> ServiceResult<T>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_value(payload).map_err(|err| ServiceError::UnsupportedCommand(err.to_string()))
}

fn to_value<T>(value: T) -> ServiceResult<serde_json::Value>
where
    T: serde::Serialize,
{
    serde_json::to_value(value).map_err(|err| ServiceError::AdapterFailure(err.to_string()))
}

fn adapter_error(error: macaca_proto::MacacaError) -> ServiceError {
    ServiceError::AdapterFailure(error.to_string())
}

fn policy_resolution_event(
    scope: &ExecutionControlScope,
    reason_code: &str,
    metadata: &BTreeMap<String, String>,
) -> ExecutionControlEvent {
    let mut event_metadata = scope.metadata.clone();
    event_metadata.extend(metadata.clone());
    ExecutionControlEvent {
        execution_id: scope.execution_id.clone(),
        state: ExecutionControlState::Running,
        reason_code: reason_code.into(),
        trace: scope.trace.clone(),
        metadata: sanitize_metadata(&event_metadata),
    }
}

fn sanitize_metadata(metadata: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    metadata
        .iter()
        .filter(|(key, _)| !is_sensitive_key(key))
        .map(|(key, value)| (key.clone(), sanitize_value(value)))
        .collect()
}

fn is_sensitive_key(key: &str) -> bool {
    let normalized = key.to_ascii_lowercase();
    [
        "secret",
        "token",
        "credential",
        "private_key",
        "prompt",
        "manifest",
        "payload",
        "wasm",
        "package_bytes",
        "signature",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

fn sanitize_value(value: &str) -> String {
    const MAX_VALUE_LEN: usize = 256;
    let trimmed = value.trim();
    if trimmed.chars().count() <= MAX_VALUE_LEN {
        return trimmed.to_string();
    }
    trimmed.chars().take(MAX_VALUE_LEN).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_proto::{
        ApplicationId, ExecutionControlCheckpointMode, ExecutionControlPolicy,
        ExecutionControlResolutionStatus, ExecutionControlResumeSource, ExecutionControlScope,
        ExecutionControlTrigger,
    };

    fn scope() -> ExecutionControlScope {
        ExecutionControlScope::new(
            ApplicationId::from_name("execution-control-test"),
            "session-a",
            "execution-a",
            TraceContext::new("trace-execution-control-provider"),
            "execution-control-provider-test",
        )
        .unwrap()
    }

    #[tokio::test]
    async fn provider_registers_execution_and_returns_snapshot() {
        let provider = ExecutionControlSystemServiceProvider::new(Arc::new(
            ExecutionControlRuntimeCapability::new(),
        ));
        let policy = macaca_proto::ExecutionControlResolvedPolicy {
            status: ExecutionControlResolutionStatus::Enabled,
            policy: Some(ExecutionControlPolicy::enabled(
                vec![ExecutionControlTrigger::tool_call_barrier("create_goal")],
                vec![ExecutionControlResumeSource::goal_lifecycle()],
                ExecutionControlCheckpointMode::ReferenceOnly,
            )),
            reason_code: "execution_control.enabled.test".into(),
            metadata: BTreeMap::new(),
            events: Vec::new(),
        };
        let register = ExecutionControlRegisterExecutionCommand {
            scope: scope(),
            policy,
        };

        let reply = provider
            .call(register.into_service_command().unwrap())
            .await
            .unwrap();
        let result: ExecutionControlCommandResult = serde_json::from_value(reply.output).unwrap();
        assert_eq!(result.status, ExecutionControlResolutionStatus::Enabled);

        let snapshot = ExecutionControlSnapshotCommand { scope: scope() };
        let reply = provider
            .call(snapshot.into_service_command().unwrap())
            .await
            .unwrap();
        let snapshot: ExecutionControlRuntimeSnapshot =
            serde_json::from_value(reply.output).unwrap();
        assert_eq!(snapshot.executions.len(), 1);
    }

    #[tokio::test]
    async fn provider_resolves_policy_with_full_effective_policy() {
        let provider = ExecutionControlSystemServiceProvider::new(Arc::new(
            ExecutionControlRuntimeCapability::new(),
        ));
        let app_policy = ExecutionControlPolicy::enabled(
            vec![
                ExecutionControlTrigger::tool_call_barrier("create_goal"),
                ExecutionControlTrigger::GoalLifecycle,
            ],
            vec![ExecutionControlResumeSource::goal_lifecycle()],
            ExecutionControlCheckpointMode::SnapshotReference,
        )
        .allow_command_overrides(true);
        let command_override = macaca_proto::ExecutionControlPolicyOverride::enable_for_run(
            vec![ExecutionControlTrigger::tool_call_barrier("create_goal")],
            vec![ExecutionControlResumeSource::goal_lifecycle()],
            ExecutionControlCheckpointMode::ReferenceOnly,
        );

        let reply = provider
            .call(
                ExecutionControlResolvePolicyCommand {
                    scope: scope(),
                    app_default: Some(app_policy),
                    command_override: Some(command_override),
                }
                .into_service_command()
                .unwrap(),
            )
            .await
            .unwrap();
        let resolved: macaca_proto::ExecutionControlResolvedPolicy =
            serde_json::from_value(reply.output).unwrap();

        assert_eq!(resolved.status, ExecutionControlResolutionStatus::Enabled);
        assert_eq!(resolved.policy.unwrap().triggers.len(), 1);
        assert_eq!(
            resolved.events.last().unwrap().reason_code,
            "execution_control.enabled.command_override"
        );
        assert_eq!(reply.status, "enabled");
    }

    #[tokio::test]
    async fn unavailable_provider_fails_structurally() {
        let provider = ExecutionControlSystemServiceProvider::unavailable();
        let policy = macaca_proto::ExecutionControlResolvedPolicy {
            status: ExecutionControlResolutionStatus::Enabled,
            policy: None,
            reason_code: "execution_control.enabled.test".into(),
            metadata: BTreeMap::new(),
            events: Vec::new(),
        };
        let err = provider
            .call(
                ExecutionControlRegisterExecutionCommand {
                    scope: scope(),
                    policy,
                }
                .into_service_command()
                .unwrap(),
            )
            .await
            .unwrap_err();
        assert!(matches!(err, ServiceError::ServiceUnavailable(_)));
    }
}
