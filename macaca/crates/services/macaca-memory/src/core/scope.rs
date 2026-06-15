//! Memory scope protocol re-exports.
//!
//! Scope identity is a cross-boundary service contract, so the canonical DTOs
//! live in `macaca-proto`. The Memory crate re-exports them for provider-local
//! code that still imports through `crate::core`.

pub use macaca_proto::{MemoryIdentity, MemoryScope, MemoryVisibility};
