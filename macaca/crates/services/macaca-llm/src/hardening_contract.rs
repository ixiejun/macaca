//! Hardened LLM service protocol contract re-export.
//!
//! The canonical hardening DTOs live under `macaca_proto::llm_service`, keeping
//! SDK and shell callers decoupled from concrete LLM provider implementations.

pub use macaca_proto::llm_service::hardening::*;
