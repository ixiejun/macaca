//! Application consumption helpers for downstream crates.
//!
//! These helpers keep application-level interpretation inside `macaca-app`
//! so consumers do not re-implement manifest reading, entry-agent fallback,
//! or runtime assembly details in parallel.

use crate::model::{AgentSource, AppManifest, CapabilityRef, InlineAgentConfig};
use crate::registry::DiscoveredApp;
use crate::runtime::{
    AppRuntimeBuilder, ApplicationRuntimeFactory, DefaultApplicationRuntimeFactory,
};
use crate::workflow::{
    DefaultWorkflowPromptStrategy, WorkflowEngine, WorkflowPromptContext, WorkflowPromptStrategy,
    DEFAULT_WORKFLOW,
};
use macaca_proto::{
    ApplicationPlanningAgentProfile, ApplicationTaskPlanningContract, MacacaResult,
};

/// Read-only view of a manifest-declared inline agent.
#[derive(Debug, Clone, Copy)]
pub struct AppAgentManifestView<'a> {
    manifest: &'a AppManifest,
    inline: &'a InlineAgentConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppToolPolicyView {
    pub allowed_tools: Vec<String>,
    pub execution_tools: Vec<String>,
    pub is_entry_agent: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppAgentPromptSemantics {
    pub base_prompt: String,
    pub workflow_name: String,
    pub entry_agent: String,
    pub is_entry_agent: bool,
    pub prompt_parts: Option<crate::workflow::WorkflowPromptParts>,
    pub tool_policy: AppToolPolicyView,
}

pub type AppPlanningAgentProfile = ApplicationPlanningAgentProfile;
pub type AppTaskPlanningContract = ApplicationTaskPlanningContract;

impl<'a> AppAgentManifestView<'a> {
    pub fn name(&self) -> &str {
        &self.inline.name
    }

    pub fn is_entry_agent(&self) -> bool {
        app_entry_agent_name(self.manifest).is_some_and(|entry| entry == self.inline.name)
    }

    pub fn allowed_tools(&self) -> &[String] {
        &self.inline.allowed_tools
    }

    pub fn capabilities(&self) -> &[CapabilityRef] {
        &self.inline.capabilities
    }

    pub fn has_capability(&self, capability_name: &str) -> bool {
        self.inline
            .capabilities
            .iter()
            .any(|capability| capability.name == capability_name)
    }

    pub fn execution_tools(&self) -> Vec<&str> {
        self.inline
            .allowed_tools
            .iter()
            .filter_map(|tool| tool.ends_with("_execute").then_some(tool.as_str()))
            .collect()
    }
}

/// Resolve the configured entry agent name if the manifest declares one.
pub fn app_entry_agent_name(manifest: &AppManifest) -> Option<&str> {
    manifest.entry_agent.as_deref()
}

/// Resolve the effective entry agent name with a caller-provided fallback.
#[deprecated(note = "Use `app_entry_agent_name(manifest).unwrap_or(fallback)` instead.")]
pub fn app_entry_agent_name_or(manifest: &AppManifest, fallback: &str) -> String {
    app_entry_agent_name(manifest)
        .unwrap_or(fallback)
        .to_string()
}

/// Resolve the effective entry workflow name using `WorkflowEngine`'s current
/// compatibility logic.
pub fn app_entry_workflow_name(manifest: &AppManifest) -> String {
    WorkflowEngine::get_entrypoint_workflow(manifest)
}

/// Build a compatibility base prompt for an agent from application-level
/// workflow semantics.
///
/// This keeps downstream consumers from re-implementing entry-agent fallback
/// prompt rules. Non-entry agents retain the legacy generic fallback. Entry
/// agents continue to identify as the concrete agent name, while workflow
/// constraints and tool policy come from `WorkflowPromptStrategy`.
#[deprecated(note = "Use `app_agent_prompt_semantics(manifest, agent_name).base_prompt` instead.")]
pub fn app_agent_base_prompt(manifest: &AppManifest, agent_name: &str) -> String {
    app_agent_prompt_semantics(manifest, agent_name).base_prompt
}

/// Resolve application prompt semantics for an agent from manifest-level
/// workflow and tool policy declarations.
pub fn app_agent_prompt_semantics(
    manifest: &AppManifest,
    agent_name: &str,
) -> AppAgentPromptSemantics {
    let entry_agent = app_entry_agent_name(manifest)
        .unwrap_or(agent_name)
        .to_string();
    let workflow_name = app_entry_workflow_name(manifest);
    let is_entry_agent = entry_agent == agent_name;
    let tool_policy = app_agent_manifest_view(manifest, agent_name)
        .map(|agent| AppToolPolicyView {
            allowed_tools: agent.allowed_tools().to_vec(),
            execution_tools: agent
                .execution_tools()
                .into_iter()
                .map(str::to_string)
                .collect(),
            is_entry_agent,
        })
        .unwrap_or_else(|| AppToolPolicyView {
            allowed_tools: vec![],
            execution_tools: vec![],
            is_entry_agent,
        });

    if app_entry_agent_name(manifest).is_some_and(|entry| entry == agent_name) {
        let context = WorkflowPromptContext {
            manifest: manifest.clone(),
            workflow_name: workflow_name.clone(),
            coordinator: agent_name.to_string(),
            additional_context: None,
        };
        let parts = DefaultWorkflowPromptStrategy.prompt_parts(&context);
        let mut sections = vec![format!("You are the {} agent in Macaca OS.", agent_name)];
        if !parts.constraints.trim().is_empty() {
            sections.push(parts.constraints.clone());
        }
        if !parts.tools.trim().is_empty() {
            sections.push(parts.tools.clone());
        }
        if !parts.handoff.trim().is_empty() {
            sections.push(parts.handoff.clone());
        }
        AppAgentPromptSemantics {
            base_prompt: sections.join("\n\n"),
            workflow_name,
            entry_agent,
            is_entry_agent,
            prompt_parts: Some(parts),
            tool_policy,
        }
    } else {
        AppAgentPromptSemantics {
            base_prompt: format!("You are the {} agent in Macaca OS.", agent_name),
            workflow_name,
            entry_agent,
            is_entry_agent,
            prompt_parts: None,
            tool_policy,
        }
    }
}

/// Build a task-planning contract from application semantics and live worker
/// profiles. Downstream planners consume this instead of rebuilding prompt
/// rules ad hoc in each crate.
pub fn app_task_planning_contract(
    manifest: &AppManifest,
    worker_agents: Vec<AppPlanningAgentProfile>,
) -> AppTaskPlanningContract {
    AppTaskPlanningContract {
        workflow_name: app_entry_workflow_name(manifest),
        entry_agent: app_entry_agent_name(manifest)
            .unwrap_or("entry_agent")
            .to_string(),
        worker_agents,
    }
}

/// Build the legacy default planning contract for older task callers that
/// only know the list of available worker agent names.
///
/// This keeps compatibility behavior centralized in `macaca-app` while newer
/// runtime paths pass a real application manifest into
/// [`app_task_planning_contract`].
#[deprecated(
    note = "Use `app_task_planning_contract` or construct `AppTaskPlanningContract` explicitly."
)]
pub fn legacy_app_task_planning_contract(available_agents: &[String]) -> AppTaskPlanningContract {
    AppTaskPlanningContract {
        workflow_name: DEFAULT_WORKFLOW.into(),
        entry_agent: "entry_agent".into(),
        worker_agents: available_agents
            .iter()
            .cloned()
            .map(AppPlanningAgentProfile::legacy)
            .collect(),
    }
}

/// Find a manifest-declared inline agent view by name.
pub fn app_agent_manifest_view<'a>(
    manifest: &'a AppManifest,
    agent_name: &str,
) -> Option<AppAgentManifestView<'a>> {
    manifest.agents.iter().find_map(|source| match source {
        AgentSource::Inline(inline) if inline.name == agent_name => {
            Some(AppAgentManifestView { manifest, inline })
        }
        _ => None,
    })
}

/// Build an `AppRuntimeBuilder` for a discovered application.
pub fn discovered_app_runtime_builder(app: &DiscoveredApp) -> AppRuntimeBuilder {
    DefaultApplicationRuntimeFactory.build_runtime_builder(app.manifest.clone(), app.path.clone())
}

/// Resolve the app-scoped registered agent names via the runtime builder path.
pub fn discovered_app_agent_names(app: &DiscoveredApp) -> MacacaResult<Vec<String>> {
    discovered_app_runtime_builder(app)
        .resolve_agent_configs()
        .map(|configs| configs.into_iter().map(|config| config.name).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        AgentSkillsConfig, AppLayer, AppManifest, CapabilityRef, InlineAgentConfig,
    };

    fn manifest() -> AppManifest {
        AppManifest {
            id: macaca_proto::ApplicationId::new(),
            name: "demo".into(),
            description: None,
            version: "0.1.0".into(),
            layer: AppLayer::L3Declarative,
            ui_type: None,
            agents: vec![AgentSource::Inline(InlineAgentConfig {
                name: "coordinator".into(),
                capabilities: vec![CapabilityRef {
                    name: "todo_goal_management".into(),
                    description: "goal".into(),
                }],
                prompt_template: String::new(),
                model: "mock".into(),
                permission_level: "system".into(),
                allowed_tools: vec!["claude_code_execute".into(), "file_write".into()],
                max_tokens: None,
                temperature: None,
                skills: Some(AgentSkillsConfig::default()),
            })],
            llm_config: None,
            entry_agent: Some("coordinator".into()),
            entrypoint: None,
            workflows: None,
            resources: None,
        }
    }

    #[test]
    fn app_agent_view_exposes_manifest_semantics() {
        let manifest = manifest();
        let agent = app_agent_manifest_view(&manifest, "coordinator").unwrap();
        assert!(agent.is_entry_agent());
        assert!(agent.has_capability("todo_goal_management"));
        assert_eq!(agent.execution_tools(), vec!["claude_code_execute"]);
        assert_eq!(agent.allowed_tools().len(), 2);
    }

    #[test]
    #[allow(deprecated)]
    fn entry_agent_helpers_preserve_fallback_behavior() {
        let manifest = manifest();
        assert_eq!(app_entry_agent_name(&manifest), Some("coordinator"));
        assert_eq!(
            app_entry_agent_name_or(&manifest, "fallback"),
            "coordinator"
        );

        let mut no_entry = manifest;
        no_entry.entry_agent = None;
        assert_eq!(app_entry_agent_name(&no_entry), None);
        assert_eq!(app_entry_agent_name_or(&no_entry, "fallback"), "fallback");
    }

    #[test]
    #[allow(deprecated)]
    fn entry_agent_base_prompt_uses_workflow_sections() {
        let manifest = manifest();
        let prompt = app_agent_base_prompt(&manifest, "coordinator");
        assert!(prompt.contains("You are the coordinator agent in Macaca OS."));
        assert!(prompt.contains("SDD"));
        assert!(prompt.contains("claude_code_execute"));
    }

    #[test]
    #[allow(deprecated)]
    fn worker_base_prompt_preserves_legacy_generic_fallback() {
        let manifest = manifest();
        let prompt = app_agent_base_prompt(&manifest, "backend");
        assert_eq!(prompt, "You are the backend agent in Macaca OS.");
    }

    #[test]
    fn prompt_semantics_expose_structured_workflow_and_tool_policy() {
        let manifest = manifest();
        let semantics = app_agent_prompt_semantics(&manifest, "coordinator");
        assert!(semantics.is_entry_agent);
        assert_eq!(semantics.entry_agent, "coordinator");
        assert_eq!(semantics.workflow_name, "default");
        assert!(semantics.prompt_parts.is_some());
        assert_eq!(semantics.tool_policy.allowed_tools.len(), 2);
        assert_eq!(
            semantics.tool_policy.execution_tools,
            vec!["claude_code_execute".to_string()]
        );
    }

    #[test]
    fn task_planning_contract_renders_worker_profiles() {
        let manifest = manifest();
        let contract = app_task_planning_contract(
            &manifest,
            vec![AppPlanningAgentProfile {
                name: "backend".into(),
                capabilities: vec!["api: implement backend endpoints".into()],
                available: true,
                current_load: 0,
                max_load: 2,
                permission_level: "system".into(),
                model: "mock".into(),
                allowed_tools: vec!["claude_code_execute".into()],
            }],
        );
        assert_eq!(contract.available_agent_names(), vec!["backend"]);
        let rendered = contract.render_agent_profiles();
        assert!(rendered.contains("Agent `backend`"));
        assert!(rendered.contains("api: implement backend endpoints"));
    }

    #[test]
    #[allow(deprecated)]
    fn legacy_task_planning_contract_preserves_legacy_defaults() {
        let contract = legacy_app_task_planning_contract(&["backend".into(), "frontend".into()]);
        assert_eq!(contract.workflow_name, "default");
        assert_eq!(contract.entry_agent, "entry_agent");
        assert_eq!(
            contract.available_agent_names(),
            vec!["backend".to_string(), "frontend".to_string()]
        );
    }
}
