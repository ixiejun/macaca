use async_trait::async_trait;
use macaca_proto::{
    error::MacacaResult,
    types::{LlmMessage, LlmOptions, LlmResponse},
};

/// Core abstraction for any LLM backend.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Returns the provider's identifier (e.g. "openai", "anthropic").
    fn name(&self) -> &str;

    /// Sends a chat request and returns a complete response.
    async fn chat(
        &self,
        messages: Vec<LlmMessage>,
        options: &LlmOptions,
    ) -> MacacaResult<LlmResponse>;
}
