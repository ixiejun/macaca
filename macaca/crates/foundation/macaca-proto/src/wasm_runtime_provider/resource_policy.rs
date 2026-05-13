//! Provider-neutral WASM sandbox and resource governance DTOs.
//!
//! The types in this module are deliberately data-only.  They form the shared
//! vocabulary used by admission, runtime hosts, SDK fixtures, and future engine
//! adapters to describe resource boundaries without exposing concrete engine
//! handles or host resources.  The module applies Strategy through policy DTOs,
//! Specification through deterministic validation and merge helpers, and
//! Observer through sanitized audit reports.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{PackageRuntimeKind, TraceContext};

use super::{sanitize_diagnostic_text, FORBIDDEN_DIAGNOSTIC_MARKERS};

/// Maximum payload accepted by the default WASM runtime guard.
///
/// The value is intentionally conservative because command payloads are JSON
/// envelopes, not bulk data transport.  Applications that need large data must
/// pass stable references through approved services instead of embedding raw
/// bytes in a runtime command.
pub const DEFAULT_WASM_MAX_PAYLOAD_BYTES: u64 = 64 * 1024;

/// Maximum number of concurrently active sessions per quota key.
///
/// The default keeps the in-process provider deterministic and auditable while
/// still permitting a small amount of parallelism for independent abilities.
pub const DEFAULT_WASM_MAX_CONCURRENT_SESSIONS: u64 = 4;

/// Stable quota scope used by resource guards and audit reports.
///
/// The key intentionally contains only application and ability identifiers.  It
/// does not contain workflow names, driver names, tenant secrets, paths, env
/// values, or raw payload fragments, so it can safely appear in logs.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct WasmResourceQuotaKey {
    pub application_id: String,
    pub ability_id: String,
    pub scope: String,
}

impl WasmResourceQuotaKey {
    /// Build a quota key after trimming all caller-provided identifiers.
    pub fn new(
        application_id: impl Into<String>,
        ability_id: impl Into<String>,
        scope: impl Into<String>,
    ) -> Self {
        Self {
            application_id: application_id.into().trim().to_string(),
            ability_id: ability_id.into().trim().to_string(),
            scope: scope.into().trim().to_string(),
        }
    }

    /// Return a stable text label for metadata and audit logs.
    pub fn stable_label(&self) -> String {
        format!("{}:{}:{}", self.application_id, self.ability_id, self.scope)
    }
}

/// Provider-neutral WASM resource policy.
///
/// Each optional numeric field is interpreted as a maximum.  `None` means the
/// current layer does not express an opinion for that limit; merge rules keep
/// the stricter concrete value when several layers define the same limit.  The
/// runtime host may enforce only the limits it can prove with its current
/// engine adapter, but keeping the full envelope here lets future providers
/// implement memory/table/fuel/epoch guards without changing public contracts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WasmResourcePolicy {
    pub max_memory_bytes: Option<u64>,
    pub max_table_elements: Option<u64>,
    pub max_fuel: Option<u64>,
    pub max_epoch_ticks: Option<u64>,
    pub max_wall_time_ms: Option<u64>,
    pub max_host_call_time_ms: Option<u64>,
    pub max_host_calls: Option<u64>,
    pub max_payload_bytes: Option<u64>,
    pub max_concurrent_sessions: Option<u64>,
    pub quota: Option<WasmResourceQuotaKey>,
    pub metadata: BTreeMap<String, String>,
}

impl Default for WasmResourcePolicy {
    fn default() -> Self {
        Self {
            max_memory_bytes: Some(64 * 1024 * 1024),
            max_table_elements: Some(10_000),
            max_fuel: Some(10_000_000),
            max_epoch_ticks: Some(10_000),
            max_wall_time_ms: Some(30_000),
            max_host_call_time_ms: Some(5_000),
            max_host_calls: Some(1_000),
            max_payload_bytes: Some(DEFAULT_WASM_MAX_PAYLOAD_BYTES),
            max_concurrent_sessions: Some(DEFAULT_WASM_MAX_CONCURRENT_SESSIONS),
            quota: None,
            metadata: BTreeMap::new(),
        }
    }
}

impl WasmResourcePolicy {
    /// Build the default policy and attach an application/ability quota key.
    pub fn default_for(application_id: impl Into<String>, ability_id: impl Into<String>) -> Self {
        let application_id = application_id.into();
        let ability_id = ability_id.into();
        Self {
            quota: Some(WasmResourceQuotaKey::new(
                application_id,
                ability_id,
                "session",
            )),
            ..Self::default()
        }
    }

    /// Merge layers deterministically using the stricter numeric maximum.
    ///
    /// The method is a Specification helper: all callers receive the same
    /// result regardless of map iteration order because metadata is stored in a
    /// `BTreeMap` and every numeric field uses an explicit minimum rule.
    pub fn merge_deterministic(layers: &[Self]) -> Self {
        let mut merged = Self {
            max_memory_bytes: None,
            max_table_elements: None,
            max_fuel: None,
            max_epoch_ticks: None,
            max_wall_time_ms: None,
            max_host_call_time_ms: None,
            max_host_calls: None,
            max_payload_bytes: None,
            max_concurrent_sessions: None,
            quota: None,
            metadata: BTreeMap::new(),
        };
        for layer in layers {
            merged.max_memory_bytes =
                stricter_limit(merged.max_memory_bytes, layer.max_memory_bytes);
            merged.max_table_elements =
                stricter_limit(merged.max_table_elements, layer.max_table_elements);
            merged.max_fuel = stricter_limit(merged.max_fuel, layer.max_fuel);
            merged.max_epoch_ticks = stricter_limit(merged.max_epoch_ticks, layer.max_epoch_ticks);
            merged.max_wall_time_ms =
                stricter_limit(merged.max_wall_time_ms, layer.max_wall_time_ms);
            merged.max_host_call_time_ms =
                stricter_limit(merged.max_host_call_time_ms, layer.max_host_call_time_ms);
            merged.max_host_calls = stricter_limit(merged.max_host_calls, layer.max_host_calls);
            merged.max_payload_bytes =
                stricter_limit(merged.max_payload_bytes, layer.max_payload_bytes);
            merged.max_concurrent_sessions = stricter_limit(
                merged.max_concurrent_sessions,
                layer.max_concurrent_sessions,
            );
            if layer.quota.is_some() {
                merged.quota = layer.quota.clone();
            }
            merged.metadata.extend(layer.metadata.clone());
        }
        merged
    }
}

/// WASI preopen grant represented as a virtual capability, not a raw host path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WasmWasiPreopenGrant {
    pub virtual_label: String,
    pub capability_scope: String,
    pub access: String,
}

impl WasmWasiPreopenGrant {
    /// Build a sanitized preopen grant for future WASI-capable providers.
    pub fn new(
        virtual_label: impl Into<String>,
        capability_scope: impl Into<String>,
        access: impl Into<String>,
    ) -> Self {
        Self {
            virtual_label: sanitize_resource_label(virtual_label),
            capability_scope: sanitize_resource_label(capability_scope),
            access: sanitize_resource_label(access),
        }
    }
}

/// Provider-neutral WASI policy.
///
/// The default denies raw env, filesystem, and network access.  Future runtime
/// providers may consume the virtual preopen list, but they must still avoid
/// logging raw host paths or environment values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WasmWasiPolicy {
    pub raw_env_enabled: bool,
    pub raw_filesystem_enabled: bool,
    pub raw_network_enabled: bool,
    pub preopens: Vec<WasmWasiPreopenGrant>,
    pub metadata: BTreeMap<String, String>,
}

impl Default for WasmWasiPolicy {
    fn default() -> Self {
        Self {
            raw_env_enabled: false,
            raw_filesystem_enabled: false,
            raw_network_enabled: false,
            preopens: Vec::new(),
            metadata: BTreeMap::new(),
        }
    }
}

/// High-level sandbox policy consumed by runtime guards.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WasmSandboxPolicy {
    pub allow_raw_env: bool,
    pub allow_raw_filesystem: bool,
    pub allow_raw_network: bool,
    pub allow_unrestricted_clocks: bool,
    pub allow_unrestricted_randomness: bool,
    pub wasi: WasmWasiPolicy,
    pub metadata: BTreeMap<String, String>,
}

impl Default for WasmSandboxPolicy {
    fn default() -> Self {
        Self {
            allow_raw_env: false,
            allow_raw_filesystem: false,
            allow_raw_network: false,
            allow_unrestricted_clocks: false,
            allow_unrestricted_randomness: false,
            wasi: WasmWasiPolicy::default(),
            metadata: BTreeMap::new(),
        }
    }
}

impl WasmSandboxPolicy {
    /// Report whether raw WASI access is completely denied.
    pub fn denies_raw_wasi(&self) -> bool {
        !self.allow_raw_env
            && !self.allow_raw_filesystem
            && !self.allow_raw_network
            && !self.wasi.raw_env_enabled
            && !self.wasi.raw_filesystem_enabled
            && !self.wasi.raw_network_enabled
    }
}

/// Sanitized audit decision emitted by resource and sandbox guards.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WasmResourceAuditDecision {
    Allow,
    Deny,
    Throttle,
    Timeout,
    Exhausted,
}

/// Auditable, sanitized resource governance report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WasmResourceAuditReport {
    pub decision: WasmResourceAuditDecision,
    pub reason_code: String,
    pub runtime_kind: PackageRuntimeKind,
    pub application_id: String,
    pub ability_id: String,
    pub scope: String,
    pub trace_id: Option<String>,
    pub message: String,
    pub metadata: BTreeMap<String, String>,
}

impl WasmResourceAuditReport {
    /// Build an audit report and sanitize the human-readable message.
    pub fn new(
        decision: WasmResourceAuditDecision,
        reason_code: impl Into<String>,
        runtime_kind: PackageRuntimeKind,
        application_id: impl Into<String>,
        ability_id: impl Into<String>,
        scope: impl Into<String>,
        trace: Option<TraceContext>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            decision,
            reason_code: sanitize_resource_label(reason_code),
            runtime_kind,
            application_id: sanitize_resource_label(application_id),
            ability_id: sanitize_resource_label(ability_id),
            scope: sanitize_resource_label(scope),
            trace_id: trace.map(|value| value.trace_id),
            message: sanitize_resource_message(message),
            metadata: BTreeMap::new(),
        }
    }

    /// Return whether this audit report avoids known forbidden raw material.
    pub fn is_sanitized(&self) -> bool {
        let encoded = serde_json::to_string(self)
            .unwrap_or_default()
            .to_lowercase();
        !FORBIDDEN_RESOURCE_AUDIT_MARKERS
            .iter()
            .chain(FORBIDDEN_DIAGNOSTIC_MARKERS.iter())
            .any(|marker| encoded.contains(marker))
    }
}

fn stricter_limit(current: Option<u64>, next: Option<u64>) -> Option<u64> {
    match (current, next) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

fn sanitize_resource_label(value: impl Into<String>) -> String {
    let value = sanitize_diagnostic_text(value);
    let mut sanitized = String::with_capacity(value.len());
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | ':' | '.') {
            sanitized.push(ch);
        } else if ch == '/' {
            sanitized.push(':');
        } else {
            sanitized.push('_');
        }
    }
    sanitized.trim_matches('_').to_string()
}

fn sanitize_resource_message(value: impl Into<String>) -> String {
    let mut sanitized = sanitize_diagnostic_text(value);
    for marker in FORBIDDEN_RESOURCE_AUDIT_MARKERS {
        sanitized = sanitized.replace(marker, "[redacted]");
        sanitized = sanitized.replace(&marker.to_ascii_uppercase(), "[redacted]");
        if let Some(first) = marker.chars().nth(1) {
            let title_case = marker.replacen(first, &first.to_ascii_uppercase().to_string(), 1);
            sanitized = sanitized.replace(&title_case, "[redacted]");
        }
    }
    sanitized
}

const FORBIDDEN_RESOURCE_AUDIT_MARKERS: &[&str] = &[
    "raw filesystem",
    "raw network",
    "raw env",
    "memory dump",
    "stdout",
    "stderr",
    "/users/",
    "/etc/",
];
