//! Workflow Engine — executes defined workflows from app manifests.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use macaca_kernel::Kernel;
use macaca_llm::LlmProvider;
use macaca_proto::{AgentId, MacacaError, MacacaResult};

use crate::model::{AppManifest, WorkflowDefinition, WorkflowStep, EntrypointType};

/// Default workflow name if not specified.
pub const DEFAULT_WORKFLOW: &str = "default";

/// Default coordinator agent name.
pub const DEFAULT_COORDINATOR: &str = "architect";

/// Result of a workflow execution.
#[derive(Debug, Clone)]
pub struct WorkflowResult {
    /// Output from the workflow.
    pub output: String,
    /// Agent that produced the final output.
    pub agent_id: AgentId,
    /// Number of steps executed.
    pub steps_executed: usize,
}

/// Context for workflow execution.
pub struct WorkflowContext {
    /// Application manifest.
    pub manifest: AppManifest,
    /// Application directory.
    pub app_dir: PathBuf,
    /// Kernel for agent operations.
    pub kernel: Arc<Kernel>,
    /// LLM provider.
    pub llm: Arc<dyn LlmProvider>,
    /// User input.
    pub input: String,
}

/// Engine for executing workflows defined in app manifests.
pub struct WorkflowEngine {
    /// Kernel reference.
    kernel: Arc<Kernel>,
    /// LLM provider reference.
    llm: Arc<dyn LlmProvider>,
}

impl WorkflowEngine {
    /// Create a new workflow engine.
    pub fn new(kernel: Arc<Kernel>, llm: Arc<dyn LlmProvider>) -> Self {
        Self { kernel, llm }
    }

    /// Build the system prompt for a workflow.
    ///
    /// This combines:
    /// 1. Persona from the coordinator agent
    /// 2. Workflow instructions (SDD or custom)
    /// 3. Skill catalog (if available)
    pub fn build_system_prompt(
        &self,
        manifest: &AppManifest,
        app_dir: &PathBuf,
        workflow_name: &str,
        additional_context: Option<&str>,
    ) -> MacacaResult<String> {
        // Get workflow definition
        let workflow = manifest.workflows
            .as_ref()
            .and_then(|w| w.get(workflow_name));

        // Determine coordinator agent
        let coordinator = workflow
            .and_then(|w| w.steps.first())
            .map(|s| s.agent.as_str())
            .unwrap_or(DEFAULT_COORDINATOR);

        // Try to load persona
        let persona_dir = app_dir.join(format!("personas/{coordinator}"));
        let base_prompt = if persona_dir.exists() {
            // Persona exists - we'll return a marker and let the caller load it
            // For now, return a default prompt that will be combined with persona
            Self::default_workflow_prompt()
        } else {
            Self::default_assistant_prompt()
        };

        // Combine with additional context
        let full_prompt = match additional_context {
            Some(ctx) => format!("{base_prompt}\n\n{ctx}"),
            None => base_prompt,
        };

        Ok(full_prompt)
    }

    /// Get the persona directory for a workflow.
    pub fn get_persona_dir(
        &self,
        manifest: &AppManifest,
        app_dir: &PathBuf,
        workflow_name: &str,
    ) -> Option<PathBuf> {
        let workflow = manifest.workflows
            .as_ref()
            .and_then(|w| w.get(workflow_name))?;

        let coordinator = workflow.steps.first()?.agent.as_str();
        let persona_dir = app_dir.join(format!("personas/{coordinator}"));

        if persona_dir.exists() {
            Some(persona_dir)
        } else {
            None
        }
    }

    /// Default SDD workflow prompt.
    pub fn default_workflow_prompt() -> String {
        "You are the coordinator of a fullstack development team in Macaca OS.\n\
         \n\
         ## SDD (Spec-Driven Development) Workflow\n\
         You MUST follow this workflow for every project:\n\
         \n\
         ### Stage 1: Initialize\n\
         Analyze the user request and create a plan.\n\
         \n\
         ### Stage 2: Plan\n\
         Break down the work into actionable tasks.\n\
         \n\
         ### Stage 3: Execute\n\
         Implement the plan step by step.\n\
         \n\
         ### Stage 4: Validate\n\
         Verify the implementation works correctly.\n\
         \n\
         ## MANDATORY: Use Tools\n\
         - Use `claude_code_execute` for ALL code generation\n\
         - If execution fails, report the error and stop\n\
         - NEVER write source code directly\n\
         \n\
         ## Important Rules\n\
         - Complete the ENTIRE task — do not stop halfway\n\
         - Be thorough and precise".into()
    }

    /// Default assistant prompt when no persona is available.
    pub fn default_assistant_prompt() -> String {
        "You are an AI assistant in Macaca OS with access to tools.\n\
         Use `claude_code_execute` for ALL coding tasks.\n\
         NEVER use file_write to write source code.\n\
         If claude_code_execute fails, report the error and stop.\n\
         Respond helpfully and concisely.".into()
    }

    /// Validate workflow steps (check for circular dependencies, etc.).
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

        let step = steps.iter()
            .find(|s| s.name == step_name)
            .ok_or_else(|| MacacaError::Config(format!("Step not found: {step_name}")))?;

        for dep in &step.depends_on {
            Self::check_dependencies(steps, dep, visited)?;
        }

        visited.insert(step_name.into(), false);
        Ok(())
    }

    /// Get the entry point workflow name for an app.
    pub fn get_entrypoint_workflow(manifest: &AppManifest) -> String {
        manifest.entrypoint
            .as_ref()
            .filter(|e| e.type_ == EntrypointType::Workflow)
            .map(|e| e.name.clone())
            .unwrap_or_else(|| DEFAULT_WORKFLOW.into())
    }
}

impl Default for WorkflowEngine {
    fn default() -> Self {
        panic!("WorkflowEngine requires kernel and llm - use WorkflowEngine::new()");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_prompts_are_valid() {
        let workflow = WorkflowEngine::default_workflow_prompt();
        assert!(workflow.contains("SDD"));
        assert!(workflow.contains("Tools"));

        let assistant = WorkflowEngine::default_assistant_prompt();
        assert!(assistant.contains("assistant"));
    }
}
