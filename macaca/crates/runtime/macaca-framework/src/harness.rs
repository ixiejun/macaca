// SPDX-License-Identifier: Apache-2.0
//
// Derived from AgentScope Java 2.0 concepts and APIs.
// Copyright 2024-2026 the original AgentScope author or authors.
// Licensed under the Apache License, Version 2.0.

//! AgentScope 2.0-style Harness wrapper and workspace context contracts.
//!
//! Harness is deliberately a thin engineering layer over `ReActAgent`. The
//! framework owns only provider-neutral descriptors and prompt middleware. Real
//! filesystem, sandbox, memory, skill, MCP, and subagent backends must enter
//! through runtime-host services or plugins so generic OS code never bypasses
//! policy or hardcodes application behavior.

#[path = "harness_context.rs"]
mod harness_context;
#[path = "harness_filesystem.rs"]
mod harness_filesystem;
#[path = "harness_plan.rs"]
mod harness_plan;
#[path = "harness_skill.rs"]
mod harness_skill;
#[path = "harness_subagent.rs"]
mod harness_subagent;

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::agent::AgentResult;
use crate::formatter::Formatter;
use crate::message::Msg;
use crate::middleware::{
    AgentMiddleware, AgentMiddlewareChain, MiddlewareContext, MiddlewarePhase, MiddlewareStage,
};
use crate::model::ChatModel;
use crate::react_agent::{AgentRunCommand, AgentRunResult, ReActAgent};
use crate::runtime_context::RuntimeContext;

pub use harness_context::{
    HarnessContextCompactionResult, HarnessContextManager, HarnessContextPolicy,
    HarnessMemoryFlushCommand, HarnessMemoryFlushResult, HarnessMemoryPort,
    HarnessMemoryToolDescriptor, UnavailableHarnessMemoryPort,
};
pub use harness_filesystem::{
    FilesystemOperation, HarnessFilesystemCommand, HarnessFilesystemPort, HarnessFilesystemResult,
    HarnessSandboxCommand, HarnessSandboxLease, HarnessSandboxPort, HarnessSandboxSnapshot,
    UnavailableHarnessFilesystemPort, UnavailableHarnessSandboxPort,
};
pub use harness_plan::{
    HarnessPlanModeCommand, HarnessPlanModeManager, HarnessPlanModeOperation, HarnessPlanModeResult,
};
pub use harness_skill::{
    HarnessSkillLoadCommand, HarnessSkillPromptBuilder, HarnessSkillRepositoryPort,
    HarnessSkillRuntimeEntry, HarnessSkillSearchCommand, UnavailableHarnessSkillRepository,
};
pub use harness_subagent::{
    HarnessSubagentDelegationCommand, HarnessSubagentDelegationMode,
    HarnessSubagentDelegationResult, HarnessSubagentEvent, HarnessSubagentPort,
    HarnessSubagentTaskRecord, HarnessSubagentTaskRepositoryPort, UnavailableHarnessSubagentPort,
    UnavailableHarnessSubagentTaskRepository,
};

const MAX_WORKSPACE_SECTION_CHARS: usize = 32 * 1024;

/// Data-only workspace snapshot used by harness middleware.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceSnapshot {
    pub root_ref: Option<String>,
    pub agents_md: Option<String>,
    pub memory_md: Option<String>,
    pub knowledge_context: Option<String>,
    #[serde(default)]
    pub skills: Vec<HarnessSkillDeclaration>,
    #[serde(default)]
    pub subagents: Vec<HarnessSubagentDeclaration>,
    #[serde(default)]
    pub tool_declarations: Vec<String>,
}

/// Provider-neutral skill descriptor. The skill service owns loading,
/// governance, mutation, curation, and entitlement decisions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HarnessSkillDeclaration {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}

/// Provider-neutral subagent declaration. Task/execution-control services own
/// sync/background dispatch and durable task records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HarnessSubagentDeclaration {
    pub id: String,
    pub name: String,
    pub mode: Option<String>,
}

/// Workspace manager for already-authorized workspace context.
///
/// This manager never reads host paths. Runtime-host or service adapters can
/// populate snapshots after filesystem/sandbox/path-policy checks have run.
#[derive(Debug, Clone, Default)]
pub struct WorkspaceManager {
    snapshot: WorkspaceSnapshot,
}

impl WorkspaceManager {
    pub fn new(snapshot: WorkspaceSnapshot) -> Self {
        info!("harness workspace manager initialized from provider-neutral snapshot");
        Self { snapshot }
    }

    pub fn snapshot(&self) -> &WorkspaceSnapshot {
        &self.snapshot
    }

    /// Build the bounded system-prompt section injected before model calls.
    pub fn build_prompt_section(&self, runtime: &RuntimeContext) -> String {
        let mut section = String::new();
        section.push_str("## Workspace Context\n");
        if let Some(root_ref) = &self.snapshot.root_ref {
            section.push_str("Workspace reference: ");
            section.push_str(root_ref);
            section.push('\n');
        }
        section.push_str("Trace reference: ");
        section.push_str(&runtime.trace_id);
        section.push('\n');
        append_optional_block(&mut section, "AGENTS.md", &self.snapshot.agents_md);
        append_optional_block(&mut section, "MEMORY.md", &self.snapshot.memory_md);
        append_optional_block(&mut section, "Knowledge", &self.snapshot.knowledge_context);
        append_named_list(
            &mut section,
            "Skills",
            self.snapshot
                .skills
                .iter()
                .map(|skill| format!("{} ({})", skill.name, skill.id)),
        );
        append_named_list(
            &mut section,
            "Subagents",
            self.snapshot
                .subagents
                .iter()
                .map(|agent| format!("{} ({})", agent.name, agent.id)),
        );
        append_named_list(
            &mut section,
            "Tool declarations",
            self.snapshot.tool_declarations.iter().cloned(),
        );
        if section.len() > MAX_WORKSPACE_SECTION_CHARS {
            section.truncate(MAX_WORKSPACE_SECTION_CHARS);
            section.push_str("\n[workspace context truncated]\n");
        }
        section
    }
}

/// Middleware that appends workspace context to the system prompt.
pub struct WorkspaceContextMiddleware {
    workspace: Arc<WorkspaceManager>,
}

impl WorkspaceContextMiddleware {
    pub fn new(workspace: Arc<WorkspaceManager>) -> Self {
        Self { workspace }
    }
}

#[async_trait]
impl AgentMiddleware for WorkspaceContextMiddleware {
    fn name(&self) -> &str {
        "workspace_context"
    }

    fn supports(&self, stage: MiddlewareStage, phase: MiddlewarePhase) -> bool {
        matches!(stage, MiddlewareStage::SystemPrompt) && matches!(phase, MiddlewarePhase::Before)
    }

    async fn handle(&self, mut context: MiddlewareContext) -> AgentResult<MiddlewareContext> {
        debug!(
            trace_id = %context.runtime.trace_id,
            "harness workspace context middleware appending system prompt section"
        );
        let section = self.workspace.build_prompt_section(&context.runtime);
        let current = context.message.get_text();
        context.message = Msg::system(format!("{current}\n\n{section}"));
        Ok(context)
    }
}

/// Thin Harness wrapper over `ReActAgent`.
pub struct HarnessAgent {
    inner: ReActAgent,
    workspace: Arc<WorkspaceManager>,
}

impl HarnessAgent {
    pub fn new(
        name: impl Into<String>,
        sys_prompt: impl Into<String>,
        model: Arc<dyn ChatModel>,
        formatter: Arc<dyn Formatter>,
        workspace: WorkspaceManager,
    ) -> Self {
        let workspace = Arc::new(workspace);
        let mut middleware = AgentMiddlewareChain::new();
        middleware.push(Arc::new(WorkspaceContextMiddleware::new(workspace.clone())));
        let inner = ReActAgent::new(name, sys_prompt, model, formatter).with_middleware(middleware);
        Self { inner, workspace }
    }

    pub fn workspace(&self) -> &WorkspaceManager {
        &self.workspace
    }

    pub async fn run(&self, command: AgentRunCommand) -> AgentResult<AgentRunResult> {
        self.inner.run(command).await
    }
}

fn append_optional_block(section: &mut String, label: &str, value: &Option<String>) {
    if let Some(value) = value.as_ref().filter(|value| !value.trim().is_empty()) {
        section.push_str("\n### ");
        section.push_str(label);
        section.push('\n');
        section.push_str(value.trim());
        section.push('\n');
    }
}

fn append_named_list<I>(section: &mut String, label: &str, values: I)
where
    I: IntoIterator<Item = String>,
{
    let values = values.into_iter().collect::<Vec<_>>();
    if values.is_empty() {
        return;
    }
    section.push_str("\n### ");
    section.push_str(label);
    section.push('\n');
    for value in values {
        section.push_str("- ");
        section.push_str(&value);
        section.push('\n');
    }
}

#[cfg(test)]
#[path = "harness_tests.rs"]
mod harness_tests;
