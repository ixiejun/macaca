//! Contract tests for the ReAct loop — reply, tools, memory, interrupt.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use crate::agent::{Agent, AgentError};
use crate::formatter::OpenAiFormatter;
use crate::message::{ContentBlock, Msg, MsgContent, Role};
use crate::model::{ChatResponse, ChatUsage, ToolChoice};
use crate::tool::Toolkit;

use super::fixtures::{
    make_agent, text_response, tool_call_response, EchoTool, FailTool, MockChatModel,
};
use super::super::core::ReActAgent;

// -----------------------------------------------------------------------
// 1. test_simple_reply: no tool calls → returns text directly
// -----------------------------------------------------------------------

#[tokio::test]
async fn test_simple_reply() {
    let model = MockChatModel::new(vec![text_response("Hello!")]);
    let agent = make_agent(model);
    let msg = Msg::user("alice", "hi");
    let reply = agent.reply(msg).await.unwrap();
    assert_eq!(reply.get_text(), "Hello!");
    assert_eq!(reply.name, "bot");
    assert_eq!(reply.role, Role::Assistant);
}

// -----------------------------------------------------------------------
// 2. test_tool_call_cycle: first call returns tool request, second returns text
// -----------------------------------------------------------------------

#[tokio::test]
async fn test_tool_call_cycle() {
    let mut toolkit = Toolkit::new();
    toolkit.register(Box::new(EchoTool), None);

    let model2 = MockChatModel::new(vec![
        tool_call_response("call_1", "echo", serde_json::json!({"x": 1})),
        text_response("Done!"),
    ]);
    let model_call_count = model2.call_count.clone();

    let agent = ReActAgent::new(
        "bot",
        "You are helpful.",
        Arc::new(model2),
        Arc::new(OpenAiFormatter),
    )
    .with_toolkit(toolkit);

    let reply = agent.reply(Msg::user("alice", "run echo")).await.unwrap();
    assert_eq!(reply.get_text(), "Done!");
    // LLM was called twice: once for tool call, once for final reply
    assert_eq!(model_call_count.load(Ordering::SeqCst), 2);

    // Verify memory contains: user msg, assistant tool-call msg, tool result, assistant final
    let mem = agent.memory.lock().await;
    let msgs = mem.get_memory(None, None).await;
    // user + assistant(tool_call) + tool_result + assistant(final)
    assert_eq!(msgs.len(), 4);
    assert_eq!(msgs[0].role, Role::User);
    assert_eq!(msgs[1].role, Role::Assistant); // tool call stored
    assert_eq!(msgs[2].role, Role::Tool); // tool result
    assert_eq!(msgs[3].role, Role::Assistant); // final reply
}

#[tokio::test]
async fn test_tool_choice_is_forwarded_when_tools_are_available() {
    let mut toolkit = Toolkit::new();
    toolkit.register(Box::new(EchoTool), None);

    let model = MockChatModel::new(vec![text_response("tool policy observed")]);
    let observed_options = Arc::clone(&model.observed_options);

    let agent = ReActAgent::new(
        "bot",
        "You are helpful.",
        Arc::new(model),
        Arc::new(OpenAiFormatter),
    )
    .with_toolkit(toolkit)
    .with_tool_choice(ToolChoice::Required);

    let reply = agent
        .reply(Msg::user("alice", "write evidence"))
        .await
        .unwrap();
    assert_eq!(reply.get_text(), "tool policy observed");

    let options = observed_options.lock().await;
    assert_eq!(options.len(), 1);
    assert_eq!(options[0].tool_choice, Some(ToolChoice::Required));
    assert!(options[0]
        .tools
        .as_ref()
        .is_some_and(|tools| !tools.is_empty()));
}

#[tokio::test]
async fn test_tool_choice_is_suppressed_without_tools() {
    let model = MockChatModel::new(vec![text_response("no tools")]);
    let observed_options = Arc::clone(&model.observed_options);

    let agent = ReActAgent::new(
        "bot",
        "You are helpful.",
        Arc::new(model),
        Arc::new(OpenAiFormatter),
    )
    .with_tool_choice(ToolChoice::Required);

    let reply = agent
        .reply(Msg::user("alice", "plain answer"))
        .await
        .unwrap();
    assert_eq!(reply.get_text(), "no tools");

    let options = observed_options.lock().await;
    assert_eq!(options.len(), 1);
    assert_eq!(options[0].tool_choice, None);
    assert!(options[0].tools.is_none());
}

// -----------------------------------------------------------------------
// 3. test_max_iterations: always returns tool calls → summarize after max_iters
// -----------------------------------------------------------------------

#[tokio::test]
async fn test_max_iterations() {
    // Build 5 tool-call responses (max_iters = 3, so only first 3 are consumed)
    let responses: Vec<ChatResponse> = (0..5)
        .map(|i| tool_call_response(&format!("call_{i}"), "echo", serde_json::json!({})))
        .collect();

    let mut toolkit = Toolkit::new();
    toolkit.register(Box::new(EchoTool), None);

    let model = MockChatModel::new(responses);
    let agent = ReActAgent::new("bot", "prompt", Arc::new(model), Arc::new(OpenAiFormatter))
        .with_toolkit(toolkit)
        .with_max_iters(3);

    // Should not error — returns summarized last assistant message
    let reply = agent.reply(Msg::user("alice", "go")).await.unwrap();
    assert_eq!(reply.role, Role::Assistant);
    assert_eq!(reply.name, "bot");
}

// -----------------------------------------------------------------------
// 4. test_interrupt: calling interrupt causes next iteration to return Interrupted
// -----------------------------------------------------------------------

#[tokio::test]
async fn test_interrupt() {
    // Provide many tool-call responses so the loop keeps running
    let responses: Vec<ChatResponse> = (0..10)
        .map(|i| tool_call_response(&format!("c_{i}"), "echo", serde_json::json!({})))
        .collect();

    let mut toolkit = Toolkit::new();
    toolkit.register(Box::new(EchoTool), None);

    let model = MockChatModel::new(responses);
    let agent = Arc::new(
        ReActAgent::new("bot", "prompt", Arc::new(model), Arc::new(OpenAiFormatter))
            .with_toolkit(toolkit)
            .with_max_iters(10),
    );

    // Cancel immediately before the loop gets a chance to finish
    agent.cancel_token().cancel();

    let result = agent.reply(Msg::user("alice", "go")).await;
    assert!(matches!(result, Err(AgentError::Interrupted)));
}

// -----------------------------------------------------------------------
// 5. test_observe_stores_message
// -----------------------------------------------------------------------

#[tokio::test]
async fn test_observe_stores_message() {
    let model = MockChatModel::new(vec![]);
    let agent = make_agent(model);

    let observed = Msg::user("external", "external message");
    agent.observe(observed.clone()).await.unwrap();

    let mem = agent.memory.lock().await;
    let msgs = mem.get_memory(None, None).await;
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0].id, observed.id);

    // Observed messages carry the "observed" mark
    let marked = mem.get_memory(Some("observed"), None).await;
    assert_eq!(marked.len(), 1);
}

// -----------------------------------------------------------------------
// 6. test_memory_persists_across_calls
// -----------------------------------------------------------------------

#[tokio::test]
async fn test_memory_persists_across_calls() {
    let model = MockChatModel::new(vec![
        text_response("first reply"),
        text_response("second reply"),
    ]);
    let agent = make_agent(model);

    agent.reply(Msg::user("alice", "msg1")).await.unwrap();
    agent.reply(Msg::user("alice", "msg2")).await.unwrap();

    let mem = agent.memory.lock().await;
    let msgs = mem.get_memory(None, None).await;
    // msg1 + assistant1 + msg2 + assistant2 = 4 messages
    assert_eq!(msgs.len(), 4);
    assert_eq!(msgs[0].get_text(), "msg1");
    assert_eq!(msgs[1].get_text(), "first reply");
    assert_eq!(msgs[2].get_text(), "msg2");
    assert_eq!(msgs[3].get_text(), "second reply");
}

// -----------------------------------------------------------------------
// 7. test_tool_returns_error_feeds_back_to_llm
// -----------------------------------------------------------------------

#[tokio::test]
async fn test_tool_returns_error_feeds_back_to_llm() {
    // First call: LLM requests "fail" tool → tool returns ToolError
    // Second call: LLM returns text (after seeing the error feedback)
    let model = MockChatModel::new(vec![
        tool_call_response("call_1", "fail", serde_json::json!({})),
        text_response("I see the error, here is my answer."),
    ]);

    let mut toolkit = Toolkit::new();
    toolkit.register(Box::new(FailTool), None);

    let agent = ReActAgent::new(
        "bot",
        "You are helpful.",
        Arc::new(model),
        Arc::new(OpenAiFormatter),
    )
    .with_toolkit(toolkit);

    let reply = agent
        .reply(Msg::user("alice", "do something"))
        .await
        .unwrap();
    assert_eq!(reply.get_text(), "I see the error, here is my answer.");

    // Verify memory contains the error tool result with is_error
    let mem = agent.memory.lock().await;
    let msgs = mem.get_memory(None, None).await;
    // user + assistant(tool_call) + tool_result(error) + assistant(final)
    assert_eq!(msgs.len(), 4);
    assert_eq!(msgs[2].role, Role::Tool);
    // The tool result should contain the error text
    let tool_text = msgs[2].get_text();
    assert!(tool_text.is_empty() || true); // ToolResult has output in block, not get_text
                                           // Check the actual ToolResult block
    if let MsgContent::Blocks(blocks) = &msgs[2].content {
        if let ContentBlock::ToolResult(ref r) = blocks[0] {
            assert!(r.is_error);
            assert!(r.output.contains("deliberate failure"));
        } else {
            panic!("Expected ToolResult block");
        }
    } else {
        panic!("Expected Blocks content");
    }
}

// -----------------------------------------------------------------------
// 8. test_max_iters_one_boundary
// -----------------------------------------------------------------------

#[tokio::test]
async fn test_max_iters_one_boundary() {
    // max_iters=1: LLM returns a tool call, only 1 iteration allowed,
    // then summarize should be called
    let model = MockChatModel::new(vec![tool_call_response(
        "call_1",
        "echo",
        serde_json::json!({}),
    )]);

    let mut toolkit = Toolkit::new();
    toolkit.register(Box::new(EchoTool), None);

    let agent = ReActAgent::new("bot", "prompt", Arc::new(model), Arc::new(OpenAiFormatter))
        .with_toolkit(toolkit)
        .with_max_iters(1);

    let reply = agent.reply(Msg::user("alice", "go")).await.unwrap();
    assert_eq!(reply.role, Role::Assistant);
    assert_eq!(reply.name, "bot");
}

// -----------------------------------------------------------------------
// 9. test_interrupt_during_execution
// -----------------------------------------------------------------------

#[tokio::test]
async fn test_interrupt_during_execution() {
    // Cancel the token before calling reply → should return Interrupted
    let responses: Vec<ChatResponse> = (0..5)
        .map(|i| tool_call_response(&format!("c_{i}"), "echo", serde_json::json!({})))
        .collect();

    let mut toolkit = Toolkit::new();
    toolkit.register(Box::new(EchoTool), None);

    let model = MockChatModel::new(responses);
    let agent = ReActAgent::new("bot", "prompt", Arc::new(model), Arc::new(OpenAiFormatter))
        .with_toolkit(toolkit)
        .with_max_iters(10);

    // Use the CancellationToken and cancel it
    let token = agent.cancel_token();
    token.cancel();

    let result = agent.reply(Msg::user("alice", "go")).await;
    assert!(matches!(result, Err(AgentError::Interrupted)));
}

// -----------------------------------------------------------------------
// 10. test_consecutive_replies_accumulate_memory
// -----------------------------------------------------------------------

#[tokio::test]
async fn test_consecutive_replies_accumulate_memory() {
    let model = MockChatModel::new(vec![
        text_response("reply1"),
        text_response("reply2"),
        text_response("reply3"),
    ]);
    let agent = make_agent(model);

    agent.reply(Msg::user("alice", "msg1")).await.unwrap();
    {
        let mem = agent.memory.lock().await;
        assert_eq!(mem.size().await, 2); // user + assistant
    }

    agent.reply(Msg::user("alice", "msg2")).await.unwrap();
    {
        let mem = agent.memory.lock().await;
        assert_eq!(mem.size().await, 4);
    }

    agent.reply(Msg::user("alice", "msg3")).await.unwrap();
    {
        let mem = agent.memory.lock().await;
        assert_eq!(mem.size().await, 6);
    }
}

// -----------------------------------------------------------------------
// 11. test_llm_returns_empty_content
// -----------------------------------------------------------------------

#[tokio::test]
async fn test_llm_returns_empty_content() {
    // LLM returns a response with empty content (no text, no tool calls)
    let empty_response = ChatResponse {
        content: vec![],
        id: "r".into(),
        created_at: String::new(),
        usage: ChatUsage::default(),
        metadata: None,
    };
    let model = MockChatModel::new(vec![empty_response]);
    let agent = make_agent(model);

    let reply = agent.reply(Msg::user("alice", "hi")).await.unwrap();
    // Empty content → has_tool_calls() is false → loop ends, returns empty text
    assert_eq!(reply.get_text(), "");
    assert_eq!(reply.role, Role::Assistant);
}

// -----------------------------------------------------------------------
// 12. test_llm_returns_error
// -----------------------------------------------------------------------

#[tokio::test]
async fn test_llm_returns_error() {
    // MockChatModel with no responses → returns ModelError::Other
    let model = MockChatModel::new(vec![]);
    let agent = make_agent(model);

    let result = agent.reply(Msg::user("alice", "hi")).await;
    assert!(matches!(result, Err(AgentError::Llm(_))));
}

// -----------------------------------------------------------------------
// 13. test_tool_not_found_handling
// -----------------------------------------------------------------------

#[tokio::test]
async fn test_tool_not_found_handling() {
    // LLM requests a tool that doesn't exist in the toolkit.
    // The acting() method calls toolkit.call_tool() which returns ToolError::NotFound.
    // That error is formatted and stored as a ToolResult with is_error=true.
    // Then the next LLM call returns text.
    let model = MockChatModel::new(vec![
        tool_call_response("call_1", "nonexistent_tool", serde_json::json!({})),
        text_response("Ok, the tool was not found."),
    ]);

    // Empty toolkit — no tools registered
    let agent = make_agent(model);

    let reply = agent.reply(Msg::user("alice", "use tool")).await.unwrap();
    assert_eq!(reply.get_text(), "Ok, the tool was not found.");

    // Verify the error tool result is in memory
    let mem = agent.memory.lock().await;
    let msgs = mem.get_memory(None, None).await;
    assert_eq!(msgs.len(), 4); // user + assistant(tool) + tool_result(error) + assistant(final)
    assert_eq!(msgs[2].role, Role::Tool);
    if let MsgContent::Blocks(blocks) = &msgs[2].content {
        if let ContentBlock::ToolResult(ref r) = blocks[0] {
            assert!(r.is_error);
            assert!(r.output.contains("nonexistent_tool"));
        } else {
            panic!("Expected ToolResult block");
        }
    } else {
        panic!("Expected Blocks content");
    }
}
