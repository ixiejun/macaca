//! Turn-level session model helpers (mutable turn list operations).
//!
//! Converts between legacy `messages` arrays and richer `turns` with execution metadata.

use macaca_proto::LlmMessage;
use macaca_runtime_host::executor::ExecutorEvent;

use super::types::{AssistantExecutionMeta, StoredSession, StoredTraceStep, StoredTurn, AgentTrace};

pub(crate) fn ensure_running_assistant_turn(turns: &mut Vec<StoredTurn>) -> &mut StoredTurn {
    let has_running_idx = turns.iter().rposition(|turn| {
        turn.role == "assistant"
            && matches!(turn.status.as_deref(), Some("running") | Some("pending"))
    });

    let idx = match has_running_idx {
        Some(idx) => idx,
        None => {
            turns.push(StoredTurn {
                role: "assistant".into(),
                content: String::new(),
                status: Some("running".into()),
                trace_steps: Vec::new(),
                meta: None,
                agent_traces: std::collections::HashMap::new(),
            });
            turns.len() - 1
        }
    };

    &mut turns[idx]
}

pub(crate) fn session_status_from_executor_event(event: &ExecutorEvent) -> Option<&'static str> {
    match event {
        ExecutorEvent::TaskStarted { .. } => Some("running"),
        ExecutorEvent::TaskCompleted { .. } => Some("completed"),
        ExecutorEvent::TaskFailed { .. } => Some("failed"),
        ExecutorEvent::TaskCancelled { .. } => Some("cancelled"),
        ExecutorEvent::HookEvent { event: hook_event } => {
            use macaca_runtime_host::executor::fork_manager::HookEvent;
            match hook_event {
                HookEvent::ForkMerged { .. } => Some("completed"),
                HookEvent::DelegateFailed { .. } => Some("failed"),
                HookEvent::DelegateCompleted { .. } => Some("running"),
                HookEvent::ForkCreated { .. } => Some("running"),
                HookEvent::ForkValidated { .. } => Some("running"),
                _ => None,
            }
        }
        _ => None,
    }
}
pub(crate) fn build_turns_from_messages(messages: &[LlmMessage]) -> Vec<StoredTurn> {
    messages
        .iter()
        .filter_map(|msg| match msg.role {
            macaca_proto::LlmRole::User => Some(StoredTurn {
                role: "user".into(),
                content: msg.content.clone(),
                status: None,
                trace_steps: Vec::new(),
                meta: None,
                agent_traces: std::collections::HashMap::new(),
            }),
            macaca_proto::LlmRole::Assistant => Some(StoredTurn {
                role: "assistant".into(),
                content: msg.content.clone(),
                status: Some("completed".into()),
                trace_steps: Vec::new(),
                meta: None,
                agent_traces: std::collections::HashMap::new(),
            }),
            _ => None,
        })
        .collect()
}

pub(crate) fn stored_turns_or_messages(stored: &StoredSession) -> Vec<StoredTurn> {
    if stored.turns.is_empty() {
        build_turns_from_messages(&stored.messages)
    } else {
        stored.turns.clone()
    }
}
