// SPDX-License-Identifier: Apache-2.0
//
// Derived from AgentScope Java 2.0 concepts and APIs.
// Copyright 2024-2026 the original AgentScope author or authors.
// Licensed under the Apache License, Version 2.0.

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;

use super::*;
use crate::formatter::OpenAiFormatter;
use crate::message::{ContentBlock, Msg, MsgContent, TextBlock, ToolResultBlock, ToolUseBlock};
use crate::model::{ChatOptions, ChatResponse, ChatUsage, ModelError};
use crate::react_agent::{AgentRunCommand, GenerateReason};
use crate::runtime_context::{
    AgentSessionStore, AgentState, InMemoryAgentSessionStore, RuntimeContext, SessionKey,
};

struct CaptureHarnessModel {
    captured_messages: Mutex<Vec<serde_json::Value>>,
}

#[async_trait]
impl ChatModel for CaptureHarnessModel {
    async fn chat(
        &self,
        messages: Vec<serde_json::Value>,
        _options: &ChatOptions,
    ) -> Result<ChatResponse, ModelError> {
        *self.captured_messages.lock().await = messages;
        Ok(ChatResponse {
            content: vec![ContentBlock::Text(TextBlock {
                text: "harness done".to_string(),
            })],
            id: "harness-response".to_string(),
            created_at: String::new(),
            usage: ChatUsage::default(),
            metadata: None,
        })
    }

    fn name(&self) -> &str {
        "capture-harness-model"
    }
}

#[tokio::test]
async fn harness_agent_injects_workspace_context_through_system_prompt_middleware() {
    let model = Arc::new(CaptureHarnessModel {
        captured_messages: Mutex::new(Vec::new()),
    });
    let workspace = WorkspaceManager::new(WorkspaceSnapshot {
        root_ref: Some("workspace://tenant/app/session".to_string()),
        agents_md: Some("Follow workspace conventions.".to_string()),
        memory_md: Some("Remember prior decisions.".to_string()),
        knowledge_context: Some("Knowledge catalog is available.".to_string()),
        skills: vec![HarnessSkillDeclaration {
            id: "skill://generic/debug".to_string(),
            name: "debug".to_string(),
            description: None,
        }],
        subagents: vec![HarnessSubagentDeclaration {
            id: "agent://reviewer".to_string(),
            name: "reviewer".to_string(),
            mode: Some("sync".to_string()),
        }],
        tool_declarations: vec!["tool://mcp/search".to_string()],
    });
    let agent = HarnessAgent::new(
        "harness",
        "base system",
        model.clone(),
        Arc::new(OpenAiFormatter),
        workspace,
    );

    let result = agent
        .run(AgentRunCommand {
            input: Msg::user("user", "hello"),
            runtime: RuntimeContext::new("trace-harness"),
            state: AgentState::empty(),
        })
        .await
        .unwrap();

    assert_eq!(result.generate_reason, GenerateReason::EndTurn);
    assert_eq!(result.final_message.get_text(), "harness done");
    let formatted = serde_json::to_string(&*model.captured_messages.lock().await).unwrap();
    assert!(formatted.contains("base system"));
    assert!(formatted.contains("Workspace Context"));
    assert!(formatted.contains("Follow workspace conventions."));
    assert!(formatted.contains("Remember prior decisions."));
    assert!(formatted.contains("skill://generic/debug"));
    assert!(formatted.contains("agent://reviewer"));
}

#[test]
fn workspace_manager_bounds_large_context_sections() {
    let workspace = WorkspaceManager::new(WorkspaceSnapshot {
        agents_md: Some("x".repeat(MAX_WORKSPACE_SECTION_CHARS * 2)),
        ..Default::default()
    });

    let section = workspace.build_prompt_section(&RuntimeContext::new("trace-harness-bound"));
    assert!(section.len() <= MAX_WORKSPACE_SECTION_CHARS + 40);
    assert!(section.contains("workspace context truncated"));
}

#[test]
fn harness_context_manager_evicts_tool_results_and_truncates_arguments() {
    let mut state = AgentState::empty();
    state.push_message(Msg::assistant(
        "assistant",
        MsgContent::Blocks(vec![
            ContentBlock::ToolUse(ToolUseBlock {
                id: "call-large-args".to_string(),
                name: "generic_tool".to_string(),
                input: serde_json::json!({ "payload": "x".repeat(128) }),
                raw_input: None,
            }),
            ContentBlock::ToolResult(ToolResultBlock {
                tool_use_id: "call-large-result".to_string(),
                output: "y".repeat(128),
                name: Some("generic_tool".to_string()),
                is_error: false,
            }),
        ]),
    ));
    let manager = HarnessContextManager::new(HarnessContextPolicy {
        max_context_chars: 1024,
        max_tool_result_chars: 16,
        max_tool_argument_chars: 16,
    });

    let result = manager
        .compact(&RuntimeContext::new("trace-harness-compact"), &state)
        .unwrap();

    assert_eq!(result.evicted_tool_results, vec!["call-large-result"]);
    assert_eq!(result.truncated_tool_arguments, vec!["call-large-args"]);
    assert!(!result.overflow_retry_required);
    let encoded = serde_json::to_string(&result.state.conversation).unwrap();
    assert!(encoded.contains("tool result evicted"));
    assert!(encoded.contains("\"truncated\":true"));
}

#[test]
fn harness_context_manager_marks_overflow_retry_when_budget_still_exceeded() {
    let mut state = AgentState::empty();
    state.push_message(Msg::assistant(
        "assistant",
        MsgContent::Blocks(vec![ContentBlock::Text(TextBlock {
            text: "z".repeat(128),
        })]),
    ));
    let manager = HarnessContextManager::new(HarnessContextPolicy {
        max_context_chars: 32,
        max_tool_result_chars: 16,
        max_tool_argument_chars: 16,
    });

    let result = manager
        .compact(&RuntimeContext::new("trace-harness-overflow"), &state)
        .unwrap();

    assert!(result.overflow_retry_required);
}

#[tokio::test]
async fn unavailable_harness_memory_port_returns_structured_result() {
    let port = UnavailableHarnessMemoryPort::new("memory service disabled");
    let result = port
        .flush(HarnessMemoryFlushCommand {
            runtime: RuntimeContext::new("trace-harness-memory"),
            state: AgentState::empty(),
        })
        .await
        .unwrap();

    assert!(!result.flushed);
    assert_eq!(result.reason.as_deref(), Some("memory service disabled"));
    assert!(HarnessMemoryToolDescriptor::default_descriptors()
        .iter()
        .any(|descriptor| descriptor.capability_id == "harness.memory.flush"));
}

#[tokio::test]
async fn unavailable_harness_filesystem_and_sandbox_ports_fail_closed() {
    let filesystem = UnavailableHarnessFilesystemPort::new("filesystem service disabled");
    let fs_result = filesystem
        .execute(HarnessFilesystemCommand {
            runtime: RuntimeContext::new("trace-harness-fs"),
            operation: FilesystemOperation::Read,
            path: "workspace://authorized/file.txt".to_string(),
            destination_path: None,
            content: None,
        })
        .await
        .unwrap();
    assert!(!fs_result.success);
    assert_eq!(
        fs_result.reason.as_deref(),
        Some("filesystem service disabled")
    );

    let sandbox = UnavailableHarnessSandboxPort::new("sandbox module absent");
    let command = HarnessSandboxCommand {
        runtime: RuntimeContext::new("trace-harness-sandbox"),
        workspace_ref: Some("workspace://authorized".to_string()),
        isolation_key: "tenant/app/session".to_string(),
    };
    assert!(sandbox.acquire(command.clone()).await.unwrap().is_none());
    let snapshot = sandbox.snapshot(command).await.unwrap();
    assert!(!snapshot.available);
    assert_eq!(snapshot.reason.as_deref(), Some("sandbox module absent"));
}

#[tokio::test]
async fn harness_skill_prompt_and_unavailable_repository_are_provider_neutral() {
    let prompt = HarnessSkillPromptBuilder::build(&[HarnessSkillRuntimeEntry {
        skill_id: "skill://generic/review".to_string(),
        name: "review".to_string(),
        description: "Review generic task output.".to_string(),
        prompt: Some("Use only authorized context.".to_string()),
        evidence_ref: Some("evidence://skill/review".to_string()),
    }]);
    assert!(prompt.contains("Available Skills"));
    assert!(prompt.contains("skill://generic/review"));
    assert!(prompt.contains("Use only authorized context."));

    let repository = UnavailableHarnessSkillRepository::new("skill service disabled");
    let found = repository
        .search(HarnessSkillSearchCommand {
            runtime: RuntimeContext::new("trace-harness-skill-search"),
            query: Some("review".to_string()),
            limit: 5,
        })
        .await
        .unwrap();
    assert!(found.is_empty());
    let loaded = repository
        .load(HarnessSkillLoadCommand {
            runtime: RuntimeContext::new("trace-harness-skill-load"),
            skill_id: "skill://generic/review".to_string(),
        })
        .await
        .unwrap();
    assert!(loaded.is_none());
}

#[tokio::test]
async fn unavailable_harness_subagent_ports_return_structured_absence() {
    let port = UnavailableHarnessSubagentPort::new("subagent service disabled");
    let result = port
        .delegate(HarnessSubagentDelegationCommand {
            runtime: RuntimeContext::new("trace-harness-subagent"),
            subagent_id: "agent://reviewer".to_string(),
            mode: HarnessSubagentDelegationMode::Background,
            input: Msg::user("user", "review this"),
        })
        .await
        .unwrap();
    assert!(result.task_id.is_none());
    assert_eq!(result.reason.as_deref(), Some("subagent service disabled"));

    let repository = UnavailableHarnessSubagentTaskRepository::new("task repository disabled");
    repository
        .save(
            RuntimeContext::new("trace-harness-subagent-task-save"),
            HarnessSubagentTaskRecord {
                task_id: "task-1".to_string(),
                subagent_id: "agent://reviewer".to_string(),
                event_source_id: "event-source://task-1".to_string(),
                status: "pending".to_string(),
            },
        )
        .await
        .unwrap();
    let record = repository
        .get(
            RuntimeContext::new("trace-harness-subagent-task-get"),
            "task-1",
        )
        .await
        .unwrap();
    assert!(record.is_none());
}

#[test]
fn harness_plan_mode_is_agent_local_and_preserves_task_board_boundary() {
    let entered = HarnessPlanModeManager::apply(HarnessPlanModeCommand {
        runtime: RuntimeContext::new("trace-harness-plan"),
        state: AgentState::empty(),
        operation: HarnessPlanModeOperation::Enter {
            plan_id: Some("plan-local".to_string()),
        },
    })
    .unwrap();
    assert!(entered.state.plan_mode.enabled);
    assert_eq!(
        entered.state.plan_mode.plan_id.as_deref(),
        Some("plan-local")
    );
    assert!(entered.task_board_boundary_preserved);

    let noted = HarnessPlanModeManager::apply(HarnessPlanModeCommand {
        runtime: RuntimeContext::new("trace-harness-plan-note"),
        state: entered.state,
        operation: HarnessPlanModeOperation::AddNote {
            note: "Local note only; no TaskBoard mutation.".to_string(),
        },
    })
    .unwrap();
    assert_eq!(noted.state.plan_mode.notes.len(), 1);

    let exited = HarnessPlanModeManager::apply(HarnessPlanModeCommand {
        runtime: RuntimeContext::new("trace-harness-plan-exit"),
        state: noted.state,
        operation: HarnessPlanModeOperation::Exit,
    })
    .unwrap();
    assert!(!exited.state.plan_mode.enabled);
    assert!(exited.evidence_ref.starts_with("harness.plan_mode://"));
}

#[tokio::test]
async fn harness_session_state_uses_provider_neutral_session_store() {
    let store = InMemoryAgentSessionStore::new();
    let key = SessionKey::new("tenant", "app", "session", "harness-agent").unwrap();
    let mut state = AgentState::empty();
    state.plan_mode.enabled = true;
    state.push_message(Msg::user("user", "persist harness state"));

    store.save_state(&key, state).await.unwrap();
    let loaded = store
        .load_state(&key)
        .await
        .unwrap()
        .expect("harness state should load from provider-neutral session store");

    assert!(loaded.plan_mode.enabled);
    assert_eq!(loaded.conversation.len(), 1);
}
