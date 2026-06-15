//! Web shell composition-root bootstrap (Facade + phased Composition Root).
//!
//! `serve_web_server` was extracted from `lib.rs` into phase modules so the crate root
//! stays a thin adapter surface. Each submodule owns one bootstrap slice; [`serve`] sequences
//! them and binds axum.

mod app_state_assembly;
mod application_discovery;
mod bootstrap_ctx;
pub(crate) mod bootstrap_path_helpers;
mod config_and_kernel;
#[cfg(test)]
pub(crate) mod contract_source;
mod domain_pack_wiring;
mod post_bootstrap_hooks;
mod service_client_facades;
mod service_runtime_wiring;
mod tooling_and_persist;

mod serve;

pub(crate) use serve::serve_web_server;
