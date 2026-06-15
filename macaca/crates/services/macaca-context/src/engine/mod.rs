//! Context engine contracts and built-in strategy implementations.
//!
//! Split from monolithic `engine.rs` (P5 iteration 91) using a **Facade** module tree.
//!
//! # Design patterns
//! - **Facade**: `mod.rs` re-exports preserve `macaca_context::engine::*` public API.
//! - **Strategy**: [`ContextEngine`] trait with passthrough/windowed/pruning/summary implementations.
//! - **Registry**: [`ContextEngineRegistry`] resolves engines by id with passthrough default.
//! - **Chain of Responsibility**: [`ContextRuntimeFacade`] primary→fallback assembly path.
//! - **Decorator**: [`SummaryContextEngine`] wraps pruning before compaction.
//! - **Adapter**: [`PruningContextEngine`] delegates rendering to [`ContextRenderable`].

mod facade;
mod helpers;
mod passthrough;
mod pruning;
mod registry;
mod summary;
mod types;
mod windowed;

#[cfg(test)]
mod tests;

pub use facade::{list_builtin_engine_infos, ContextManagerFacade, ContextRuntimeFacade};
pub use passthrough::PassthroughContextEngine;
pub use pruning::PruningContextEngine;
pub use registry::ContextEngineRegistry;
pub use summary::SummaryContextEngine;
pub use types::{
    ContextAfterTurnInput, ContextAssembleInput, ContextAssembleResult, ContextEngine,
    ContextEngineInfo, ContextEngineSelection, ContextOptionsPatch,
};
pub use windowed::WindowedContextEngine;
