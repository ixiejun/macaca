//! Deterministic `LlmProvider` that returns a fixed sequence of responses.
//! Use with `AgenticLoop` to exercise LLM → tool → LLM without network.

use std::collections::VecDeque;
use std::sync::Mutex;

use async_trait::async_trait;
use macaca_llm::LlmProvider;
use macaca_proto::error::{MacacaError, MacacaResult};
use macaca_proto::types::{LlmMessage, LlmOptions, LlmResponse};

/// Queue-backed LLM: each `chat` pops the next scripted response.
pub struct ScriptedLlm {
    name: String,
    queue: Mutex<VecDeque<LlmResponse>>,
}

impl ScriptedLlm {
    pub fn new(name: impl Into<String>, responses: Vec<LlmResponse>) -> Self {
        Self {
            name: name.into(),
            queue: Mutex::new(responses.into()),
        }
    }

    /// Panics if any scripted turn was not consumed (helps catch under-driving tests).
    pub fn assert_drained(&self) {
        let q = self.queue.lock().expect("scripted llm mutex");
        assert!(
            q.is_empty(),
            "scripted LLM had {} unconsumed response(s)",
            q.len()
        );
    }

    pub fn remaining(&self) -> usize {
        self.queue.lock().map(|q| q.len()).unwrap_or(0)
    }
}

#[async_trait]
impl LlmProvider for ScriptedLlm {
    fn name(&self) -> &str {
        &self.name
    }

    async fn chat(
        &self,
        _messages: Vec<LlmMessage>,
        _options: &LlmOptions,
    ) -> MacacaResult<LlmResponse> {
        let mut q = self.queue.lock().map_err(|e| MacacaError::Llm(e.to_string()))?;
        q.pop_front()
            .ok_or_else(|| MacacaError::Llm("scripted LLM queue exhausted".into()))
    }
}
