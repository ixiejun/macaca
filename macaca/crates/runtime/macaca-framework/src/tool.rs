//! Tool system — registration, middleware, and execution (**Facade**).
//!
//! Provides a toolkit abstraction for managing agent tools:
//! - [`ToolHandler`] trait for implementing tools (**Strategy**)
//! - [`ToolMiddleware`] for cross-cutting concerns (**Chain of Responsibility**)
//! - [`ToolGroup`] for activating/deactivating sets of tools
//! - [`Toolkit`] as the central registry that wires everything together (**Registry**)
//!
//! # Module tree (P4 iteration 112)
//! - `response` — [`ToolResponse`] value object
//! - `error` — [`ToolError`] taxonomy
//! - `traits` — handler, middleware, and resource traits
//! - `types` — group and registered-tool value types
//! - `toolkit` — registry and `call_tool` pipeline
//! - `schema_macro` — `tool_schema!` export
//! - `tests` — contract tests for registry, middleware, and macro

mod error;
mod response;
mod schema_macro;
mod toolkit;
mod traits;
mod types;

#[cfg(test)]
mod tests;

pub use error::ToolError;
pub use response::ToolResponse;
pub use toolkit::Toolkit;
pub use traits::{ToolHandler, ToolMiddleware, ToolkitResource};
pub use types::{RegisteredTool, ToolGroup};
