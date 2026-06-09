//! Contract tests for context assembly — system prompt and observe memory.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::agent::Agent;
use crate::formatter::OpenAiFormatter;
use crate::message::Msg;
use crate::model::{ChatModel, ChatOptions, ChatResponse, ModelError};

use super::fixtures::{text_response, MockChatModel};
use super::super::core::ReActAgent;

// -----------------------------------------------------------------------
// 14. test_system_prompt_position
// -----------------------------------------------------------------------

/// A MockChatModel that captures the messages it receives.
struct CapturingMockModel {
    responses: Arc<Mutex<Vec<ChatResponse>>>,
    call_count: Arc<AtomicUsize>,
    captured_messages: Arc<Mutex<Vec<Vec<serde_json::Value>>>>,
}

impl CapturingMockModel {
    fn new(responses: Vec<ChatResponse>) -> Self {
        Self {
            responses: Arc::new(Mutex::new(responses)),
            call_count: Arc::new(AtomicUsize::new(0)),
            captured_messages: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

#[async_trait]
impl ChatModel for CapturingMockModel {
    async fn chat(
        &self,
        messages: Vec<serde_json::Value>,
        _options: &ChatOptions,
    ) -> Result<ChatResponse, ModelError> {
        self.captured_messages.lock().await.push(messages.clone());
        let idx = self.call_count.fetch_add(1, Ordering::SeqCst);
        let responses = self.responses.lock().await;
        responses
            .get(idx)
            .cloned()
            .ok_or_else(|| ModelError::Other("no more responses".into()))
    }

    fn name(&self) -> &str {
        "capturing-mock"
    }
}

#[tokio::test]
async fn test_system_prompt_position() {
    let capturing_model = CapturingMockModel::new(vec![text_response("ok")]);
    let captured = capturing_model.captured_messages.clone();

    let agent = ReActAgent::new(
        "bot",
        "You are helpful.",
        Arc::new(capturing_model),
        Arc::new(OpenAiFormatter),
    );

    agent.reply(Msg::user("alice", "hi")).await.unwrap();

    let calls = captured.lock().await;
    assert_eq!(calls.len(), 1);
    let first_call_msgs = &calls[0];
    // The first message must be the system prompt
    assert_eq!(first_call_msgs[0]["role"], "system");
    assert_eq!(first_call_msgs[0]["content"], "You are helpful.");
}

// -----------------------------------------------------------------------
// 15. test_observe_then_reply_memory
// -----------------------------------------------------------------------

#[tokio::test]
async fn test_observe_then_reply_memory() {
    let capturing_model =
        CapturingMockModel::new(vec![text_response("I saw your observation.")]);
    let captured = capturing_model.captured_messages.clone();

    let agent = ReActAgent::new(
        "bot",
        "You are helpful.",
        Arc::new(capturing_model),
        Arc::new(OpenAiFormatter),
    );

    // Observe a message first
    let observed_msg = Msg::user("external", "context information");
    agent.observe(observed_msg).await.unwrap();

    // Now reply
    agent
        .reply(Msg::user("alice", "what do you know?"))
        .await
        .unwrap();

    let calls = captured.lock().await;
    assert_eq!(calls.len(), 1);
    let msgs_sent = &calls[0];
    // System prompt + observed msg + user msg = 3
    assert_eq!(msgs_sent.len(), 3);
    assert_eq!(msgs_sent[0]["role"], "system");
    // The observed message should appear before the user's reply message
    assert_eq!(msgs_sent[1]["content"], "context information");
    assert_eq!(msgs_sent[2]["content"], "what do you know?");
}
