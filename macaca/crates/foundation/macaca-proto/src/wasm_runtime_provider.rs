//! Provider-neutral WASM Application Runtime Provider contracts.
//!
//! This module is intentionally data-only.  It defines the vocabulary that the
//! Application Framework, SDK, runtime host, and future runtime providers use
//! to describe WASM execution support without importing a concrete engine.  The
//! DTOs apply Bridge and Strategy boundaries at the protocol layer: callers can
//! ask for descriptors, capabilities, profiles, sessions, and diagnostics while
//! a later runtime-host provider decides how to satisfy them.  No type in this
//! file stores raw WASM bytes, raw command payloads, manifests, secrets,
//! environment values, API keys, engine handles, or provider-specific objects.

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::{ApplicationAbiError, ApplicationAbiVersion, PackageRuntimeKind, TraceContext};

mod compile_cache;
mod runtime_errors;
#[cfg(test)]
mod tests;

pub use compile_cache::*;
pub use runtime_errors::*;

/// String-backed reference to a WASM artifact known to package/runtime layers.
///
/// The reference is a handle, not bytes.  It lets a package manager or future
/// provider locate an artifact through approved storage while keeping raw WASM
/// bytes out of logs, diagnostics, DTO snapshots, and SDK-visible structures.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct WasmRuntimeArtifactRef(String);

impl WasmRuntimeArtifactRef {
    /// Create a normalized artifact reference after trimming whitespace.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into().trim().to_string())
    }

    /// Return the provider-neutral artifact reference string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Report whether the reference is empty after normalization.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Display for WasmRuntimeArtifactRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Capability declaration for a WASM runtime engine family.
///
/// The fields describe behavior rather than a concrete engine.  Future
/// providers can advertise compile, instantiate, execute, component model, and
/// host import bridge support without leaking Wasmtime, WasmEdge, Wasmer, or
/// any other concrete implementation type into public contracts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WasmEngineCapabilities {
    pub can_compile: bool,
    pub can_instantiate: bool,
    pub can_execute: bool,
    pub supports_component_model: bool,
    pub supports_host_import_bridge: bool,
    pub supports_wasi: bool,
    pub engine_features: Vec<String>,
    pub metadata: BTreeMap<String, String>,
}

impl WasmEngineCapabilities {
    /// Build the capability set used by an unavailable Null Object provider.
    pub fn unavailable() -> Self {
        Self {
            can_compile: false,
            can_instantiate: false,
            can_execute: false,
            supports_component_model: false,
            supports_host_import_bridge: false,
            supports_wasi: false,
            engine_features: Vec::new(),
            metadata: BTreeMap::new(),
        }
    }
}

/// Bounded resource envelope requested for a runtime session.
///
/// These limits are advisory in this contract slice because no real provider
/// executes code yet.  Keeping them in the profile now gives future providers a
/// stable place to enforce memory, fuel, wall-clock, and host-call limits
/// without changing the public session request shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WasmRuntimeResourceEnvelope {
    pub max_memory_bytes: Option<u64>,
    pub max_fuel: Option<u64>,
    pub max_wall_time_ms: Option<u64>,
    pub max_host_calls: Option<u64>,
    pub metadata: BTreeMap<String, String>,
}

impl Default for WasmRuntimeResourceEnvelope {
    fn default() -> Self {
        Self {
            max_memory_bytes: Some(64 * 1024 * 1024),
            max_fuel: None,
            max_wall_time_ms: Some(30_000),
            max_host_calls: Some(1_000),
            metadata: BTreeMap::new(),
        }
    }
}

/// Provider-neutral compile strategy requested by the execution profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WasmCompileMode {
    Deferred,
    AheadOfTime,
    JustInTime,
}

/// Provider-neutral instantiation strategy requested by the execution profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WasmInstantiationMode {
    PerSession,
    Pooled,
    ReusableSnapshot,
}

/// Execution profile selected before a runtime provider creates a session.
///
/// The profile is the Strategy input for future provider selection.  It is
/// intentionally generic: it names runtime kind, ABI version, compile mode,
/// instantiation mode, diagnostics detail, and resources, but never a concrete
/// provider or engine implementation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WasmExecutionProfile {
    pub runtime_kind: PackageRuntimeKind,
    pub abi_version: ApplicationAbiVersion,
    pub compile_mode: WasmCompileMode,
    pub instantiation_mode: WasmInstantiationMode,
    pub diagnostics_level: String,
    pub resources: WasmRuntimeResourceEnvelope,
    pub metadata: BTreeMap<String, String>,
}

impl WasmExecutionProfile {
    /// Return the default profile for current WASM Component Application ABI.
    pub fn default_wasm_component() -> Self {
        Self {
            runtime_kind: PackageRuntimeKind::WasmComponent,
            abi_version: ApplicationAbiVersion::v0(),
            compile_mode: WasmCompileMode::Deferred,
            instantiation_mode: WasmInstantiationMode::PerSession,
            diagnostics_level: "sanitized".into(),
            resources: WasmRuntimeResourceEnvelope::default(),
            metadata: BTreeMap::new(),
        }
    }
}

/// Structured reason explaining why a WASM runtime provider is unavailable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WasmRuntimeUnavailableReason {
    pub code: String,
    pub message: String,
    pub metadata: BTreeMap<String, String>,
}

impl WasmRuntimeUnavailableReason {
    /// Build a provider-missing reason used by the default unavailable provider.
    pub fn provider_missing(message: impl Into<String>) -> Self {
        Self {
            code: "provider_missing".into(),
            message: sanitize_diagnostic_text(message),
            metadata: BTreeMap::new(),
        }
    }
}

/// Availability state returned by providers and provider registries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WasmRuntimeAvailability {
    pub state: String,
    pub reason: Option<WasmRuntimeUnavailableReason>,
    pub capabilities: WasmEngineCapabilities,
    pub diagnostics: Option<WasmRuntimeDiagnostics>,
    pub metadata: BTreeMap<String, String>,
}

impl WasmRuntimeAvailability {
    /// Build an unavailable state without relying on panics or provider errors.
    pub fn unavailable(
        runtime_kind: PackageRuntimeKind,
        reason: WasmRuntimeUnavailableReason,
        trace: Option<TraceContext>,
    ) -> Self {
        Self {
            state: "unavailable".into(),
            diagnostics: Some(WasmRuntimeDiagnostics::new(
                runtime_kind,
                reason.code.clone(),
                reason.message.clone(),
                trace,
            )),
            reason: Some(reason),
            capabilities: WasmEngineCapabilities::unavailable(),
            metadata: BTreeMap::new(),
        }
    }

    /// Report whether the provider can execute guest code.
    pub fn is_available(&self) -> bool {
        self.state == "available" && self.capabilities.can_execute
    }
}

/// Sanitized diagnostic emitted by availability and session lifecycle paths.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WasmRuntimeDiagnostics {
    pub runtime_kind: PackageRuntimeKind,
    pub reason_code: String,
    pub message: String,
    pub trace_id: Option<String>,
    pub metadata: BTreeMap<String, String>,
}

impl WasmRuntimeDiagnostics {
    /// Build diagnostics while applying conservative text redaction.
    pub fn new(
        runtime_kind: PackageRuntimeKind,
        reason_code: impl Into<String>,
        message: impl Into<String>,
        trace: Option<TraceContext>,
    ) -> Self {
        Self {
            runtime_kind,
            reason_code: reason_code.into(),
            message: sanitize_diagnostic_text(message),
            trace_id: trace.map(|value| value.trace_id),
            metadata: BTreeMap::new(),
        }
    }

    /// Return whether this diagnostic avoids known forbidden raw material.
    pub fn is_sanitized(&self) -> bool {
        let lower = self.message.to_lowercase();
        !FORBIDDEN_DIAGNOSTIC_MARKERS
            .iter()
            .any(|marker| lower.contains(marker))
    }
}

/// Descriptor exported by a runtime provider registry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WasmRuntimeProviderDescriptor {
    pub runtime_kind: PackageRuntimeKind,
    pub provider_class: String,
    pub capabilities: WasmEngineCapabilities,
    pub default_profile: WasmExecutionProfile,
    pub availability: WasmRuntimeAvailability,
    pub diagnostics: Option<WasmRuntimeDiagnostics>,
    pub metadata: BTreeMap<String, String>,
}

impl WasmRuntimeProviderDescriptor {
    /// Return the default descriptor used when no runtime provider is installed.
    pub fn unavailable_default() -> Self {
        let reason = WasmRuntimeUnavailableReason::provider_missing(
            "WASM runtime provider is not installed",
        );
        let availability =
            WasmRuntimeAvailability::unavailable(PackageRuntimeKind::WasmComponent, reason, None);
        Self {
            runtime_kind: PackageRuntimeKind::WasmComponent,
            provider_class: "unavailable".into(),
            capabilities: WasmEngineCapabilities::unavailable(),
            default_profile: WasmExecutionProfile::default_wasm_component(),
            diagnostics: availability.diagnostics.clone(),
            availability,
            metadata: BTreeMap::new(),
        }
    }
}

/// Session creation request validated before any provider may run.
///
/// This is a Specification object as well as a DTO.  `validate` centralizes
/// invariants shared by unavailable and future real providers: trace,
/// application id, ability id, artifact reference, and runtime profile must all
/// be present before execution can proceed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WasmRuntimeSessionRequest {
    pub trace: Option<TraceContext>,
    pub application_id: String,
    pub ability_id: String,
    pub artifact: WasmRuntimeArtifactRef,
    pub profile: WasmExecutionProfile,
    pub metadata: BTreeMap<String, String>,
}

impl WasmRuntimeSessionRequest {
    /// Build a session request and validate all required execution invariants.
    pub fn new(
        trace: TraceContext,
        application_id: impl Into<String>,
        ability_id: impl Into<String>,
        artifact: WasmRuntimeArtifactRef,
        profile: WasmExecutionProfile,
    ) -> Result<Self, ApplicationAbiError> {
        let request = Self {
            trace: Some(trace),
            application_id: application_id.into().trim().to_string(),
            ability_id: ability_id.into().trim().to_string(),
            artifact,
            profile,
            metadata: BTreeMap::new(),
        };
        request.validate()?;
        Ok(request)
    }

    /// Validate the request before provider/session creation.
    pub fn validate(&self) -> Result<(), ApplicationAbiError> {
        let Some(trace) = &self.trace else {
            return Err(ApplicationAbiError::MissingTraceContext);
        };
        if trace.trace_id.trim().is_empty() {
            return Err(ApplicationAbiError::MissingTraceContext);
        }
        if self.application_id.trim().is_empty() {
            return Err(ApplicationAbiError::InvalidDeclaration(
                "wasm runtime session requires application id".into(),
            ));
        }
        if self.ability_id.trim().is_empty() {
            return Err(ApplicationAbiError::InvalidDeclaration(
                "wasm runtime session requires ability id".into(),
            ));
        }
        if self.artifact.is_empty() {
            return Err(ApplicationAbiError::InvalidDeclaration(
                "wasm runtime session requires artifact reference".into(),
            ));
        }
        Ok(())
    }

    /// Return the runtime kind requested by the execution profile.
    pub fn runtime_kind(&self) -> PackageRuntimeKind {
        self.profile.runtime_kind.clone()
    }
}

const FORBIDDEN_DIAGNOSTIC_MARKERS: &[&str] = &[
    "raw wasm",
    "raw payload",
    "raw manifest",
    "secret",
    "api_key",
    "apikey",
    "private key",
    "environment",
    "env var",
    "prompt",
];

pub(crate) fn sanitize_diagnostic_text(value: impl Into<String>) -> String {
    let mut sanitized = value.into();
    for marker in FORBIDDEN_DIAGNOSTIC_MARKERS {
        sanitized = sanitized.replace(marker, "[redacted]");
        sanitized = sanitized.replace(&marker.to_uppercase(), "[redacted]");
    }
    sanitized
}
