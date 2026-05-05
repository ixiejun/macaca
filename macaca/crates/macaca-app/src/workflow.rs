//! Workflow Engine — executes defined workflows from app manifests.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use macaca_kernel::Kernel;
use macaca_llm::LlmProvider;
use macaca_proto::{AgentId, MacacaError, MacacaResult};

use crate::model::{AgentSource, AppManifest, EntrypointType, WorkflowDefinition, WorkflowStep};

/// Default workflow name if not specified.
pub const DEFAULT_WORKFLOW: &str = "default";

/// Default coordinator agent name.
pub const DEFAULT_COORDINATOR: &str = "architect";

/// Structured parts of a workflow prompt. This is an internal compatibility
/// layer so prompt rendering can evolve without scattering large string blocks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowPromptParts {
    pub role: String,
    pub constraints: String,
    pub tools: String,
    pub handoff: String,
}

/// Context used by prompt rendering strategies.
#[derive(Debug, Clone)]
pub struct WorkflowPromptContext {
    pub manifest: AppManifest,
    pub workflow_name: String,
    pub coordinator: String,
    pub additional_context: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WorkflowToolPolicy {
    execution_tools: Vec<String>,
    allowed_tools: Vec<String>,
    provider: Option<String>,
}

impl WorkflowPromptContext {
    fn coordinator_tool_policy(&self) -> WorkflowToolPolicy {
        let allowed_tools = self
            .manifest
            .agents
            .iter()
            .find_map(|source| match source {
                AgentSource::Inline(inline) if inline.name == self.coordinator => {
                    Some(inline.allowed_tools.clone())
                }
                _ => None,
            })
            .unwrap_or_default();
        let execution_tools = allowed_tools
            .iter()
            .filter(|tool| tool.ends_with("_execute"))
            .cloned()
            .collect();
        WorkflowToolPolicy {
            execution_tools,
            allowed_tools,
            provider: self
                .manifest
                .llm_config
                .as_ref()
                .map(|config| config.provider.clone()),
        }
    }
}

/// Strategy for rendering workflow prompt sections.
pub trait WorkflowPromptStrategy: Send + Sync {
    fn prompt_parts(&self, ctx: &WorkflowPromptContext) -> WorkflowPromptParts;

    fn render(&self, ctx: &WorkflowPromptContext) -> String {
        let parts = self.prompt_parts(ctx);
        let mut prompt = format!("{}\n\n{}\n\n{}", parts.role, parts.constraints, parts.tools);
        if !parts.handoff.is_empty() {
            prompt.push_str("\n\n");
            prompt.push_str(&parts.handoff);
        }
        if let Some(additional) = ctx.additional_context.as_deref() {
            prompt.push_str("\n\n");
            prompt.push_str(additional);
        }
        prompt
    }
}

/// Default prompt strategy that preserves the current prompt text and behavior.
#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultWorkflowPromptStrategy;

impl WorkflowPromptStrategy for DefaultWorkflowPromptStrategy {
    fn prompt_parts(&self, ctx: &WorkflowPromptContext) -> WorkflowPromptParts {
        WorkflowPromptParts {
            role: "You are the coordinator of a fullstack development team in Macaca OS.".into(),
            constraints: "## SDD (Spec-Driven Development) Workflow\n\
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
                          Verify the implementation works correctly."
                .into(),
            tools: render_tool_policy_block(
                ctx,
                "## MANDATORY: Use Tools",
                Some("claude_code_execute"),
            ),
            handoff: "## Important Rules\n\
                      - Complete the ENTIRE task — do not stop halfway\n\
                      - Be thorough and precise"
                .into(),
        }
    }
}

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
        let workflow = manifest
            .workflows
            .as_ref()
            .and_then(|w| w.get(workflow_name));

        // Determine coordinator agent
        let coordinator = workflow
            .and_then(|w| w.steps.first())
            .map(|s| s.agent.as_str())
            .unwrap_or(DEFAULT_COORDINATOR);

        // Try to load persona
        let persona_dir = app_dir.join(format!("personas/{coordinator}"));
        let context = WorkflowPromptContext {
            manifest: manifest.clone(),
            workflow_name: workflow_name.into(),
            coordinator: coordinator.into(),
            additional_context: additional_context.map(str::to_owned),
        };

        Ok(if persona_dir.exists() {
            DefaultWorkflowPromptStrategy.render(&context)
        } else {
            Self::default_assistant_prompt_with_context(&context)
        })
    }

    /// Get the persona directory for a workflow.
    pub fn get_persona_dir(
        &self,
        manifest: &AppManifest,
        app_dir: &PathBuf,
        workflow_name: &str,
    ) -> Option<PathBuf> {
        let workflow = manifest
            .workflows
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
        DefaultWorkflowPromptStrategy.render(&WorkflowPromptContext {
            manifest: AppManifest {
                id: macaca_proto::ApplicationId::new(),
                name: "default".into(),
                description: None,
                version: "0.1.0".into(),
                layer: crate::model::AppLayer::L3Declarative,
                ui_type: None,
                agents: vec![],
                llm_config: None,
                entry_agent: None,
                entrypoint: None,
                workflows: None,
                resources: None,
                context: None,
            },
            workflow_name: DEFAULT_WORKFLOW.into(),
            coordinator: DEFAULT_COORDINATOR.into(),
            additional_context: None,
        })
    }

    /// Default assistant prompt when no persona is available.
    pub fn default_assistant_prompt() -> String {
        Self::default_assistant_prompt_with_context(&WorkflowPromptContext {
            manifest: AppManifest {
                id: macaca_proto::ApplicationId::new(),
                name: "default".into(),
                description: None,
                version: "0.1.0".into(),
                layer: crate::model::AppLayer::L3Declarative,
                ui_type: None,
                agents: vec![],
                llm_config: None,
                entry_agent: None,
                entrypoint: None,
                workflows: None,
                resources: None,
                context: None,
            },
            workflow_name: DEFAULT_WORKFLOW.into(),
            coordinator: DEFAULT_COORDINATOR.into(),
            additional_context: None,
        })
    }

    fn default_assistant_prompt_with_context(ctx: &WorkflowPromptContext) -> String {
        let mut prompt = format!(
            "You are an AI assistant in Macaca OS with access to tools.\n{}\nRespond helpfully and concisely.",
            render_tool_policy_block(ctx, "", Some("claude_code_execute"))
        );
        if let Some(additional) = ctx.additional_context.as_deref() {
            prompt.push_str("\n\n");
            prompt.push_str(additional);
        }
        prompt
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

    /// Get the entry point workflow name for an app.
    pub fn get_entrypoint_workflow(manifest: &AppManifest) -> String {
        manifest
            .entrypoint
            .as_ref()
            .filter(|e| e.type_ == EntrypointType::Workflow)
            .map(|e| e.name.clone())
            .unwrap_or_else(|| DEFAULT_WORKFLOW.into())
    }
}

fn render_tool_policy_block(
    ctx: &WorkflowPromptContext,
    heading: &str,
    legacy_default_execute_tool: Option<&str>,
) -> String {
    let policy = ctx.coordinator_tool_policy();
    let mut lines = Vec::new();
    if !heading.is_empty() {
        lines.push(heading.to_string());
    }

    if !policy.execution_tools.is_empty() {
        if policy.execution_tools.len() == 1 {
            lines.push(format!(
                "- Use `{}` for ALL code generation",
                policy.execution_tools[0]
            ));
        } else {
            lines.push(format!(
                "- Use the execution driver required by the application or task: {}",
                policy
                    .execution_tools
                    .iter()
                    .map(|tool| format!("`{tool}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
            lines.push(
                "- Do not substitute one execution driver for another unless the requested driver is unavailable"
                    .into(),
            );
        }
        lines.push("- If execution fails, report the error and stop".into());
    } else if let Some(default_tool) = legacy_default_execute_tool {
        lines.push(format!("- Use `{default_tool}` for ALL code generation"));
        lines.push(format!(
            "- If `{default_tool}` fails, report the error and stop"
        ));
    } else {
        lines.push("- Use only the tools allowed by the application and current agent".into());
        lines.push("- If execution fails, report the error and stop".into());
    }

    if policy.allowed_tools.iter().any(|tool| tool == "file_write") {
        lines.push("- NEVER use file_write to write source code".into());
    } else {
        lines.push("- NEVER write source code directly".into());
    }
    lines.join("\n")
}

impl Default for WorkflowEngine {
    fn default() -> Self {
        panic!("WorkflowEngine requires kernel and llm - use WorkflowEngine::new()");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::Arc;

    use async_trait::async_trait;
    use macaca_kernel::KernelBuilder;
    use macaca_llm::LlmProvider;
    use macaca_proto::config::KernelConfig;
    use macaca_proto::{LlmMessage, LlmOptions, LlmResponse, MacacaResult, TokenUsage};
    use macaca_tools::DefaultToolSet;

    use crate::loader::AppLoader;
    use crate::model::AppLayer;

    struct MockLlm;

    #[async_trait]
    impl LlmProvider for MockLlm {
        fn name(&self) -> &str {
            "mock"
        }

        async fn chat(
            &self,
            _messages: Vec<LlmMessage>,
            _options: &LlmOptions,
        ) -> MacacaResult<LlmResponse> {
            Ok(LlmResponse {
                content: "ok".into(),
                reasoning_content: None,
                model: "mock".into(),
                usage: TokenUsage {
                    prompt_tokens: 1,
                    completion_tokens: 1,
                    total_tokens: 2,
                },
                finish_reason: "stop".into(),
                tool_calls: None,
            })
        }
    }

    fn make_engine() -> WorkflowEngine {
        let llm: Arc<dyn LlmProvider> = Arc::new(MockLlm);
        let config = KernelConfig {
            max_agents: 4,
            heartbeat_interval_ms: 1000,
            agent_timeout_ms: 1000,
        };
        let kernel = Arc::new(
            KernelBuilder::new(config, Arc::clone(&llm), Box::new(DefaultToolSet::new())).build(),
        );
        WorkflowEngine::new(kernel, llm)
    }

    fn default_manifest_with_workflow() -> AppManifest {
        let mut workflows = HashMap::new();
        workflows.insert(
            DEFAULT_WORKFLOW.into(),
            WorkflowDefinition {
                description: None,
                steps: vec![WorkflowStep {
                    name: "coordinate".into(),
                    agent: DEFAULT_COORDINATOR.into(),
                    prompt_template: None,
                    depends_on: vec![],
                }],
            },
        );
        AppManifest {
            id: macaca_proto::ApplicationId::new(),
            name: "workflow-app".into(),
            description: None,
            version: "0.1.0".into(),
            layer: AppLayer::L3Declarative,
            ui_type: None,
            agents: vec![],
            llm_config: None,
            entry_agent: None,
            entrypoint: None,
            workflows: Some(workflows),
            resources: None,
            context: None,
        }
    }

    fn example_app_dir(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/apps")
            .join(name)
    }

    #[test]
    fn default_prompts_are_valid() {
        let workflow = WorkflowEngine::default_workflow_prompt();
        assert!(workflow.contains("SDD"));
        assert!(workflow.contains("Tools"));

        let assistant = WorkflowEngine::default_assistant_prompt();
        assert!(assistant.contains("assistant"));
    }

    #[test]
    fn default_workflow_strategy_matches_default_prompt() {
        let rendered = DefaultWorkflowPromptStrategy.render(&WorkflowPromptContext {
            manifest: default_manifest_with_workflow(),
            workflow_name: DEFAULT_WORKFLOW.into(),
            coordinator: DEFAULT_COORDINATOR.into(),
            additional_context: None,
        });
        assert_eq!(rendered, WorkflowEngine::default_workflow_prompt());
    }

    #[test]
    fn build_system_prompt_appends_additional_context() {
        let engine = make_engine();
        let manifest = default_manifest_with_workflow();
        let app_dir = std::env::temp_dir();
        let prompt = engine
            .build_system_prompt(&manifest, &app_dir, DEFAULT_WORKFLOW, Some("EXTRA_CTX"))
            .unwrap();
        assert!(prompt.contains("EXTRA_CTX"));
    }

    #[test]
    fn build_system_prompt_for_fullstack_fixture() {
        let engine = make_engine();
        let app_dir = example_app_dir("fullstack-autodev");
        let manifest = AppLoader::load_manifest(app_dir.join("app.yaml")).unwrap();
        let workflow_name = WorkflowEngine::get_entrypoint_workflow(&manifest);
        let prompt = engine
            .build_system_prompt(&manifest, &app_dir, &workflow_name, None)
            .unwrap();
        assert!(!prompt.trim().is_empty());
        assert!(prompt.contains("Use Tools") || prompt.contains("assistant"));
    }

    #[test]
    fn build_system_prompt_for_newsroom_fixture() {
        let engine = make_engine();
        let app_dir = example_app_dir("newsroom-autowriter");
        let manifest = AppLoader::load_manifest(app_dir.join("app.yaml")).unwrap();
        let workflow_name = WorkflowEngine::get_entrypoint_workflow(&manifest);
        let prompt = engine
            .build_system_prompt(&manifest, &app_dir, &workflow_name, None)
            .unwrap();
        assert!(!prompt.trim().is_empty());
        assert!(prompt.contains("Use Tools") || prompt.contains("assistant"));
    }

    #[test]
    fn build_system_prompt_uses_manifest_execution_tools_instead_of_single_hardcoded_driver() {
        let engine = make_engine();
        let manifest = AppManifest {
            agents: vec![AgentSource::Inline(crate::model::InlineAgentConfig {
                name: DEFAULT_COORDINATOR.into(),
                capabilities: vec![],
                prompt_template: String::new(),
                model: "mock".into(),
                permission_level: "system".into(),
                allowed_tools: vec![
                    "opencode_execute".into(),
                    "claude_code_execute".into(),
                    "file_write".into(),
                ],
                max_tokens: None,
                temperature: None,
                skills: None,
                context_engine: None,
            })],
            ..default_manifest_with_workflow()
        };
        let prompt = engine
            .build_system_prompt(&manifest, &std::env::temp_dir(), DEFAULT_WORKFLOW, None)
            .unwrap();
        assert!(prompt.contains("`opencode_execute`"));
        assert!(prompt.contains("`claude_code_execute`"));
        assert!(prompt.contains("Do not substitute one execution driver"));
    }
}
