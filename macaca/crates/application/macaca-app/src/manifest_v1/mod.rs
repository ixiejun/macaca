//! Application Manifest v1 adapters owned by the Application Framework.
//!
//! This module is intentionally small and provider-neutral.  It converts
//! Application Framework concepts into protocol DTOs without constructing a
//! kernel, service runtime, provider, Web state, or runtime-host component.

pub mod yaml_adapter;

pub use yaml_adapter::{
    YamlApplicationManifestAdapter, YamlApplicationManifestProjection, YamlProjectionDiagnostic,
    YamlToApplicationManifestV1Report,
};
