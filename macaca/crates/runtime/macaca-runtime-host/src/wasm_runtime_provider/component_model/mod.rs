//! Component Model WASM runtime provider — Facade module tree.
//!
//! The provider implements the same [`WasmApplicationRuntimeProvider`] Strategy
//! used by the default and unavailable providers.  It keeps Component Model
//! validation and invocation behind a private Adapter so public Macaca layers
//! remain provider-neutral and Route C dependency boundaries stay intact.
//!
//! **Pattern:** Facade — this module re-exports the public provider type while
//! delegating implementation to focused submodules:
//! - `provider.rs` — provider Strategy and session factory
//! - `session.rs` — execution session core dispatch and lifecycle surface
//! - `session_host.rs` — declared host-command orchestration and bridge dispatch
//! - `session_lifecycle.rs` — checkpoint / restore / upgrade / rollback mementos
//! - `template_engine.rs` — placeholder resolution for declarative host commands
//! - `support.rs` — artifact loading, digest, and dispatch classification

mod provider;
mod session;
mod session_host;
mod session_lifecycle;
mod support;
mod template_engine;

pub use provider::ComponentModelWasmRuntimeProvider;
