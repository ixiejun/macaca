//! Generic routes for application-owned UI bundles (Facade module).
//!
//! These handlers implement the host side of the Application-Owned UI Runtime
//! contract. They are intentionally application-agnostic: static files are
//! served only from manifest-declared package paths, and bridge calls are
//! admitted only through manifest-declared capabilities before being routed to
//! the existing Application Service host-dispatch boundary.
//!
//! # Design patterns
//!
//! - **Facade** — `mod.rs` re-exports handlers so `bootstrap.rs` keeps stable paths.
//! - **Adapter** — [`adapter`] normalizes paths and maps bridge envelopes to host commands.
//! - **Strategy** — capability-specific dispatch in [`bridge_handlers`] (`service.call` today).
//!
//! `bootstrap.rs` registers `get_app_ui_asset` and `post_app_ui_bridge`.

mod adapter;
mod asset_handlers;
mod bridge_adapter;
mod bridge_handlers;
mod context;
mod types;

#[cfg(test)]
pub(crate) mod contract_source;
#[cfg(test)]
mod tests;

pub use asset_handlers::get_app_ui_asset;
pub use bridge_handlers::post_app_ui_bridge;
