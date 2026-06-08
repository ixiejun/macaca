//! Message formatters — convert framework [`Msg`] to provider-specific API formats (**Facade**).
//!
//! Each formatter implements two operations:
//! - `format`: converts a slice of [`Msg`] into the JSON array the provider API expects
//! - `parse_response`: converts the raw provider response JSON into a unified [`ChatResponse`]
//!
//! # Module tree (P4 iteration 107)
//! - `error` — [`FormatterError`] taxonomy
//! - `core` — [`Formatter`] Strategy trait
//! - `openai_common` — shared OpenAI-compatible encode/decode helpers
//! - `openai` — [`OpenAiFormatter`] Chat Completions Adapter
//! - `dashscope` — [`DashScopeFormatter`] OpenAI-compatible + native response Adapter
//! - `anthropic` — [`AnthropicFormatter`] Messages API Adapter
//! - `tests` — wire-format contract tests per provider

mod anthropic;
mod dashscope;
mod error;
mod openai;
mod openai_common;
mod core;

#[cfg(test)]
mod tests;

pub use anthropic::AnthropicFormatter;
pub use dashscope::DashScopeFormatter;
pub use error::FormatterError;
pub use core::Formatter;
pub use openai::OpenAiFormatter;
