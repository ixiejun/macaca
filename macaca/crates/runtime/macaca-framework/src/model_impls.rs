//! Legacy HTTP [`ChatModel`] adapters (**Facade** module tree).
//!
//! Concrete implementations for OpenAI-compatible chat-completions and Messages API
//! HTTP shapes. Vendor routing identifiers are injected at construction time via
//! compile-time concatenation so this layer does not own parallel routing tables.
//!
//! # Module tree (P4 iteration 105)
//! - `helpers` — option merge + tool-choice encoding (shared Adapter utilities)
//! - `openai` — [`OpenAiChatModel`] (chat-completions Adapter + Formatter Strategy)
//! - `anthropic` — [`AnthropicChatModel`] (messages API Adapter + Formatter Strategy)
//! - `tests` — request-body and response-parsing contract tests

mod helpers;

#[cfg(feature = "chat_completions_api")]
mod openai;

#[cfg(feature = "messages_api")]
mod anthropic;

#[cfg(test)]
mod tests;

#[cfg(feature = "chat_completions_api")]
pub use openai::OpenAiChatModel;

#[cfg(feature = "messages_api")]
pub use anthropic::AnthropicChatModel;
