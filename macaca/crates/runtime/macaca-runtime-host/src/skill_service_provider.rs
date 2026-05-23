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
    skill_service_descriptor, ExecutableSkillToolSet, SkillAliasResolveCommand,
    SkillAliasSnapshotCommand, SkillAliasUpsertCommand, SkillCurationDryRunCommand,
    SkillCurationLifecycleAction, SkillCurationLifecycleCommand, SkillExecutableLoadCommand,
    SkillExecutableLoadResult, SkillExperienceProposalCommand,
    SkillExperienceProposalSnapshotCommand, SkillGovernanceRecordUsageCommand,
    SkillGovernanceSnapshotCommand, SkillRuntimeFacade, SkillServiceSnapshot,
    SkillServiceSnapshotCommand, SkillSnapshotRequest, SkillSnapshotServiceCommand,
    SkillStatusCommand, SkillStatusResult, SkillToolCatalogCommand, SkillToolCatalogResult,
    SkillToolInvokeCommand, SKILL_ALIAS_RESOLVE_COMMAND, SKILL_ALIAS_SNAPSHOT_COMMAND,
    SKILL_ALIAS_UPSERT_COMMAND, SKILL_CLEANUP_COMMAND, SKILL_CURATION_ARCHIVE_COMMAND,
    SKILL_CURATION_DRY_RUN_COMMAND, SKILL_CURATION_PIN_COMMAND, SKILL_CURATION_RESTORE_COMMAND,
    SKILL_CURATION_UNPIN_COMMAND, SKILL_EVOLUTION_PROPOSE_FROM_TASK_COMMAND,
    SKILL_EVOLUTION_SNAPSHOT_COMMAND, SKILL_EXECUTABLE_LOAD_COMMAND,
    SKILL_GOVERNANCE_RECORD_USAGE_COMMAND, SKILL_GOVERNANCE_SNAPSHOT_COMMAND, SKILL_SERVICE_ID,
    SKILL_SERVICE_SNAPSHOT_COMMAND, SKILL_SNAPSHOT_COMMAND, SKILL_STATUS_COMMAND,
    SKILL_TOOL_CATALOG_COMMAND, SKILL_TOOL_INVOKE_COMMAND,
};
use macaca_tools::{ToolCommand, ToolCommandExecutor};
use tokio::sync::Mutex;

use crate::skill_service_provider_state::SkillProviderGovernanceState;

/// Host-owned Skill service provider backed by skill facades.
pub struct SkillSystemServiceProvider {
    descriptor: ServiceDescriptor,
    snapshot_facade: Option<SkillRuntimeFacade>,
    executable_tools: Arc<Mutex<ExecutableSkillToolSet>>,
    governance_state: Arc<SkillProviderGovernanceState>,
}

impl SkillSystemServiceProvider {
    /// Create a provider with the default skill runtime facades.
    pub fn new() -> Self {
        Self {
            descriptor: skill_service_descriptor(),
            snapshot_facade: Some(SkillRuntimeFacade::new()),
            executable_tools: Arc::new(Mutex::new(ExecutableSkillToolSet::new())),
            governance_state: Arc::new(SkillProviderGovernanceState::default()),
        }
    }

    /// Create a Null Object provider that returns structured unavailable.
    pub fn unavailable() -> Self {
        Self {
            descriptor: skill_service_descriptor(),
            snapshot_facade: None,
            executable_tools: Arc::new(Mutex::new(ExecutableSkillToolSet::new())),
            governance_state: Arc::new(SkillProviderGovernanceState::default()),
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

    /// Decode and apply one metadata-only lifecycle curation command.
    ///
    /// The runtime-host provider acts as the built-in Strategy behind the Skill
    /// service contract.  It logs the auditable boundary event and delegates the
    /// state transition to `SkillProviderGovernanceState`; it never edits skill
    /// instruction files, package bytes, aliases, or executable scripts.
    async fn apply_lifecycle_command(
        &self,
        payload: serde_json::Value,
        trace: TraceContext,
        action: SkillCurationLifecycleAction,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: SkillCurationLifecycleCommand = decode(payload)?;
        typed.validate().map_err(ServiceError::InvalidArgument)?;
        let result = self
            .governance_state
            .apply_lifecycle(typed.clone(), action.clone())
            .await
            .map_err(ServiceError::InvalidArgument)?;
        tracing::info!(
            trace_id = %typed.trace.trace_id,
            skill_id = %result.skill_id,
            action = ?action,
            lifecycle = ?result.lifecycle,
            pinned = result.pinned,
            mutated = result.mutated,
            "skill curation lifecycle metadata updated"
        );
        Ok(Self::service_result(to_value(result)?, trace))
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
            SKILL_GOVERNANCE_RECORD_USAGE_COMMAND => {
                let typed: SkillGovernanceRecordUsageCommand = decode(command.payload)?;
                let key = typed.observation.key();
                let event = typed.observation.event.clone();
                let result = self.governance_state.record_usage(typed).await;
                tracing::info!(
                    trace_id = %trace.trace_id,
                    skill_id = %key,
                    event = ?event,
                    "skill governance usage observation recorded"
                );
                Ok(Self::service_result(to_value(result)?, trace))
            }
            SKILL_GOVERNANCE_SNAPSHOT_COMMAND => {
                let typed: SkillGovernanceSnapshotCommand = decode(command.payload)?;
                let result = self
                    .governance_state
                    .governance_snapshot(typed.include_archived)
                    .await;
                tracing::info!(
                    trace_id = %typed.trace.trace_id,
                    records = result.records.len(),
                    "skill governance snapshot emitted"
                );
                Ok(Self::service_result(to_value(result)?, typed.trace))
            }
            SKILL_CURATION_DRY_RUN_COMMAND => {
                let typed: SkillCurationDryRunCommand = decode(command.payload)?;
                let result = self.governance_state.curation_dry_run(&typed).await;
                tracing::info!(
                    trace_id = %typed.trace.trace_id,
                    recommendations = result.recommendations.len(),
                    mutated = result.mutated,
                    "skill curation dry-run completed"
                );
                Ok(Self::service_result(to_value(result)?, typed.trace))
            }
            SKILL_CURATION_PIN_COMMAND => {
                self.apply_lifecycle_command(
                    command.payload,
                    trace,
                    SkillCurationLifecycleAction::Pin,
                )
                .await
            }
            SKILL_CURATION_UNPIN_COMMAND => {
                self.apply_lifecycle_command(
                    command.payload,
                    trace,
                    SkillCurationLifecycleAction::Unpin,
                )
                .await
            }
            SKILL_CURATION_ARCHIVE_COMMAND => {
                self.apply_lifecycle_command(
                    command.payload,
                    trace,
                    SkillCurationLifecycleAction::Archive,
                )
                .await
            }
            SKILL_CURATION_RESTORE_COMMAND => {
                self.apply_lifecycle_command(
                    command.payload,
                    trace,
                    SkillCurationLifecycleAction::Restore,
                )
                .await
            }
            SKILL_ALIAS_UPSERT_COMMAND => {
                let typed: SkillAliasUpsertCommand = decode(command.payload)?;
                let source_skill_id = typed.record.source_skill_id.clone();
                let target_skill_id = typed.record.target_skill_id.clone();
                let result = self.governance_state.upsert_alias(typed).await;
                tracing::info!(
                    trace_id = %trace.trace_id,
                    source_skill_id = %source_skill_id,
                    target_skill_id = %target_skill_id,
                    "skill alias record upserted"
                );
                Ok(Self::service_result(to_value(result)?, trace))
            }
            SKILL_ALIAS_RESOLVE_COMMAND => {
                let typed: SkillAliasResolveCommand = decode(command.payload)?;
                let result = self.governance_state.resolve_alias(&typed).await;
                tracing::info!(
                    trace_id = %typed.trace.trace_id,
                    skill_id = %typed.skill_id,
                    resolved = result.resolved,
                    "skill alias resolution completed"
                );
                Ok(Self::service_result(to_value(result)?, typed.trace))
            }
            SKILL_ALIAS_SNAPSHOT_COMMAND => {
                let typed: SkillAliasSnapshotCommand = decode(command.payload)?;
                let result = self.governance_state.alias_snapshot().await;
                tracing::info!(
                    trace_id = %typed.trace.trace_id,
                    aliases = result.aliases.len(),
                    "skill alias snapshot emitted"
                );
                Ok(Self::service_result(to_value(result)?, typed.trace))
            }
            SKILL_EVOLUTION_PROPOSE_FROM_TASK_COMMAND => {
                let typed: SkillExperienceProposalCommand = decode(command.payload)?;
                typed
                    .candidate
                    .validate()
                    .map_err(ServiceError::InvalidArgument)?;
                let result = self
                    .governance_state
                    .propose_experience(typed.clone())
                    .await;
                tracing::info!(
                    trace_id = %typed.trace.trace_id,
                    proposal_id = %result.proposal.proposal_id,
                    task_id = %result.proposal.task_id,
                    action = ?result.proposal.recommended_action,
                    mutated = result.mutated,
                    "skill experience proposal created"
                );
                Ok(Self::service_result(to_value(result)?, typed.trace))
            }
            SKILL_EVOLUTION_SNAPSHOT_COMMAND => {
                let typed: SkillExperienceProposalSnapshotCommand = decode(command.payload)?;
                let result = self.governance_state.experience_snapshot().await;
                tracing::info!(
                    trace_id = %typed.trace.trace_id,
                    proposals = result.proposals.len(),
                    mutated = result.mutated,
                    include_discarded = typed.include_discarded,
                    "skill experience proposal snapshot emitted"
                );
                Ok(Self::service_result(to_value(result)?, typed.trace))
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
