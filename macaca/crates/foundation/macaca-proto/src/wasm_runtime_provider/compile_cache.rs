//! Provider-neutral compiled WASM artifact cache DTOs.
//!
//! The runtime host owns actual compiled modules.  These DTOs only describe
//! deterministic cache identity and cache lookup outcomes, making cache
//! decisions traceable without exposing raw WASM bytes or engine handles.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::sanitize_diagnostic_text;

/// Deterministic cache key for compiled WASM artifacts.
///
/// The key is a provider-neutral Memento for compile-cache lookup.  It combines
/// the artifact digest, ABI version, provider capability fingerprint, and
/// execution-profile fingerprint so cached modules are never reused across
/// incompatible ABI or runtime capability boundaries.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct WasmCompiledArtifactCacheKey {
    pub digest_algorithm: String,
    pub digest_value: String,
    pub abi_version: String,
    pub capability_fingerprint: String,
    pub profile_fingerprint: String,
}

impl WasmCompiledArtifactCacheKey {
    /// Build a normalized cache key from admission/runtime metadata.
    pub fn new(
        digest_algorithm: impl Into<String>,
        digest_value: impl Into<String>,
        abi_version: impl Into<String>,
        metadata: &BTreeMap<String, String>,
    ) -> Self {
        Self {
            digest_algorithm: digest_algorithm.into().trim().to_ascii_lowercase(),
            digest_value: digest_value.into().trim().to_string(),
            abi_version: abi_version.into().trim().to_string(),
            capability_fingerprint: metadata
                .get("capability_fingerprint")
                .map(|value| value.trim().to_string())
                .unwrap_or_else(|| "capability-unspecified".into()),
            profile_fingerprint: metadata
                .get("profile_fingerprint")
                .map(|value| value.trim().to_string())
                .unwrap_or_else(|| "profile-unspecified".into()),
        }
    }
}

/// Provider-neutral compile-cache lookup state.
///
/// Runtime providers use this enum to explain whether a session reused a
/// compiled artifact, compiled a fresh module, or bypassed the cache for a
/// documented reason.  The status is deliberately independent of cache backend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WasmCompiledArtifactCacheState {
    Hit,
    Miss,
    Bypass,
    Stored,
}

/// Auditable report describing one compiled-artifact cache decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WasmCompiledArtifactCacheReport {
    pub key: WasmCompiledArtifactCacheKey,
    pub state: WasmCompiledArtifactCacheState,
    pub reason: String,
    pub metadata: BTreeMap<String, String>,
}

impl WasmCompiledArtifactCacheReport {
    /// Build a cache report with normalized, sanitized reason text.
    pub fn new(
        key: WasmCompiledArtifactCacheKey,
        state: WasmCompiledArtifactCacheState,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            key,
            state,
            reason: sanitize_diagnostic_text(reason),
            metadata: BTreeMap::new(),
        }
    }
}
