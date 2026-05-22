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
    skill_service_descriptor, ExecutableSkillToolSet, SkillCurationDryRunCommand,
    SkillCurationDryRunResult, SkillExecutableLoadCommand, SkillExecutableLoadResult,
    SkillGovernanceRecord, SkillGovernanceRecordUsageCommand, SkillGovernanceRecordUsageResult,
    SkillGovernanceSnapshotCommand, SkillGovernanceSnapshotResult, SkillRuntimeFacade,
    SkillServiceSnapshot, SkillServiceSnapshotCommand, SkillSnapshotRequest,
    SkillSnapshotServiceCommand, SkillStatusCommand, SkillStatusResult, SkillToolCatalogCommand,
    SkillToolCatalogResult, SkillToolInvokeCommand, SKILL_CLEANUP_COMMAND,
    SKILL_CURATION_DRY_RUN_COMMAND, SKILL_EXECUTABLE_LOAD_COMMAND,
    SKILL_GOVERNANCE_RECORD_USAGE_COMMAND, SKILL_GOVERNANCE_SNAPSHOT_COMMAND, SKILL_SERVICE_ID,
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
    governance_records: Arc<Mutex<BTreeMap<String, SkillGovernanceRecord>>>,
}

impl SkillSystemServiceProvider {
    /// Create a provider with the default skill runtime facades.
    pub fn new() -> Self {
        Self {
            descriptor: skill_service_descriptor(),
            snapshot_facade: Some(SkillRuntimeFacade::new()),
            executable_tools: Arc::new(Mutex::new(ExecutableSkillToolSet::new())),
            governance_records: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }

    /// Create a Null Object provider that returns structured unavailable.
    pub fn unavailable() -> Self {
        Self {
            descriptor: skill_service_descriptor(),
            snapshot_facade: None,
            executable_tools: Arc::new(Mutex::new(ExecutableSkillToolSet::new())),
            governance_records: Arc::new(Mutex::new(BTreeMap::new())),
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
            SKILL_GOVERNANCE_RECORD_USAGE_COMMAND => {
                let typed: SkillGovernanceRecordUsageCommand = decode(command.payload)?;
                let observed_at = chrono::Utc::now();
                let key = typed.observation.key();
                let mut records = self.governance_records.lock().await;
                let record = records
                    .entry(key.clone())
                    .and_modify(|record| record.apply(&typed.observation, observed_at))
                    .or_insert_with(|| {
                        SkillGovernanceRecord::from_observation(&typed.observation, observed_at)
                    })
                    .clone();
                tracing::info!(
                    trace_id = %typed.trace.trace_id,
                    skill_id = %key,
                    event = ?typed.observation.event,
                    "skill governance usage observation recorded"
                );
                Ok(Self::service_result(
                    to_value(SkillGovernanceRecordUsageResult {
                        record,
                        captured_at: observed_at,
                    })?,
                    typed.trace,
                ))
            }
            SKILL_GOVERNANCE_SNAPSHOT_COMMAND => {
                let typed: SkillGovernanceSnapshotCommand = decode(command.payload)?;
                let mut records: Vec<_> = self
                    .governance_records
                    .lock()
                    .await
                    .values()
                    .filter(|record| {
                        typed.include_archived
                            || record.lifecycle != macaca_skill::SkillLifecycleState::Archived
                    })
                    .cloned()
                    .collect();
                records.sort_by(|left, right| {
                    left.provenance.skill_id.cmp(&right.provenance.skill_id)
                });
                tracing::info!(
                    trace_id = %typed.trace.trace_id,
                    records = records.len(),
                    "skill governance snapshot emitted"
                );
                Ok(Self::service_result(
                    to_value(SkillGovernanceSnapshotResult {
                        records,
                        captured_at: chrono::Utc::now(),
                    })?,
                    typed.trace,
                ))
            }
            SKILL_CURATION_DRY_RUN_COMMAND => {
                let typed: SkillCurationDryRunCommand = decode(command.payload)?;
                let records = self
                    .governance_records
                    .lock()
                    .await
                    .values()
                    .cloned()
                    .collect::<Vec<_>>();
                let result =
                    SkillCurationDryRunResult::from_records(records, &typed, chrono::Utc::now());
                tracing::info!(
                    trace_id = %typed.trace.trace_id,
                    recommendations = result.recommendations.len(),
                    mutated = result.mutated,
                    "skill curation dry-run completed"
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

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_kernel::SystemService;
    use macaca_proto::{ServiceCommandName, TraceContext};
    use macaca_skill::{
        SkillAuthorKind, SkillCurationAction, SkillServiceScope, SkillUsageEventKind,
        SkillUsageObservation,
    };

    fn traced_command<T: serde::Serialize>(
        name: &str,
        payload: T,
        trace: TraceContext,
    ) -> ServiceCommand {
        ServiceCommand::with_trace(
            ServiceCommandName::new(name),
            serde_json::to_value(payload).expect("test command payload must serialize"),
            trace,
        )
    }

    fn observation(event: SkillUsageEventKind, pinned: Option<bool>) -> SkillUsageObservation {
        SkillUsageObservation {
            skill_id: "skill://agent/example".into(),
            name: "agent-example".into(),
            source: "test".into(),
            source_scope: "workspace".into(),
            event,
            author_kind: SkillAuthorKind::Agent,
            created_by: Some("agent".into()),
            pinned,
            evidence_id: Some("event-1".into()),
            metadata: BTreeMap::new(),
        }
    }

    #[tokio::test]
    async fn skill_governance_records_usage_and_snapshots_state() {
        let provider = SkillSystemServiceProvider::new();
        let trace = TraceContext::new("trace-skill-governance-record");
        let payload = SkillGovernanceRecordUsageCommand {
            trace: trace.clone(),
            scope: SkillServiceScope::default(),
            observation: observation(SkillUsageEventKind::Used, None),
        };

        let result = provider
            .call(traced_command(
                SKILL_GOVERNANCE_RECORD_USAGE_COMMAND,
                payload,
                trace.clone(),
            ))
            .await
            .expect("usage recording should succeed");
        let typed: SkillGovernanceRecordUsageResult =
            serde_json::from_value(result.output).expect("usage result should decode");

        assert_eq!(typed.record.provenance.name, "agent-example");
        assert_eq!(typed.record.telemetry.use_count, 1);
        assert_eq!(typed.record.provenance.author_kind, SkillAuthorKind::Agent);

        let snapshot_payload = SkillGovernanceSnapshotCommand {
            trace: trace.clone(),
            scope: SkillServiceScope::default(),
            include_archived: false,
        };
        let snapshot = provider
            .call(traced_command(
                SKILL_GOVERNANCE_SNAPSHOT_COMMAND,
                snapshot_payload,
                trace,
            ))
            .await
            .expect("snapshot should succeed");
        let snapshot: SkillGovernanceSnapshotResult =
            serde_json::from_value(snapshot.output).expect("snapshot result should decode");
        assert_eq!(snapshot.records.len(), 1);
        assert_eq!(snapshot.records[0].telemetry.use_count, 1);
    }

    #[tokio::test]
    async fn skill_governance_dry_run_keeps_pinned_skills_protected() {
        let provider = SkillSystemServiceProvider::new();
        let trace = TraceContext::new("trace-skill-curation-dry-run");
        let payload = SkillGovernanceRecordUsageCommand {
            trace: trace.clone(),
            scope: SkillServiceScope::default(),
            observation: observation(SkillUsageEventKind::Pinned, Some(true)),
        };
        provider
            .call(traced_command(
                SKILL_GOVERNANCE_RECORD_USAGE_COMMAND,
                payload,
                trace.clone(),
            ))
            .await
            .expect("pinned observation should succeed");

        let dry_run = SkillCurationDryRunCommand {
            trace: trace.clone(),
            scope: SkillServiceScope::default(),
            stale_after_days: 0,
            narrow_use_threshold: 0,
        };
        let result = provider
            .call(traced_command(
                SKILL_CURATION_DRY_RUN_COMMAND,
                dry_run,
                trace,
            ))
            .await
            .expect("dry-run should succeed");
        let result: SkillCurationDryRunResult =
            serde_json::from_value(result.output).expect("dry-run result should decode");

        assert!(!result.mutated);
        assert_eq!(result.recommendations.len(), 1);
        assert_eq!(
            result.recommendations[0].action,
            SkillCurationAction::Protected
        );
        assert!(result.recommendations[0].protected);
    }
}
