//! Runtime facade command handlers for snapshot, executable tools, and status.
//!
//! These commands bridge the Skill service contract to `SkillRuntimeFacade` and
//! `ExecutableSkillToolSet` without embedding governance or evolution logic.

use macaca_proto::{
    CapabilityToolDescriptor, CapabilityToolInvocationResult, CapabilityToolOriginKind,
    CapabilityToolResourceScope, ServiceCallResult, ServiceCommand, ServiceResult,
};
use macaca_skill::{
    SkillExecutableLoadCommand, SkillExecutableLoadResult, SkillServiceSnapshot,
    SkillServiceSnapshotCommand, SkillSnapshotRequest, SkillSnapshotServiceCommand,
    SkillStatusCommand, SkillStatusResult, SkillToolCatalogCommand, SkillToolCatalogResult,
    SkillToolInvokeCommand, SKILL_SERVICE_ID,
};
use macaca_tools::{ToolCommand, ToolCommandExecutor};

use crate::skill_service_codec::{decode, service_adapter_error, service_result, to_value};

use super::SkillSystemServiceProvider;

impl SkillSystemServiceProvider {
    /// Build an agent-scoped skill snapshot through the configured runtime facade.
    pub(crate) async fn handle_skill_snapshot(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<ServiceCallResult> {
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
        Ok(service_result(to_value(snapshot)?, typed.trace))
    }

    /// Load executable skill tool definitions from configured directories.
    pub(crate) async fn handle_executable_load(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<ServiceCallResult> {
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
        Ok(service_result(to_value(result)?, typed.trace))
    }

    /// Emit capability tool descriptors for all loaded executable skills.
    pub(crate) async fn handle_tool_catalog(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<ServiceCallResult> {
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
            .collect::<Result<Vec<_>, _>>()
            .map_err(service_adapter_error)?;
        tracing::info!(
            trace_id = %typed.trace.trace_id,
            count = descriptors.len(),
            "skill service tool catalog emitted"
        );
        Ok(service_result(
            to_value(SkillToolCatalogResult::new(descriptors))?,
            typed.trace,
        ))
    }

    /// Invoke a loaded executable skill tool by name.
    pub(crate) async fn handle_tool_invoke(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<ServiceCallResult> {
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
        Ok(service_result(to_value(result)?, typed.invocation.trace))
    }

    /// Report bounded service health and telemetry aggregates.
    pub(crate) async fn handle_status(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: SkillStatusCommand = decode(command.payload)?;
        let registry = self.executable_tools.lock().await.snapshot();
        let telemetry_aggregate = self.governance_state.telemetry_aggregate().await;
        let result = SkillStatusResult {
            service_id: SKILL_SERVICE_ID.into(),
            healthy: self.snapshot_facade.is_some(),
            snapshot_skills: 0,
            executable_skills: registry.len(),
            telemetry_aggregate,
            captured_at: chrono::Utc::now(),
        };
        Ok(service_result(to_value(result)?, typed.trace))
    }

    /// Emit a lightweight service snapshot containing the executable registry only.
    pub(crate) async fn handle_service_snapshot(
        &self,
        command: ServiceCommand,
    ) -> ServiceResult<ServiceCallResult> {
        let typed: SkillServiceSnapshotCommand = decode(command.payload)?;
        let registry = self.executable_tools.lock().await.snapshot();
        let snapshot = SkillServiceSnapshot::new(None, Some(registry.clone()), registry.len());
        tracing::info!(trace_id = %typed.trace.trace_id, "skill service snapshot emitted");
        Ok(service_result(to_value(snapshot)?, typed.trace))
    }
}
