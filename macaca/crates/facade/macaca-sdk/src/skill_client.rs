//! SDK Skill client facade for Route C S6.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_proto::{MacacaError, MacacaResult};
use macaca_skill::{
    SkillAliasResolveCommand, SkillAliasResolveResult, SkillAliasSnapshotCommand,
    SkillAliasSnapshotResult, SkillAliasUpsertCommand, SkillAliasUpsertResult, SkillCleanupCommand,
    SkillCurationDryRunCommand, SkillCurationDryRunResult, SkillCurationLifecycleAction,
    SkillCurationLifecycleCommand, SkillCurationLifecycleResult, SkillExecutableLoadCommand,
    SkillExecutableLoadResult, SkillExperienceProposalCommand, SkillExperienceProposalResult,
    SkillExperienceProposalSnapshotCommand, SkillExperienceProposalSnapshotResult,
    SkillGovernanceRecordUsageCommand, SkillGovernanceRecordUsageResult,
    SkillGovernanceSnapshotCommand, SkillGovernanceSnapshotResult, SkillServiceSnapshot,
    SkillServiceSnapshotCommand, SkillSnapshotServiceCommand, SkillSnapshotServiceResult,
    SkillStatusCommand, SkillStatusResult, SkillToolCatalogCommand, SkillToolCatalogResult,
    SkillToolInvokeCommand, SkillToolInvokeResult, SKILL_SERVICE_ID,
};
use tracing::{info, warn};

use crate::service_client::SystemServiceClient;

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
    async fn curation_lifecycle(
        &self,
        action: SkillCurationLifecycleAction,
        command: SkillCurationLifecycleCommand,
    ) -> MacacaResult<SkillCurationLifecycleResult>;
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
    async fn skill_experience_snapshot(
        &self,
        command: SkillExperienceProposalSnapshotCommand,
    ) -> MacacaResult<SkillExperienceProposalSnapshotResult>;
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
            telemetry_aggregate: Default::default(),
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
            telemetry_aggregate: Default::default(),
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

    async fn curation_lifecycle(
        &self,
        action: SkillCurationLifecycleAction,
        command: SkillCurationLifecycleCommand,
    ) -> MacacaResult<SkillCurationLifecycleResult> {
        warn!(
            trace_id = %command.trace.trace_id,
            skill_id = %command.skill_id,
            action = ?action,
            "sdk skill client unavailable for curation lifecycle command"
        );
        Err(MacacaError::Config("Skill service is unavailable".into()))
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

    async fn skill_experience_snapshot(
        &self,
        command: SkillExperienceProposalSnapshotCommand,
    ) -> MacacaResult<SkillExperienceProposalSnapshotResult> {
        info!(
            trace_id = %command.trace.trace_id,
            "sdk skill client returning empty experience proposal snapshot"
        );
        Ok(SkillExperienceProposalSnapshotResult {
            proposals: Vec::new(),
            mutated: false,
            captured_at: chrono::Utc::now(),
        })
    }

    async fn cleanup(&self, command: SkillCleanupCommand) -> MacacaResult<serde_json::Value> {
        info!(trace_id = %command.trace.trace_id, "sdk skill client cleanup no-op");
        Ok(serde_json::json!({"status": "unavailable"}))
    }
}

/// Runtime-backed Skill client implemented over the generic SDK service client.
#[derive(Clone)]
pub struct ServiceBackedSkillClient {
    pub(crate) service: Arc<dyn SystemServiceClient>,
}

impl ServiceBackedSkillClient {
    /// Create a service-backed client from an existing generic service client.
    pub fn new(service: Arc<dyn SystemServiceClient>) -> Self {
        Self { service }
    }
}
