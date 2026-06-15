//! Macaca OS configuration types (Facade module tree).
//!
//! Provider-neutral serde surfaces for TOML + `AOS_*` environment overrides.
//! Submodules group settings by domain; this file re-exports the full public API
//! so callers continue to use `macaca_proto::config::*` unchanged.

mod autonomy;
mod builder;
mod context;
mod drivers;
mod infrastructure;
mod kernel;
mod llm;
mod mcp;
mod memory;
mod root;
mod workspace;

#[cfg(test)]
mod tests;

pub use autonomy::*;
pub use builder::*;
pub use context::*;
pub use drivers::*;
pub use infrastructure::*;
pub use kernel::*;
pub use llm::*;
pub use mcp::*;
pub use memory::*;
pub use root::*;
pub use workspace::*;
