//! Memory status protocol re-exports.
//!
//! Provider implementations report status through proto-owned DTOs so SDK and
//! runtime-host clients can deserialize snapshots without depending on this
//! concrete service crate.

pub use macaca_proto::{MemoryCapabilitySet, MemoryStatusReport};
