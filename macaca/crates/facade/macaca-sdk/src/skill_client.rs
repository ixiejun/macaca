//! SDK Skill client facade for Route C S6.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::{MacacaError, MacacaResult};
use macaca_skill::{
    SkillAliasResolveCommand, SkillAliasResolveResult, SkillAliasSnapshotCommand,
    SkillAliasSnapshotResult, SkillAliasUpsertCommand, SkillAliasUpsertResult, SkillCleanupCommand,
    SkillCurationDryRunCommand, SkillCurationDryRunResult, SkillExecutableLoadCommand,
    SkillExecutableLoadResult, SkillExperienceProposalCommand, SkillExperienceProposalResult,
    SkillGovernanceRecordUsageCommand, SkillGovernanceRecordUsageResult,
    SkillGovernanceSnapshotCommand, SkillGovernanceSnapshotResult, SkillServiceSnapshot,
    SkillServiceSnapshotCommand, SkillSnapshotServiceCommand, SkillSnapshotServiceResult,
    SkillStatusCommand, SkillStatusResult, SkillToolCatalogCommand, SkillToolCatalogResult,
    SkillToolInvokeCommand, SkillToolInvokeResult, SKILL_ALIAS_RESOLVE_COMMAND,
    SKILL_ALIAS_SNAPSHOT_COMMAND, SKILL_ALIAS_UPSERT_COMMAND, SKILL_CLEANUP_COMMAND,
    SKILL_CURATION_DRY_RUN_COMMAND, SKILL_EVOLUTION_PROPOSE_FROM_TASK_COMMAND,
    SKILL_EXECUTABLE_LOAD_COMMAND, SKILL_GOVERNANCE_RECORD_USAGE_COMMAND,
    SKILL_GOVERNANCE_SNAPSHOT_COMMAND, SKILL_SERVICE_ID, SKILL_SERVICE_SNAPSHOT_COMMAND,
    SKILL_SNAPSHOT_COMMAND, SKILL_STATUS_COMMAND, SKILL_TOOL_CATALOG_COMMAND,
    SKILL_TOOL_INVOKE_COMMAND,
};
use tracing::{info, warn};

use crate::service_client::{ServiceCallCommand, SystemServiceClient};

/// Focused Skill client consumed by Web, CLI, framework, and applications.
#[async_trait]
pub trait SystemSkillClient: Send + Sync {
    async fn snapshot(
        &self,
        command: SkillSnapshotServiceCommand,
    ) -> MacacaResult<SkillSnapshotServiceResult>;
    async fn load_executable(
        &self,
        command: SkillExecutableLoadCommand,
    ) -> MacacaResult<SkillExecutableLoadResult>;
    async fn tool_catalog(
        &self,
        command: SkillToolCatalogCommand,
    ) -> MacacaResult<SkillToolCatalogResult>;
    async fn invoke_tool(
        &self,
        command: SkillToolInvokeCommand,
    ) -> MacacaResult<SkillToolInvokeResult>;
    async fn status(&self, command: SkillStatusCommand) -> MacacaResult<SkillStatusResult>;
    async fn service_snapshot(
        &self,
        command: SkillServiceSnapshotCommand,
    ) -> MacacaResult<SkillServiceSnapshot>;
    async fn record_governance_usage(
        &self,
        command: SkillGovernanceRecordUsageCommand,
    ) -> MacacaResult<SkillGovernanceRecordUsageResult>;
    async fn governance_snapshot(
        &self,
        command: SkillGovernanceSnapshotCommand,
    ) -> MacacaResult<SkillGovernanceSnapshotResult>;
    async fn curation_dry_run(
        &self,
        command: SkillCurationDryRunCommand,
    ) -> MacacaResult<SkillCurationDryRunResult>;
    async fn alias_upsert(
        &self,
        command: SkillAliasUpsertCommand,
    ) -> MacacaResult<SkillAliasUpsertResult>;
    async fn alias_resolve(
        &self,
        command: SkillAliasResolveCommand,
    ) -> MacacaResult<SkillAliasResolveResult>;
    async fn alias_snapshot(
        &self,
        command: SkillAliasSnapshotCommand,
    ) -> MacacaResult<SkillAliasSnapshotResult>;
    async fn propose_skill_experience(
        &self,
        command: SkillExperienceProposalCommand,
    ) -> MacacaResult<SkillExperienceProposalResult>;
    async fn cleanup(&self, command: SkillCleanupCommand) -> MacacaResult<serde_json::Value>;
}

/// Null-object Skill client used when no runtime-backed service is installed.
#[derive(Debug, Clone, Default)]
pub struct UnavailableSystemSkillClient;

#[async_trait]
impl SystemSkillClient for UnavailableSystemSkillClient {
    async fn snapshot(
        &self,
        command: SkillSnapshotServiceCommand,
    ) -> MacacaResult<SkillSnapshotServiceResult> {
        warn!(trace_id = %command.trace.trace_id, "sdk skill client unavailable for snapshot");
        Err(MacacaError::Config("Skill service is unavailable".into()))
    }

    async fn load_executable(
        &self,
        command: SkillExecutableLoadCommand,
    ) -> MacacaResult<SkillExecutableLoadResult> {
        warn!(trace_id = %command.trace.trace_id, "sdk skill client unavailable for executable load");
        Err(MacacaError::Config("Skill service is unavailable".into()))
    }

    async fn tool_catalog(
        &self,
        command: SkillToolCatalogCommand,
    ) -> MacacaResult<SkillToolCatalogResult> {
        info!(trace_id = %command.trace.trace_id, "sdk skill client returning empty catalog");
        Ok(SkillToolCatalogResult::new(Vec::new()))
    }

    async fn invoke_tool(
        &self,
        command: SkillToolInvokeCommand,
    ) -> MacacaResult<SkillToolInvokeResult> {
        warn!(
            trace_id = %command.invocation.trace.trace_id,
            tool = %command.invocation.tool_name,
            "sdk skill client unavailable for invocation"
        );
        Err(MacacaError::Config("Skill service is unavailable".into()))
    }

    async fn status(&self, command: SkillStatusCommand) -> MacacaResult<SkillStatusResult> {
        info!(trace_id = %command.trace.trace_id, "sdk skill client returning unavailable status");
        Ok(SkillStatusResult {
            service_id: SKILL_SERVICE_ID.into(),
            healthy: false,
            snapshot_skills: 0,
            executable_skills: 0,
            captured_at: chrono::Utc::now(),
        })
    }

    async fn service_snapshot(
        &self,
        command: SkillServiceSnapshotCommand,
    ) -> MacacaResult<SkillServiceSnapshot> {
        info!(trace_id = %command.trace.trace_id, "sdk skill client returning unavailable snapshot");
        Ok(SkillServiceSnapshot::unavailable(
            "runtime-backed Skill service is not installed",
        ))
    }

    async fn record_governance_usage(
        &self,
        command: SkillGovernanceRecordUsageCommand,
    ) -> MacacaResult<SkillGovernanceRecordUsageResult> {
        warn!(
            trace_id = %command.trace.trace_id,
            skill_id = %command.observation.skill_id,
            "sdk skill client unavailable for governance usage recording"
        );
        Err(MacacaError::Config("Skill service is unavailable".into()))
    }

    async fn governance_snapshot(
        &self,
        command: SkillGovernanceSnapshotCommand,
    ) -> MacacaResult<SkillGovernanceSnapshotResult> {
        info!(
            trace_id = %command.trace.trace_id,
            "sdk skill client returning empty governance snapshot"
        );
        Ok(SkillGovernanceSnapshotResult {
            records: Vec::new(),
            captured_at: chrono::Utc::now(),
        })
    }

    async fn curation_dry_run(
        &self,
        command: SkillCurationDryRunCommand,
    ) -> MacacaResult<SkillCurationDryRunResult> {
        info!(
            trace_id = %command.trace.trace_id,
            "sdk skill client returning unavailable curation dry-run"
        );
        Ok(SkillCurationDryRunResult {
            recommendations: Vec::new(),
            semantic_analysis_status: "unavailable: Skill service is unavailable".into(),
            mutated: false,
            captured_at: chrono::Utc::now(),
        })
    }

    async fn alias_upsert(
        &self,
        command: SkillAliasUpsertCommand,
    ) -> MacacaResult<SkillAliasUpsertResult> {
        warn!(
            trace_id = %command.trace.trace_id,
            source_skill_id = %command.record.source_skill_id,
            "sdk skill client unavailable for alias upsert"
        );
        Err(MacacaError::Config("Skill service is unavailable".into()))
    }

    async fn alias_resolve(
        &self,
        command: SkillAliasResolveCommand,
    ) -> MacacaResult<SkillAliasResolveResult> {
        info!(
            trace_id = %command.trace.trace_id,
            skill_id = %command.skill_id,
            "sdk skill client returning unresolved alias"
        );
        Ok(SkillAliasResolveResult::unresolved(
            &command,
            chrono::Utc::now(),
        ))
    }

    async fn alias_snapshot(
        &self,
        command: SkillAliasSnapshotCommand,
    ) -> MacacaResult<SkillAliasSnapshotResult> {
        info!(
            trace_id = %command.trace.trace_id,
            "sdk skill client returning empty alias snapshot"
        );
        Ok(SkillAliasSnapshotResult {
            aliases: Vec::new(),
            captured_at: chrono::Utc::now(),
        })
    }

    async fn propose_skill_experience(
        &self,
        command: SkillExperienceProposalCommand,
    ) -> MacacaResult<SkillExperienceProposalResult> {
        warn!(
            trace_id = %command.trace.trace_id,
            task_id = %command.candidate.task_id,
            "sdk skill client unavailable for experience proposal"
        );
        Err(MacacaError::Config("Skill service is unavailable".into()))
    }

    async fn cleanup(&self, command: SkillCleanupCommand) -> MacacaResult<serde_json::Value> {
        info!(trace_id = %command.trace.trace_id, "sdk skill client cleanup no-op");
        Ok(serde_json::json!({"status": "unavailable"}))
    }
}

/// Runtime-backed Skill client implemented over the generic SDK service client.
#[derive(Clone)]
pub struct ServiceBackedSkillClient {
    service: Arc<dyn SystemServiceClient>,
}

impl ServiceBackedSkillClient {
    /// Create a service-backed client from an existing generic service client.
    pub fn new(service: Arc<dyn SystemServiceClient>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl SystemSkillClient for ServiceBackedSkillClient {
    async fn snapshot(
        &self,
        command: SkillSnapshotServiceCommand,
    ) -> MacacaResult<SkillSnapshotServiceResult> {
        call(
            &self.service,
            SKILL_SNAPSHOT_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn load_executable(
        &self,
        command: SkillExecutableLoadCommand,
    ) -> MacacaResult<SkillExecutableLoadResult> {
        call(
            &self.service,
            SKILL_EXECUTABLE_LOAD_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn tool_catalog(
        &self,
        command: SkillToolCatalogCommand,
    ) -> MacacaResult<SkillToolCatalogResult> {
        call(
            &self.service,
            SKILL_TOOL_CATALOG_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn invoke_tool(
        &self,
        command: SkillToolInvokeCommand,
    ) -> MacacaResult<SkillToolInvokeResult> {
        call(
            &self.service,
            SKILL_TOOL_INVOKE_COMMAND,
            command.invocation.trace.clone(),
            command,
        )
        .await
    }

    async fn status(&self, command: SkillStatusCommand) -> MacacaResult<SkillStatusResult> {
        call(
            &self.service,
            SKILL_STATUS_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn service_snapshot(
        &self,
        command: SkillServiceSnapshotCommand,
    ) -> MacacaResult<SkillServiceSnapshot> {
        call(
            &self.service,
            SKILL_SERVICE_SNAPSHOT_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn record_governance_usage(
        &self,
        command: SkillGovernanceRecordUsageCommand,
    ) -> MacacaResult<SkillGovernanceRecordUsageResult> {
        call(
            &self.service,
            SKILL_GOVERNANCE_RECORD_USAGE_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn governance_snapshot(
        &self,
        command: SkillGovernanceSnapshotCommand,
    ) -> MacacaResult<SkillGovernanceSnapshotResult> {
        call(
            &self.service,
            SKILL_GOVERNANCE_SNAPSHOT_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn curation_dry_run(
        &self,
        command: SkillCurationDryRunCommand,
    ) -> MacacaResult<SkillCurationDryRunResult> {
        call(
            &self.service,
            SKILL_CURATION_DRY_RUN_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn alias_upsert(
        &self,
        command: SkillAliasUpsertCommand,
    ) -> MacacaResult<SkillAliasUpsertResult> {
        call(
            &self.service,
            SKILL_ALIAS_UPSERT_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn alias_resolve(
        &self,
        command: SkillAliasResolveCommand,
    ) -> MacacaResult<SkillAliasResolveResult> {
        call(
            &self.service,
            SKILL_ALIAS_RESOLVE_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn alias_snapshot(
        &self,
        command: SkillAliasSnapshotCommand,
    ) -> MacacaResult<SkillAliasSnapshotResult> {
        call(
            &self.service,
            SKILL_ALIAS_SNAPSHOT_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn propose_skill_experience(
        &self,
        command: SkillExperienceProposalCommand,
    ) -> MacacaResult<SkillExperienceProposalResult> {
        call(
            &self.service,
            SKILL_EVOLUTION_PROPOSE_FROM_TASK_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn cleanup(&self, command: SkillCleanupCommand) -> MacacaResult<serde_json::Value> {
        call(
            &self.service,
            SKILL_CLEANUP_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }
}

async fn call<T, R>(
    service: &Arc<dyn SystemServiceClient>,
    command_name: &str,
    trace: macaca_proto::TraceContext,
    payload: T,
) -> MacacaResult<R>
where
    T: serde::Serialize,
    R: serde::de::DeserializeOwned,
{
    let service_command = ServiceCallCommand::new(
        SKILL_SERVICE_ID,
        command_name,
        serde_json::to_value(payload)?,
    )?
    .with_trace(trace);
    let result = service.call_service(&service_command).await?;
    serde_json::from_value(result.output).map_err(MacacaError::from)
}
