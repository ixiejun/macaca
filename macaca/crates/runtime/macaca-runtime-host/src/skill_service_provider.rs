//! Runtime-host adapter for the Route C Skill Service.
//!
//! The provider bridges typed Skill service commands to existing skill runtime
//! facades.  It keeps Web and CLI from constructing skill runtimes directly
//! while preserving the old semantics through deprecated compatibility anchors.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::{
    CapabilityToolDescriptor, CapabilityToolInvocationResult, CapabilityToolOriginKind,
    CapabilityToolResourceScope, CleanupPolicy, MacacaError, ServiceCallResult, ServiceCommand,
    ServiceDescriptor, ServiceError, ServiceHealth, ServiceResult, TraceContext,
};
use macaca_skill::{
    skill_service_descriptor, ExecutableSkillToolSet, SkillExecutableLoadCommand,
    SkillExecutableLoadResult, SkillRuntimeFacade, SkillServiceSnapshot,
    SkillServiceSnapshotCommand, SkillSnapshotRequest, SkillSnapshotServiceCommand,
    SkillStatusCommand, SkillStatusResult, SkillToolCatalogCommand, SkillToolCatalogResult,
    SkillToolInvokeCommand, SKILL_CLEANUP_COMMAND, SKILL_EXECUTABLE_LOAD_COMMAND, SKILL_SERVICE_ID,
    SKILL_SERVICE_SNAPSHOT_COMMAND, SKILL_SNAPSHOT_COMMAND, SKILL_STATUS_COMMAND,
    SKILL_TOOL_CATALOG_COMMAND, SKILL_TOOL_INVOKE_COMMAND,
};
use macaca_tools::{ToolCommand, ToolCommandExecutor};
use tokio::sync::Mutex;

/// Host-owned Skill service provider backed by skill facades.
pub struct SkillSystemServiceProvider {
    descriptor: ServiceDescriptor,
    snapshot_facade: Option<SkillRuntimeFacade>,
    executable_tools: Arc<Mutex<ExecutableSkillToolSet>>,
}

impl SkillSystemServiceProvider {
    /// Create a provider with the default skill runtime facades.
    pub fn new() -> Self {
        Self {
            descriptor: skill_service_descriptor(),
            snapshot_facade: Some(SkillRuntimeFacade::new()),
            executable_tools: Arc::new(Mutex::new(ExecutableSkillToolSet::new())),
        }
    }

    /// Create a Null Object provider that returns structured unavailable.
    pub fn unavailable() -> Self {
        Self {
            descriptor: skill_service_descriptor(),
            snapshot_facade: None,
            executable_tools: Arc::new(Mutex::new(ExecutableSkillToolSet::new())),
        }
    }

    fn facade(&self) -> ServiceResult<SkillRuntimeFacade> {
        self.snapshot_facade.clone().ok_or_else(|| {
            ServiceError::ServiceUnavailable("skill runtime is not configured".into())
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

impl Default for SkillSystemServiceProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SystemService for SkillSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        tracing::info!(
            service_id = %self.descriptor.id,
            configured = self.snapshot_facade.is_some(),
            "skill service provider started"
        );
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = Self::trace(&command)?;
        tracing::info!(
            service_id = %self.descriptor.id,
            command = %command.name,
            trace_id = %trace.trace_id,
            "skill service command accepted"
        );
        match command.name.as_str() {
            SKILL_SNAPSHOT_COMMAND => {
                let typed: SkillSnapshotServiceCommand = decode(command.payload)?;
                let facade = self.facade()?;
                let mut builder = SkillSnapshotRequest::builder(typed.agent_name);
                if let Some(workspace_dir) = typed.workspace_dir {
                    builder = builder.workspace_dir(Some(workspace_dir));
                }
                if let Some(app_dir) = typed.app_dir {
                    builder = builder.app_dir(Some(app_dir));
                }
                builder = builder.policy(typed.exposure_policy);
                let snapshot = facade
                    .build_snapshot(builder.build())
                    .await
                    .map_err(service_adapter_error)?;
                tracing::info!(
                    trace_id = %typed.trace.trace_id,
                    skills = snapshot.skills.len(),
                    "skill service snapshot completed"
                );
                Ok(Self::service_result(to_value(snapshot)?, typed.trace))
            }
            SKILL_EXECUTABLE_LOAD_COMMAND => {
                let typed: SkillExecutableLoadCommand = decode(command.payload)?;
                let mut loaded = 0usize;
                let mut failed = 0usize;
                let mut failures = Vec::new();
                let mut guard = self.executable_tools.lock().await;
                for dir in typed.directories {
                    match guard.load_from_directory(&dir).await {
                        Ok(count) => loaded += count,
                        Err(err) => {
                            failed += 1;
                            failures.push(err.to_string());
                        }
                    }
                }
                let result = SkillExecutableLoadResult {
                    loaded,
                    failed,
                    skipped: 0,
                    captured_at: chrono::Utc::now(),
                    failures,
                };
                tracing::info!(
                    trace_id = %typed.trace.trace_id,
                    loaded,
                    failed,
                    "skill service executable load completed"
                );
                Ok(Self::service_result(to_value(result)?, typed.trace))
            }
            SKILL_TOOL_CATALOG_COMMAND => {
                let typed: SkillToolCatalogCommand = decode(command.payload)?;
                let snapshot = self.executable_tools.lock().await.snapshot();
                let descriptors = snapshot
                    .skills
                    .into_iter()
                    .map(|skill| {
                        CapabilityToolDescriptor::new(
                            SKILL_SERVICE_ID,
                            "skill-runtime",
                            format!("skill.tool.{}", skill.name),
                            skill.name,
                            skill.description,
                            skill.parameters,
                            CapabilityToolOriginKind::Skill,
                        )
                        .map(|descriptor| {
                            descriptor.with_policy_hints(
                                vec!["skill.invoke".into()],
                                vec![CapabilityToolResourceScope::AgentSession],
                            )
                        })
                    })
                    .collect::<Result<Vec<_>, MacacaError>>()
                    .map_err(service_adapter_error)?;
                tracing::info!(
                    trace_id = %typed.trace.trace_id,
                    count = descriptors.len(),
                    "skill service tool catalog emitted"
                );
                Ok(Self::service_result(
                    to_value(SkillToolCatalogResult::new(descriptors))?,
                    typed.trace,
                ))
            }
            SKILL_TOOL_INVOKE_COMMAND => {
                let typed: SkillToolInvokeCommand = decode(command.payload)?;
                let tool = self
                    .executable_tools
                    .lock()
                    .await
                    .tool(&typed.invocation.tool_name)
                    .map_err(service_adapter_error)?;
                tracing::info!(
                    trace_id = %typed.invocation.trace.trace_id,
                    tool = %typed.invocation.tool_name,
                    "skill service invoking tool"
                );
                let output = ToolCommandExecutor::execute_command(
                    &tool,
                    ToolCommand::new(typed.invocation.input.clone()),
                )
                .await
                .map_err(service_adapter_error)?;
                let result = CapabilityToolInvocationResult::ok(
                    SKILL_SERVICE_ID,
                    CapabilityToolOriginKind::Skill,
                    typed.invocation.tool_name,
                    output,
                    typed.invocation.trace.clone(),
                );
                Ok(Self::service_result(
                    to_value(result)?,
                    typed.invocation.trace,
                ))
            }
            SKILL_STATUS_COMMAND => {
                let typed: SkillStatusCommand = decode(command.payload)?;
                let registry = self.executable_tools.lock().await.snapshot();
                let result = SkillStatusResult {
                    service_id: SKILL_SERVICE_ID.into(),
                    healthy: self.snapshot_facade.is_some(),
                    snapshot_skills: 0,
                    executable_skills: registry.len(),
                    captured_at: chrono::Utc::now(),
                };
                Ok(Self::service_result(to_value(result)?, typed.trace))
            }
            SKILL_SERVICE_SNAPSHOT_COMMAND => {
                let typed: SkillServiceSnapshotCommand = decode(command.payload)?;
                let registry = self.executable_tools.lock().await.snapshot();
                let snapshot =
                    SkillServiceSnapshot::new(None, Some(registry.clone()), registry.len());
                tracing::info!(trace_id = %typed.trace.trace_id, "skill service snapshot emitted");
                Ok(Self::service_result(to_value(snapshot)?, typed.trace))
            }
            SKILL_CLEANUP_COMMAND => {
                tracing::info!(trace_id = %trace.trace_id, "skill service cleanup completed");
                Ok(Self::service_result(
                    serde_json::json!({"status": "cleaned"}),
                    trace,
                ))
            }
            other => Err(ServiceError::UnsupportedCommand(format!(
                "unsupported Skill service command '{other}'"
            ))),
        }
    }

    async fn stop(&self) -> ServiceResult<()> {
        tracing::info!(service_id = %self.descriptor.id, "skill service provider stopped");
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        tracing::info!(service_id = %self.descriptor.id, "skill service provider cleanup completed");
        Ok(())
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        if self.snapshot_facade.is_some() {
            Ok(ServiceHealth::Healthy)
        } else {
            Ok(ServiceHealth::Unavailable {
                reason: "skill runtime is not configured".into(),
            })
        }
    }
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
