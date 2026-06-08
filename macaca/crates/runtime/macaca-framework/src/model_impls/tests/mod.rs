//! Contract tests for the model_impls module tree.

#[cfg(feature = "chat_completions_api")]
mod openai_tests;

#[cfg(feature = "messages_api")]
mod anthropic_tests;

mod helper_tests;
