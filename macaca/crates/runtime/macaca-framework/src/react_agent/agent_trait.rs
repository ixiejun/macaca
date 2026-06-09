//! `Agent` trait implementation — ReAct reply orchestrator (**Strategy**).

use async_trait::async_trait;

use crate::agent::{Agent, AgentError, AgentResult};
use crate::message::Msg;
use macaca_proto::AgentId;

use super::core::ReActAgent;

#[async_trait]
impl Agent for ReActAgent {
    async fn reply(&self, msg: Msg) -> AgentResult<Msg> {
        tracing::debug!(
            agent = %self.name,
            max_iters = self.max_iters,
            "react_agent.reply begin ReAct loop"
        );

        // Store the incoming user message
        {
            let mut mem = self.memory.lock().await;
            mem.add(msg, vec![]).await;
        }

        // Compress memory if configured and needed
        if let Some(ref compressor) = self.compressor {
            let mut mem = self.memory.lock().await;
            let _ = compressor
                .compress_if_needed(
                    mem.as_mut(),
                    self.model.as_ref(),
                    self.formatter.as_ref(),
                    &self.sys_prompt,
                )
                .await;
            // Ignore compression errors — don't fail the reply
        }

        for _iteration in 0..self.max_iters {
            // Check for cancellation before each reasoning step
            if self.cancel_token.is_cancelled() {
                tracing::warn!(agent = %self.name, iteration = _iteration, "react_agent.reply interrupted before reasoning");
                return Err(AgentError::Interrupted);
            }

            // Reasoning: call the LLM
            let response = self.reasoning(_iteration).await?;

            // If no tool calls → this is the final response
            if !response.has_tool_calls() {
                let text = response.get_text();
                let reply = Msg::assistant(&self.name, text);
                tracing::debug!(
                    agent = %self.name,
                    iteration = _iteration,
                    "react_agent.reply complete with final text response"
                );
                // The assistant message was already stored by `reasoning`; no
                // need to store it again. Return the reply directly.
                return Ok(reply);
            }

            // Acting: execute every requested tool call
            for tool_call in response.get_tool_calls() {
                // Check cancellation between tool calls as well
                if self.cancel_token.is_cancelled() {
                    tracing::warn!(
                        agent = %self.name,
                        tool_name = %tool_call.name,
                        "react_agent.reply interrupted between tool calls"
                    );
                    return Err(AgentError::Interrupted);
                }
                self.acting(tool_call).await?;
            }
        }

        // Exceeded max iterations — return whatever the last assistant said
        self.summarize().await
    }

    async fn observe(&self, msg: Msg) -> AgentResult<()> {
        let mut mem = self.memory.lock().await;
        mem.add(msg, vec!["observed".to_string()]).await;
        Ok(())
    }

    async fn interrupt(&self, _msg: Msg) -> AgentResult<()> {
        tracing::warn!(agent = %self.name, "react_agent.interrupt: cancellation requested");
        self.cancel_token.cancel();
        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn id(&self) -> &AgentId {
        &self.id
    }
}
