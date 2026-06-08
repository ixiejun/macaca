//! Sanitized Application Service projections (**Projection** module tree).
//!
//! Raw YAML manifests are adapted into Manifest v1, then projected into bounded
//! service DTOs for Web, CLI, Gateway, and framework adapters.  Projections expose
//! only identifiers, names, counts, declared capability names, policy presence flags,
//! and digests — never prompt bodies, secrets, or unbounded user input.
//!
//! # Module tree (P5 iteration 99)
//! - `projection` — public entry points (Facade API)
//! - `app_view` — service app summary projection
//! - `ui` — application-owned UI metadata projection
//! - `policy` — tool/context/skill/workbench policy sub-views
//! - `heartbeat` — manifest heartbeat agent declarations
//! - `support` — shared helpers (digest, diagnostics, agent naming)
//! - `tests` — sanitization contract tests

mod app_view;
mod heartbeat;
mod policy;
mod projection;
mod support;
mod ui;

#[cfg(test)]
mod tests;

pub use heartbeat::{
    app_manifest_to_heartbeat_agent_views, heartbeat_native_profile_id, heartbeat_wake_scope_key,
};
pub use projection::{
    app_manifest_to_metadata_view, app_manifest_to_metadata_view_with_catalog,
    app_manifest_to_service_app_view, app_manifest_to_service_app_view_with_catalog,
};
