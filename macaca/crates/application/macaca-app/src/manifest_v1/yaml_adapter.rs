//! YAML application adapter for Application Manifest v1 (**Adapter** module root).
//!
//! YAML applications are still first-class inputs, but new Application Platform
//! features should reason over Manifest v1 and ability descriptors.  This module
//! applies the Adapter pattern: it projects the legacy YAML model into
//! provider-neutral Manifest v1 data while preserving existing runtime
//! behavior in the legacy loader.  It also returns a Memento-style report so
//! inferred defaults and compatibility facts are auditable without logging raw
//! manifest bodies, prompt templates, secrets, environment values, or host
//! payloads.
//!
//! # Module tree (P5 iteration 97)
//! - `types` — Memento report types and adapter state
//! - `adapter` — Builder constructors for the adapter aggregate
//! - `entry` — entry-point and permission extraction helpers
//! - `abilities` — ability descriptor synthesis (inline/file/resolved/WASM)
//! - `projection` — orchestrated YAML → Manifest v1 projection with audit logs
//! - `tests` — contract tests for projection boundaries

mod abilities;
mod adapter;
mod entry;
mod projection;
mod types;

#[cfg(test)]
mod tests;

use crate::model::AppManifest;

pub use types::{
    LegacyAppManifestProjection, YamlApplicationManifestAdapter, YamlProjectionDiagnostic,
    YamlToApplicationManifestV1Report,
};

/// Convenience projection for callers that already hold a parsed legacy manifest.
///
/// Equivalent to `YamlApplicationManifestAdapter::new(manifest.clone()).project()`.
pub fn app_manifest_projection(manifest: &AppManifest) -> LegacyAppManifestProjection {
    YamlApplicationManifestAdapter::new(manifest.clone()).project()
}
