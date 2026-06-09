//! ReActAgent core — agent configuration and builder API (**Builder**).
//!
//! Holds injectable dependencies (model, formatter, toolkit, memory) and
//! runtime controls (max iterations, cancellation, compression). All fields
//! are `pub(crate)` so sibling modules (`loop`, `agent_trait`) can drive the
//! Think → Act → Observe cycle without exposing internals to crate consumers.

use std::sync::Arc;

use macaca_proto::AgentId;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use crate::formatter::Formatter;
use crate::memory::{CompressionConfig, InMemoryWorkingMemory, MemoryCompressor, WorkingMemory};
use crate::model::{ChatModel, ToolChoice};
use crate::tool::Toolkit;

/// An agent that implements the ReAct (Reasoning + Acting) loop.
///
/// On each call to `reply`:
/// 1. The user message is stored in working memory.
/// 2. The LLM is called with the full memory context and available tools.
/// 3. If the LLM returns tool calls, each tool is executed and the result
///    is stored in memory before the next iteration.
/// 4. When the LLM returns a plain text response (no tool calls), that
///    response is returned as the agent's reply.
/// 5. If `max_iters` is reached before a final text response, the last
///    assistant message from memory is returned.
pub struct ReActAgent {
    pub(crate) name: String,
    pub(crate) id: AgentId,
    pub(crate) sys_prompt: String,
    pub(crate) model: Arc<dyn ChatModel>,
    pub(crate) formatter: Arc<dyn Formatter>,
    pub(crate) toolkit: Arc<Mutex<Toolkit>>,
    pub(crate) memory: Arc<Mutex<Box<dyn WorkingMemory>>>,
    pub(crate) max_iters: usize,
    pub(crate) cancel_token: CancellationToken,
    pub(crate) compressor: Option<MemoryCompressor>,
    /// Model name passed to ChatOptions (caller-supplied, provider-neutral).
    pub(crate) model_name: Option<String>,
    /// Optional provider-neutral tool-selection policy for every reasoning turn.
    pub(crate) tool_choice: Option<ToolChoice>,
}

impl ReActAgent {
    /// Create a new `ReActAgent` with default settings (no tools, in-memory store, 10 iterations).
    pub fn new(
        name: impl Into<String>,
        sys_prompt: impl Into<String>,
        model: Arc<dyn ChatModel>,
        formatter: Arc<dyn Formatter>,
    ) -> Self {
        Self {
            name: name.into(),
            id: AgentId::new(),
            sys_prompt: sys_prompt.into(),
            model,
            formatter,
            toolkit: Arc::new(Mutex::new(Toolkit::new())),
            memory: Arc::new(Mutex::new(Box::new(InMemoryWorkingMemory::new()))),
            max_iters: 10,
            cancel_token: CancellationToken::new(),
            compressor: None,
            model_name: None,
            tool_choice: None,
        }
    }

    /// Set the model name passed to the LLM provider (caller-supplied identifier).
    pub fn with_model_name(mut self, name: impl Into<String>) -> Self {
        self.model_name = Some(name.into());
        self
    }

    /// Set a provider-neutral tool-selection strategy for reasoning calls.
    pub fn with_tool_choice(mut self, tool_choice: ToolChoice) -> Self {
        self.tool_choice = Some(tool_choice);
        self
    }

    /// Replace the default (empty) toolkit with the given one.
    pub fn with_toolkit(mut self, toolkit: Toolkit) -> Self {
        self.toolkit = Arc::new(Mutex::new(toolkit));
        self
    }

    /// Replace the default in-memory working memory with the given one.
    pub fn with_memory(mut self, memory: Box<dyn WorkingMemory>) -> Self {
        self.memory = Arc::new(Mutex::new(memory));
        self
    }

    /// Set the maximum number of reasoning-action iterations before giving up.
    pub fn with_max_iters(mut self, max_iters: usize) -> Self {
        self.max_iters = max_iters;
        self
    }

    /// Enable automatic memory compression with the given configuration.
    pub fn with_compression(mut self, config: CompressionConfig) -> Self {
        self.compressor = Some(MemoryCompressor::new(config));
        self
    }

    /// Return a clone of the cancellation token.
    pub fn cancel_token(&self) -> CancellationToken {
        self.cancel_token.clone()
    }
}
