//! Workflow DTOs and coordinator tool-policy adapter.
//!
//! These types are provider-neutral value objects consumed by prompt strategies
//! and the workflow engine.  Tool policy is derived from manifest agent
//! declarations rather than hardcoded driver names.

use crate::model::{AgentSource, AppManifest};

/// Default workflow name when the manifest entrypoint does not name one.
pub const DEFAULT_WORKFLOW: &str = "default";

/// Provider-neutral placeholder for skeleton prompts that intentionally lack a
/// real manifest coordinator (documentation helpers only).
pub(crate) const SKELETON_PROMPT_AGENT_LABEL: &str = "entry-agent";

/// Structured sections of a workflow system prompt.
///
/// Splitting the prompt into parts lets strategies evolve individual sections
/// without scattering large string literals across the engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowPromptParts {
    /// Role / identity preamble.
    pub role: String,
    /// Workflow constraints and staged instructions.
    pub constraints: String,
    /// Tool usage policy block derived from manifest declarations.
    pub tools: String,
    /// Handoff / completion rules appended after tool policy.
    pub handoff: String,
}

/// Input bundle for [`super::prompt_strategy::WorkflowPromptStrategy`] rendering.
#[derive(Debug, Clone)]
pub struct WorkflowPromptContext {
    /// Full application manifest supplying agent tool declarations.
    pub manifest: AppManifest,
    /// Active workflow graph name within the manifest.
    pub workflow_name: String,
    /// Resolved coordinator agent name for this workflow invocation.
    pub coordinator: String,
    /// Optional caller-supplied context appended after strategy output.
    pub additional_context: Option<String>,
}

/// Derived tool policy for the coordinator agent (manifest-driven, not hardcoded).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct WorkflowToolPolicy {
    /// Tools whose names end with `_execute` (execution drivers).
    pub execution_tools: Vec<String>,
    /// Full allow-list for the coordinator inline agent config.
    pub allowed_tools: Vec<String>,
    /// Optional LLM provider label from manifest `llm_config`.
    pub provider: Option<String>,
}

impl WorkflowPromptContext {
    /// Build coordinator tool policy by locating the inline agent matching
    /// [`Self::coordinator`] inside [`Self::manifest`].
    pub(super) fn coordinator_tool_policy(&self) -> WorkflowToolPolicy {
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

/// Builds a minimal skeleton [`WorkflowPromptContext`] for default prompt helpers.
pub(super) fn skeleton_prompt_context() -> WorkflowPromptContext {
    WorkflowPromptContext {
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
            service_contract: None,
            execution_profile: None,
            workbench: None,
            autonomy: None,
            ui: None,
            execution_control: None,
        },
        workflow_name: DEFAULT_WORKFLOW.into(),
        coordinator: SKELETON_PROMPT_AGENT_LABEL.into(),
        additional_context: None,
    }
}
