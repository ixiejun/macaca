//! Workflow prompt **Strategy** implementations and tool-policy renderer.
//!
//! Strategies assemble system prompts from structured [`WorkflowPromptParts`].
//! The default strategy preserves legacy SDD wording while delegating tool rules
//! to manifest-derived policy via [`render_tool_policy_block`].

use super::types::{WorkflowPromptContext, WorkflowPromptParts};

/// Strategy interface for rendering workflow system prompts.
///
/// Implementations may swap prompt sections without changing engine control flow
/// (Open/Closed: extend by adding new strategies, not editing the engine).
pub trait WorkflowPromptStrategy: Send + Sync {
    /// Produce structured prompt sections for the given context.
    fn prompt_parts(&self, ctx: &WorkflowPromptContext) -> WorkflowPromptParts;

    /// Render the full system prompt by concatenating [`Self::prompt_parts`]
    /// and optional [`WorkflowPromptContext::additional_context`].
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

/// Default prompt strategy preserving the historical SDD workflow text.
///
/// When persona files are absent, callers fall back to
/// [`super::engine::WorkflowEngine::default_assistant_prompt_with_context`].
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

/// Render the tool-policy section from manifest coordinator declarations.
///
/// When execution tools are declared on the coordinator agent, those names drive
/// the prompt.  A `legacy_default_execute_tool` is only used when the manifest
/// omits execution tools (skeleton / compatibility paths).
pub(super) fn render_tool_policy_block(
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
        // `file_write` is a generic workspace capability, not an execution
        // driver bypass.  Applications may author files through this tool, but
        // writes must stay inside the delegated workspace for auditability.
        lines.push(
            "- Use `file_write` only for application-declared workspace paths or explicit evidence artifact paths"
                .into(),
        );
        lines.push(
            "- Never write outside the delegated workspace, and report any path or policy denial"
                .into(),
        );
    } else {
        lines.push("- NEVER write source code directly".into());
    }
    lines.join("\n")
}

/// Assistant fallback prompt when no persona directory exists for the coordinator.
pub(super) fn default_assistant_prompt_with_context(ctx: &WorkflowPromptContext) -> String {
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
