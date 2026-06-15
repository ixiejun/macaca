//! Workflow **engine** — coordinator resolution and prompt assembly.
//!
//! [`WorkflowEngine`] is the stateless façade for manifest workflow execution helpers.
//! Coordinator resolution uses Chain of Responsibility over workflow steps and
//! `entry_agent`, failing closed when neither is declared.

use std::collections::HashMap;
use std::path::PathBuf;

use macaca_proto::{MacacaError, MacacaResult};
use tracing::info;

use crate::consumption::app_entry_agent_name;
use crate::model::{AppManifest, EntrypointType, WorkflowDefinition, WorkflowStep};

use super::prompt_strategy::{
    default_assistant_prompt_with_context, DefaultWorkflowPromptStrategy, WorkflowPromptStrategy,
};
use super::types::{skeleton_prompt_context, WorkflowPromptContext, DEFAULT_WORKFLOW};

/// Engine for executing workflows declared in application manifests.
#[derive(Debug, Clone, Copy, Default)]
pub struct WorkflowEngine;

impl WorkflowEngine {
    /// Construct a provider-neutral workflow helper.
    pub fn new() -> Self {
        tracing::info!("workflow engine constructed without provider-owned state");
        Self
    }

    /// Resolve the workflow coordinator agent from manifest declarations.
    ///
    /// Resolution order (Chain of Responsibility):
    /// 1. First step agent in `manifest.workflows[workflow_name]`
    /// 2. `manifest.entry_agent`
    ///
    /// Fails closed when neither source is declared so OS code never invents
    /// application-specific agent role names as silent fallbacks.
    pub fn resolve_coordinator_agent(
        manifest: &AppManifest,
        workflow_name: &str,
    ) -> MacacaResult<String> {
        if let Some(workflow) = manifest
            .workflows
            .as_ref()
            .and_then(|workflows| workflows.get(workflow_name))
        {
            if let Some(step) = workflow.steps.first() {
                let agent = step.agent.trim();
                if !agent.is_empty() {
                    tracing::debug!(
                        workflow = workflow_name,
                        coordinator = agent,
                        source = "workflow_first_step",
                        "resolved workflow coordinator from manifest workflow step"
                    );
                    return Ok(agent.to_string());
                }
            }
        }

        if let Some(entry_agent) = app_entry_agent_name(manifest) {
            tracing::debug!(
                workflow = workflow_name,
                coordinator = entry_agent,
                source = "manifest_entry_agent",
                "resolved workflow coordinator from manifest entry_agent"
            );
            return Ok(entry_agent.to_string());
        }

        Err(MacacaError::Config(format!(
            "workflow '{workflow_name}' requires coordinator agent: declare workflows.{workflow_name}.steps[0].agent or manifest.entry_agent"
        )))
    }

    /// Build the system prompt for a workflow invocation.
    ///
    /// Combines persona presence detection, default SDD strategy rendering, and
    /// optional caller `additional_context` appended by the strategy layer.
    pub fn build_system_prompt(
        &self,
        manifest: &AppManifest,
        app_dir: &PathBuf,
        workflow_name: &str,
        additional_context: Option<&str>,
    ) -> MacacaResult<String> {
        let coordinator = Self::resolve_coordinator_agent(manifest, workflow_name)?;
        let persona_dir = app_dir.join(format!("personas/{coordinator}"));
        let context = WorkflowPromptContext {
            manifest: manifest.clone(),
            workflow_name: workflow_name.into(),
            coordinator: coordinator.clone(),
            additional_context: additional_context.map(str::to_owned),
        };

        let prompt = if persona_dir.exists() {
            DefaultWorkflowPromptStrategy.render(&context)
        } else {
            default_assistant_prompt_with_context(&context)
        };

        info!(
            workflow = workflow_name,
            coordinator = coordinator.as_str(),
            persona_present = persona_dir.exists(),
            prompt_len = prompt.len(),
            "workflow system prompt assembled"
        );
        Ok(prompt)
    }

    /// Return the persona directory when it exists for the resolved coordinator.
    pub fn get_persona_dir(
        &self,
        manifest: &AppManifest,
        app_dir: &PathBuf,
        workflow_name: &str,
    ) -> Option<PathBuf> {
        let coordinator = Self::resolve_coordinator_agent(manifest, workflow_name).ok()?;
        let persona_dir = app_dir.join(format!("personas/{coordinator}"));
        if persona_dir.exists() {
            Some(persona_dir)
        } else {
            None
        }
    }

    /// Default SDD workflow prompt for skeleton manifest helpers.
    pub fn default_workflow_prompt() -> String {
        DefaultWorkflowPromptStrategy.render(&skeleton_prompt_context())
    }

    /// Default assistant prompt when no persona is available (skeleton manifest).
    pub fn default_assistant_prompt() -> String {
        default_assistant_prompt_with_context(&skeleton_prompt_context())
    }

    /// Validate workflow steps (cycle detection via DFS on `depends_on` edges).
    pub fn validate_workflow(workflow: &WorkflowDefinition) -> MacacaResult<()> {
        let mut visited = HashMap::new();
        for step in &workflow.steps {
            Self::check_dependencies(&workflow.steps, &step.name, &mut visited)?;
        }
        Ok(())
    }

    fn check_dependencies(
        steps: &[WorkflowStep],
        step_name: &str,
        visited: &mut HashMap<String, bool>,
    ) -> MacacaResult<()> {
        if let Some(&in_progress) = visited.get(step_name) {
            if in_progress {
                return Err(MacacaError::Config(format!(
                    "Circular dependency detected at step: {step_name}"
                )));
            }
            return Ok(());
        }

        visited.insert(step_name.into(), true);

        let step = steps
            .iter()
            .find(|s| s.name == step_name)
            .ok_or_else(|| MacacaError::Config(format!("Step not found: {step_name}")))?;

        for dep in &step.depends_on {
            Self::check_dependencies(steps, dep, visited)?;
        }

        visited.insert(step_name.into(), false);
        Ok(())
    }

    /// Resolve the entry-point workflow name from manifest `entrypoint`.
    pub fn get_entrypoint_workflow(manifest: &AppManifest) -> String {
        manifest
            .entrypoint
            .as_ref()
            .filter(|e| e.type_ == EntrypointType::Workflow)
            .map(|e| e.name.clone())
            .unwrap_or_else(|| DEFAULT_WORKFLOW.into())
    }
}
