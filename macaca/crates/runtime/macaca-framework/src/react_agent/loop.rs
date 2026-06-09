//! ReAct loop steps — reasoning, acting, and summarize (**Template Method**).
//!
//! Each method is one phase of the Think → Act → Observe cycle. The `Agent::reply`
//! orchestrator in `agent_trait` calls these in a loop until a final text response
//! or `max_iters` is reached.

use macaca_context::{
    ContextAssembleInput, ContextEngineSelection, ContextFacade, ContextFacadeAssemblyPolicy,
};

use crate::agent::{AgentError, AgentResult};
use crate::llm_wire::{
    chat_options_to_llm_options, llm_messages_to_json_values, messages_from_json_values,
};
use crate::message::{ContentBlock, Msg, Role};
use crate::model::{ChatOptions, ChatResponse};

use super::core::ReActAgent;
use super::helpers::response_to_content;

impl ReActAgent {
    /// Encodes working memory + system prompt as provider JSON, then runs
    /// `macaca_context::ContextFacade` (Composer → Engine) before calling the underlying
    /// `ChatModel`, sharing the same OS boundary as `macaca-runtime` / `macaca-web`.
    pub(crate) async fn reasoning(&self, iteration: usize) -> AgentResult<ChatResponse> {
        tracing::debug!(
            agent = %self.name,
            iteration,
            max_iters = self.max_iters,
            "react_agent.reasoning begin Think step"
        );

        let memory_msgs = {
            let mem = self.memory.lock().await;
            mem.get_with_summary().await
        };

        // Build message list: system prompt + memory contents
        let mut msgs = vec![Msg::system(self.sys_prompt.as_str())];
        msgs.extend(memory_msgs);

        // Format for the provider
        let formatted = self.formatter.format(&msgs);

        // Collect tool definitions
        let tool_defs = {
            let toolkit = self.toolkit.lock().await;
            toolkit.get_definitions()
        };
        let has_tools = !tool_defs.is_empty();

        let options = ChatOptions {
            model: self.model_name.clone(),
            tools: if has_tools { Some(tool_defs) } else { None },
            tool_choice: if has_tools {
                self.tool_choice.clone()
            } else {
                None
            },
            ..Default::default()
        };

        tracing::debug!(
            agent = %self.name,
            tool_count = options.tools.as_ref().map(|tools| tools.len()).unwrap_or_default(),
            tool_choice = ?options.tool_choice,
            "react_agent.reasoning prepared provider-neutral tool policy"
        );

        let llm_opts = chat_options_to_llm_options(&options);
        let base_messages = messages_from_json_values(&formatted);
        let facade_input = ContextAssembleInput::legacy(
            self.name.clone(),
            llm_opts.model.clone(),
            base_messages,
            llm_opts,
        );
        let assembled = ContextFacade::builtins(ContextEngineSelection::legacy())
            .assemble_model_context(facade_input, &[], ContextFacadeAssemblyPolicy::default())
            .await
            .map_err(|e| AgentError::Llm(e.to_string()))?;
        let chat_payload = llm_messages_to_json_values(&assembled.messages);

        // Call the LLM with OS-assembled messages (legacy equivalent when provider list is empty).
        let response = self
            .model
            .chat(chat_payload, &options)
            .await
            .map_err(|e| AgentError::Llm(e.to_string()))?;

        // Persist the assistant's response into memory
        let assistant_content = response_to_content(&response);
        let assistant_msg = Msg::assistant(&self.name, assistant_content);
        {
            let mut mem = self.memory.lock().await;
            mem.add(assistant_msg, vec![]).await;
        }

        tracing::debug!(
            agent = %self.name,
            iteration,
            has_tool_calls = response.has_tool_calls(),
            "react_agent.reasoning complete"
        );

        Ok(response)
    }

    /// Acting step: execute a single tool call and store the result in memory.
    pub(crate) async fn acting(&self, tool_call: &crate::message::ToolUseBlock) -> AgentResult<()> {
        tracing::debug!(
            agent = %self.name,
            tool_id = %tool_call.id,
            tool_name = %tool_call.name,
            "react_agent.acting begin tool execution"
        );
        let result = {
            let toolkit = self.toolkit.lock().await;
            toolkit
                .call_tool(&tool_call.name, tool_call.input.clone())
                .await
        };

        let (output, is_error) = match result {
            Ok(response) => {
                let text = response
                    .content
                    .iter()
                    .filter_map(|b| match b {
                        ContentBlock::Text(t) => Some(t.text.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("");
                (text, false)
            }
            Err(e) => (format!("Error: {e}"), true),
        };

        let result_msg = Msg::tool_result(&tool_call.id, &tool_call.name, output, is_error);
        {
            let mut mem = self.memory.lock().await;
            mem.add(result_msg, vec![]).await;
        }

        tracing::debug!(
            agent = %self.name,
            tool_name = %tool_call.name,
            is_error,
            "react_agent.acting complete; tool result stored in memory"
        );

        Ok(())
    }

    /// Summarize: return the last assistant message from memory (used when
    /// `max_iters` is exhausted without a tool-free response).
    pub(crate) async fn summarize(&self) -> AgentResult<Msg> {
        tracing::warn!(
            agent = %self.name,
            max_iters = self.max_iters,
            "react_agent.summarize: max iterations reached without final text reply"
        );
        let mem = self.memory.lock().await;
        let msgs = mem.get_memory(None, None).await;
        let last_content = msgs
            .iter()
            .rev()
            .find(|m| m.role == Role::Assistant)
            .map(|m| m.get_text())
            .unwrap_or_else(|| "Max iterations reached.".to_string());
        Ok(Msg::assistant(&self.name, last_content))
    }

}
