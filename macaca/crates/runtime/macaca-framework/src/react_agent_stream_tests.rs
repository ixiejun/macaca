// SPDX-License-Identifier: Apache-2.0
//
// Derived from AgentScope Java 2.0 concepts and APIs.
// Copyright 2024-2026 the original AgentScope author or authors.
// Licensed under the Apache License, Version 2.0.

use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;
use crate::formatter::OpenAiFormatter;
use crate::permission_contract::ToolPermissionDecision;

#[path = "react_agent_test_support.rs"]
mod support;
use support::*;

#[tokio::test]
async fn react_agent_streams_final_text_events() {
    let model = fake_model(vec![text_response("done")]);
    let agent = ReActAgent::new("agent", "system", model, Arc::new(OpenAiFormatter));

    let result = agent
        .run(AgentRunCommand {
            input: Msg::user("user", "hello"),
            runtime: RuntimeContext::new("trace-react2"),
            state: AgentState::empty(),
        })
        .await
        .unwrap();

    assert_eq!(result.final_message.get_text(), "done");
    assert_eq!(result.generate_reason, GenerateReason::EndTurn);
    assert!(result
        .events
        .iter()
        .any(|event| { event.event_type == AgentEventType::AgentStart }));
    assert!(result
        .events
        .iter()
        .any(|event| { event.event_type == AgentEventType::TextBlockDelta }));
}

#[tokio::test]
async fn react_agent_streams_tool_call_and_result_events() {
    let mut toolkit = Toolkit::new();
    toolkit.register(Box::new(EchoTool), None);
    let model = fake_model(vec![tool_response(), text_response("done")]);
    let agent = ReActAgent::new("agent", "system", model.clone(), Arc::new(OpenAiFormatter))
        .with_toolkit(toolkit);

    let result = agent
        .run(AgentRunCommand {
            input: Msg::user("user", "run"),
            runtime: RuntimeContext::new("trace-react2-tool"),
            state: AgentState::empty(),
        })
        .await
        .unwrap();

    assert_eq!(model.call_count.load(Ordering::SeqCst), 2);
    assert_eq!(result.final_message.get_text(), "done");
    assert!(result
        .events
        .iter()
        .any(|event| { event.event_type == AgentEventType::ToolCallStart }));
    assert!(result
        .events
        .iter()
        .any(|event| { event.event_type == AgentEventType::ToolResultTextDelta }));
    assert_eq!(result.state.conversation.len(), 4);
}

#[tokio::test]
async fn react_agent_trace_guard_denies_before_tool_side_effect() {
    let executions = Arc::new(AtomicUsize::new(0));
    let mut toolkit = Toolkit::new();
    toolkit.register(
        Box::new(CountingTool {
            executions: executions.clone(),
        }),
        None,
    );
    let model = fake_model(vec![tool_response(), text_response("done")]);
    let agent =
        ReActAgent::new("agent", "system", model, Arc::new(OpenAiFormatter)).with_toolkit(toolkit);

    let result = agent
        .run(AgentRunCommand {
            input: Msg::user("user", "run"),
            runtime: RuntimeContext::new(""),
            state: AgentState::empty(),
        })
        .await
        .unwrap();

    assert_eq!(executions.load(Ordering::SeqCst), 0);
    assert_eq!(result.final_message.get_text(), "done");
    assert!(result.state.conversation.iter().any(|message| {
        matches!(
            &message.content,
            MsgContent::Blocks(blocks) if blocks.iter().any(|block| {
                matches!(
                    block,
                    ContentBlock::ToolResult(result)
                        if result.is_error && result.output.contains("requires trace_id")
                )
            })
        )
    }));
}

#[tokio::test]
async fn react_agent_reports_max_iterations() {
    let model = fake_model(vec![tool_response()]);
    let agent =
        ReActAgent::new("agent", "system", model, Arc::new(OpenAiFormatter)).with_max_iters(1);

    let result = agent
        .run(AgentRunCommand {
            input: Msg::user("user", "loop"),
            runtime: RuntimeContext::new("trace-react2-max"),
            state: AgentState::empty(),
        })
        .await
        .unwrap();

    assert_eq!(result.generate_reason, GenerateReason::MaxIterations);
    assert!(result
        .events
        .iter()
        .any(|event| { event.event_type == AgentEventType::ExceedMaxIters }));
}

#[tokio::test]
async fn react_agent_suspends_for_user_confirmation_before_tool_side_effect() {
    let executions = Arc::new(AtomicUsize::new(0));
    let mut toolkit = Toolkit::new();
    toolkit.register(
        Box::new(CountingTool {
            executions: executions.clone(),
        }),
        None,
    );
    let model = fake_model(vec![tool_response()]);
    let agent = ReActAgent::new("agent", "system", model, Arc::new(OpenAiFormatter))
        .with_toolkit(toolkit)
        .with_permission_policy(Arc::new(StaticPermissionPolicy {
            decision: ToolPermissionDecision::AskUser {
                request_id: "confirm-1".to_string(),
                prompt: "approve tool".to_string(),
            },
        }));

    let result = agent
        .run(AgentRunCommand {
            input: Msg::user("user", "run"),
            runtime: RuntimeContext::new("trace-react2-confirm"),
            state: AgentState::empty(),
        })
        .await
        .unwrap();

    assert_eq!(executions.load(Ordering::SeqCst), 0);
    assert_eq!(result.generate_reason, GenerateReason::UserConfirmRequired);
    assert!(matches!(
        result.state.permission,
        crate::runtime_context::PermissionState::WaitingForUserConfirm { .. }
    ));
    assert!(result
        .events
        .iter()
        .any(|event| { event.event_type == AgentEventType::RequireUserConfirm }));
}

#[tokio::test]
async fn react_agent_denies_before_tool_side_effect() {
    let executions = Arc::new(AtomicUsize::new(0));
    let mut toolkit = Toolkit::new();
    toolkit.register(
        Box::new(CountingTool {
            executions: executions.clone(),
        }),
        None,
    );
    let model = fake_model(vec![tool_response()]);
    let agent = ReActAgent::new("agent", "system", model, Arc::new(OpenAiFormatter))
        .with_toolkit(toolkit)
        .with_permission_policy(Arc::new(StaticPermissionPolicy {
            decision: ToolPermissionDecision::Deny {
                reason: "policy denied".to_string(),
            },
        }));

    let result = agent
        .run(AgentRunCommand {
            input: Msg::user("user", "run"),
            runtime: RuntimeContext::new("trace-react2-deny"),
            state: AgentState::empty(),
        })
        .await
        .unwrap();

    assert_eq!(executions.load(Ordering::SeqCst), 0);
    assert_eq!(result.generate_reason, GenerateReason::Denied);
    assert!(matches!(
        result.state.permission,
        crate::runtime_context::PermissionState::Denied { .. }
    ));
    assert!(result
        .events
        .iter()
        .any(|event| { event.event_type == AgentEventType::RequestStop }));
}

#[tokio::test]
async fn react_agent_suspends_for_external_execution_before_tool_side_effect() {
    let executions = Arc::new(AtomicUsize::new(0));
    let mut toolkit = Toolkit::new();
    toolkit.register(
        Box::new(CountingTool {
            executions: executions.clone(),
        }),
        None,
    );
    let model = fake_model(vec![tool_response()]);
    let agent = ReActAgent::new("agent", "system", model, Arc::new(OpenAiFormatter))
        .with_toolkit(toolkit)
        .with_permission_policy(Arc::new(StaticPermissionPolicy {
            decision: ToolPermissionDecision::RequireExternalExecution {
                request_id: "external-1".to_string(),
            },
        }));

    let result = agent
        .run(AgentRunCommand {
            input: Msg::user("user", "run"),
            runtime: RuntimeContext::new("trace-react2-external"),
            state: AgentState::empty(),
        })
        .await
        .unwrap();

    assert_eq!(executions.load(Ordering::SeqCst), 0);
    assert_eq!(
        result.generate_reason,
        GenerateReason::ExternalExecutionRequired
    );
    assert!(matches!(
        result.state.permission,
        crate::runtime_context::PermissionState::WaitingForExternalExecution { .. }
    ));
    assert!(result
        .events
        .iter()
        .any(|event| { event.event_type == AgentEventType::RequireExternalExecution }));
}

#[tokio::test]
async fn react_agent_retries_primary_model_before_success() {
    let model = retry_model(1);
    let agent = ReActAgent::new("agent", "system", model.clone(), Arc::new(OpenAiFormatter))
        .with_model_retry_limit(1);

    let result = agent
        .run(AgentRunCommand {
            input: Msg::user("user", "retry"),
            runtime: RuntimeContext::new("trace-react2-retry"),
            state: AgentState::empty(),
        })
        .await
        .unwrap();

    assert_eq!(model.call_count.load(Ordering::SeqCst), 2);
    assert_eq!(result.final_message.get_text(), "retried");
    assert_eq!(result.generate_reason, GenerateReason::EndTurn);
}

#[tokio::test]
async fn react_agent_uses_fallback_model_after_retry_budget() {
    let primary = retry_model(10);
    let fallback = fake_model(vec![text_response("fallback")]);
    let agent = ReActAgent::new(
        "agent",
        "system",
        primary.clone(),
        Arc::new(OpenAiFormatter),
    )
    .with_model_retry_limit(0)
    .with_fallback_model(fallback.clone());

    let result = agent
        .run(AgentRunCommand {
            input: Msg::user("user", "fallback"),
            runtime: RuntimeContext::new("trace-react2-fallback"),
            state: AgentState::empty(),
        })
        .await
        .unwrap();

    assert_eq!(primary.call_count.load(Ordering::SeqCst), 1);
    assert_eq!(fallback.call_count.load(Ordering::SeqCst), 1);
    assert_eq!(result.final_message.get_text(), "fallback");
}

#[tokio::test]
async fn react_agent_attaches_structured_output_reminder_and_options() {
    let model = Arc::new(CaptureModel {
        captured_messages: Mutex::new(Vec::new()),
        captured_options: Mutex::new(None),
    });
    let config = agent_execution_config::AgentExecutionConfig::default()
        .with_generation_options(Some(0.2), Some(128), Some(0.9))
        .with_structured_output_reminder("Return a provider-neutral JSON object.");
    let agent = ReActAgent::new("agent", "system", model.clone(), Arc::new(OpenAiFormatter))
        .with_execution_config(config);

    let result = agent
        .run(AgentRunCommand {
            input: Msg::user("user", "structured"),
            runtime: RuntimeContext::new("trace-react2-structured"),
            state: AgentState::empty(),
        })
        .await
        .unwrap();

    let messages = model.captured_messages.lock().await;
    let options = model.captured_options.lock().await.clone().unwrap();
    let formatted = serde_json::to_string(&*messages).unwrap();
    assert!(formatted.contains("Return a provider-neutral JSON object."));
    assert_eq!(options.temperature, Some(0.2));
    assert_eq!(options.max_tokens, Some(128));
    assert_eq!(options.top_p, Some(0.9));
    assert_eq!(result.final_message.get_text(), "captured");
}

#[tokio::test]
async fn react_agent_graceful_shutdown_finishes_interrupted() {
    let model = fake_model(vec![text_response("should-not-run")]);
    let agent = ReActAgent::new("agent", "system", model.clone(), Arc::new(OpenAiFormatter));
    agent.request_graceful_shutdown();

    let result = agent
        .run(AgentRunCommand {
            input: Msg::user("user", "stop"),
            runtime: RuntimeContext::new("trace-react2-shutdown"),
            state: AgentState::empty(),
        })
        .await
        .unwrap();

    assert_eq!(model.call_count.load(Ordering::SeqCst), 0);
    assert_eq!(result.generate_reason, GenerateReason::Interrupted);
    assert!(result
        .events
        .iter()
        .any(|event| event.event_type == AgentEventType::Interrupted));
}

#[tokio::test]
async fn react_agent_resumes_pending_external_execution_result() {
    let model = fake_model(vec![text_response("continued")]);
    let mut state = AgentState::empty();
    state.permission = crate::runtime_context::PermissionState::WaitingForExternalExecution {
        request_id: "external-1".to_string(),
        tool_call_id: Some("call-1".to_string()),
    };
    let runtime = RuntimeContext::new("trace-react2-external-resume")
        .with_extra(
            agent_execution_config::EXTERNAL_EXECUTION_RESULT_EXTRA,
            serde_json::json!({
                "request_id": "external-1",
                "success": true,
                "output": "external output"
            }),
        )
        .unwrap();
    let agent = ReActAgent::new("agent", "system", model, Arc::new(OpenAiFormatter));

    let result = agent
        .run(AgentRunCommand {
            input: Msg::user("user", "resume"),
            runtime,
            state,
        })
        .await
        .unwrap();

    assert_eq!(result.generate_reason, GenerateReason::EndTurn);
    assert!(matches!(
        result.state.permission,
        crate::runtime_context::PermissionState::Clear
    ));
    assert!(result
        .events
        .iter()
        .any(|event| event.event_type == AgentEventType::ExternalExecutionResult));
    assert!(result.state.conversation.iter().any(|message| {
        matches!(
            &message.content,
            MsgContent::Blocks(blocks) if blocks.iter().any(|block| {
                matches!(
                    block,
                    ContentBlock::ToolResult(result) if result.output.contains("external output")
                )
            })
        )
    }));
}
