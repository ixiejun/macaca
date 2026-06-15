//! LLM service protocol contract re-export.
//!
//! The canonical LLM service DTOs live in `macaca-proto` so facade clients can
//! build typed commands without depending on this provider crate. The LLM
//! service crate re-exports them for provider-internal ergonomics.

pub use macaca_proto::llm_service::*;
