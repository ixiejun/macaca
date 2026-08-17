use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::foundation_filesystem::FilesystemRootRef;

/// Optional pack declaration block in an application `service_contract` section.
///
/// The raw service lists remain available for low-level capability declarations, while pack
/// lists give applications a provider-neutral way to request capability bundles.  This keeps
/// application manifests extensible without forcing the OS to know any business domain.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppServiceContractConfig {
    /// Legacy compatibility list of required domain pack references.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub use_packs: Vec<String>,
    /// Required pack references that must resolve before execution.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_packs: Vec<String>,
    /// Optional pack references that may degrade if unavailable.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub optional_packs: Vec<String>,
    /// Provider-neutral permission scopes requested for each declared pack.
    ///
    /// The map is intentionally keyed by pack id and stores only descriptor-defined scope
    /// names. Catalog expansion validates it before exposing callable services, so manifests
    /// cannot grant themselves provider capabilities or bypass host policy.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub pack_permission_scopes: BTreeMap<String, BTreeSet<String>>,
    /// Logical roots declared for `pack.foundation.filesystem.v1`.
    ///
    /// These references contain an opaque root id and kind only. Runtime-host
    /// composition maps an admitted id to a local, remote, temporary, or plugin
    /// root; manifests must never include a host path or provider-native handle.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub filesystem_roots: Vec<FilesystemRootRef>,
    /// Mandatory service identifiers required by the application.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_services: Vec<String>,
    /// Optional service identifiers that can improve output quality.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub optional_services: Vec<String>,
    /// Per-service policy override declarations.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub service_policy_overrides: BTreeMap<String, AppServicePolicyOverride>,
    /// Per-pack policy override declarations.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub pack_policy_overrides: BTreeMap<String, AppPackPolicyOverride>,
}

/// Optional service policy override declared by an application.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppServicePolicyOverride {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_retries: Option<u32>,
}

/// Bounded pack policy override declared by an application.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppPackPolicyOverride {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_retries: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget_units: Option<u64>,
}

/// Declared lifecycle lane for discovery and admission decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DomainPackStability {
    Experimental,
    Preview,
    Stable,
    Deprecated,
    Retired,
}

impl Default for DomainPackStability {
    fn default() -> Self {
        Self::Experimental
    }
}

/// Runtime availability lane for catalog descriptors.
///
/// Stability explains product maturity, while availability explains whether a descriptor is
/// callable in the active composition root.  Keeping the two axes separate allows Macaca to ship
/// an industrial catalog that is discoverable before every optional provider exists.  Unavailable
/// entries remain useful for SDK docs and admission diagnostics, but they must never expand into
/// executable service capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DomainPackAvailability {
    Available,
    PreviewUnavailable,
    Unsupported,
    Retired,
}

impl Default for DomainPackAvailability {
    fn default() -> Self {
        Self::Available
    }
}

/// Pack policy defaults used during admission and runtime projection.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainPackPolicyTemplate {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_retries: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget_units: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_network: Option<bool>,
}

/// Governance metadata that keeps pack observability bounded and replay-safe.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainPackDataGovernance {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub classification: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub retention_policy: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub redaction_policy: String,
}

/// SDK-facing metadata for discovery and generated client surfaces.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainPackSdkMetadata {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub client_namespace: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub docs_url: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub examples: Vec<String>,
}

/// Diagnostic metadata that helps shells report availability without provider payloads.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainPackDiagnostics {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub health_probe: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub unavailable_reason: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub replay_schema: String,
}

/// Compatibility metadata for version and hierarchy validation.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainPackCompatibility {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub version_range: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub parent_version_range: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub service_version_ranges: BTreeMap<String, String>,
}

/// Provider-neutral descriptor for one replacement provider class.
///
/// Domain packs often support several provider classes: built-in service implementations,
/// host-native bridges, browser-hosted bridges, remote providers, plugin providers, mock
/// providers, and unavailable providers.  This DTO keeps those choices declarative so discovery,
/// SDK generation, trace, audit, and conformance tests can reason about replacement mechanics
/// without hardcoding provider names or business-domain branches into OS code.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainPackProviderDescriptor {
    /// Stable provider class label such as `plugin`, `mock`, or `unavailable`.
    pub provider_class: String,
    /// Optional service id that exposes this provider class through the service runtime.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub service_id: String,
    /// Current descriptor-level availability projected without contacting a concrete provider.
    #[serde(default)]
    pub availability: DomainPackProviderCapabilityState,
    /// Deterministic hash of the provider capability descriptor, never provider payload.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub capability_hash: String,
    /// Deterministic hash of compatibility constraints used by SDK and admission reports.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub compatibility_hash: String,
    /// Trace/audit schema emitted by this provider class, if it differs from pack defaults.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub diagnostics_schema: String,
    /// Bounded, provider-neutral feature flags such as `host_native` or `remote`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, String>,
}

impl DomainPackProviderDescriptor {
    /// Create a provider descriptor with sanitized class and service identifiers.
    pub fn new(provider_class: impl Into<String>, service_id: impl Into<String>) -> Self {
        Self {
            provider_class: provider_class.into().trim().to_string(),
            service_id: service_id.into().trim().to_string(),
            ..Default::default()
        }
    }

    /// Attach descriptor-level availability while preserving builder-style composition.
    pub fn with_availability(mut self, availability: DomainPackProviderCapabilityState) -> Self {
        self.availability = availability;
        self
    }
}

/// Immutable metadata for a domain-pack family or sub-pack.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainPackMetadata {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub family_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_pack_id: Option<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub version: String,
    #[serde(default)]
    pub stability: DomainPackStability,
    #[serde(default)]
    pub availability: DomainPackAvailability,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub service_command_schemas: BTreeMap<String, BTreeSet<String>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub service_result_schemas: BTreeMap<String, BTreeSet<String>>,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub permission_scopes: BTreeSet<String>,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub source_attribution: BTreeSet<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub migration_notes: Vec<String>,
    #[serde(default)]
    pub policy_template: DomainPackPolicyTemplate,
    #[serde(default)]
    pub data_governance: DomainPackDataGovernance,
    #[serde(default)]
    pub sdk: DomainPackSdkMetadata,
    #[serde(default)]
    pub diagnostics: DomainPackDiagnostics,
    #[serde(default)]
    pub compatibility: DomainPackCompatibility,
    /// Provider replacement descriptors keyed by stable provider class labels.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub provider_descriptors: BTreeMap<String, DomainPackProviderDescriptor>,
}

/// Data-only catalog entry for one domain pack.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainPackDefinition {
    pub pack_id: String,
    pub metadata: DomainPackMetadata,
    pub services: BTreeSet<String>,
}

impl DomainPackDefinition {
    /// Create a pack definition and derive minimal metadata from a valid `pack.*.vN` id.
    ///
    /// This constructor remains useful for tests and simple optional packages.  Explicit
    /// metadata should use [`Self::with_metadata`] when governance, SDK, or diagnostics fields
    /// are available.
    pub fn new(pack_id: impl Into<String>, services: impl IntoIterator<Item = String>) -> Self {
        let pack_id = pack_id.into();
        let services = services.into_iter().collect::<BTreeSet<_>>();
        let mut metadata = minimal_metadata_from_pack_id(&pack_id);
        metadata.service_command_schemas = services
            .iter()
            .map(|service| {
                (
                    service.clone(),
                    BTreeSet::from(["service.command.v1".into()]),
                )
            })
            .collect();
        metadata.service_result_schemas = services
            .iter()
            .map(|service| {
                (
                    service.clone(),
                    BTreeSet::from(["service.result.v1".into()]),
                )
            })
            .collect();
        Self {
            pack_id,
            metadata,
            services,
        }
    }

    /// Create a pack definition with explicit metadata.
    pub fn with_metadata(
        pack_id: impl Into<String>,
        metadata: DomainPackMetadata,
        services: impl IntoIterator<Item = String>,
    ) -> Self {
        Self {
            pack_id: pack_id.into(),
            metadata,
            services: services.into_iter().collect(),
        }
    }

    /// Return whether this descriptor is callable in the active composition root.
    ///
    /// The method intentionally checks only descriptor fields.  Service health and provider
    /// startup remain owned by the service runtime; this contract-level predicate prevents
    /// unavailable catalog entries from being projected as executable capabilities.
    pub fn is_callable(&self) -> bool {
        matches!(
            self.metadata.availability,
            DomainPackAvailability::Available
        ) && !matches!(
            self.metadata.stability,
            DomainPackStability::Retired | DomainPackStability::Deprecated
        ) && !self.services.is_empty()
    }

    /// Return a deterministic, provider-neutral descriptor hash for this pack definition.
    ///
    /// The hash is intentionally computed from the serialized descriptor DTO rather than from
    /// runtime provider state.  This makes it safe to put into admission reports, SDK discovery
    /// responses, trace events, audit snapshots, and replay evidence without leaking provider
    /// payloads, credentials, manifests, prompts, package bytes, or application data.  The
    /// implementation uses the same small FNV-style accumulator as catalog snapshots so the base
    /// protocol crate does not need a crypto dependency for non-security identity evidence.
    pub fn stable_descriptor_hash(&self) -> String {
        let payload = serde_json::to_vec(self).unwrap_or_default();
        let digest = payload.iter().fold(0_u64, |state, byte| {
            state.wrapping_mul(1099511628211).wrapping_add(*byte as u64)
        });
        format!("{digest:016x}")
    }
}

fn minimal_metadata_from_pack_id(pack_id: &str) -> DomainPackMetadata {
    let Some((family_path, version_suffix)) = pack_id.rsplit_once(".v") else {
        return DomainPackMetadata::default();
    };
    let family_id = family_path
        .trim_start_matches("pack.")
        .split('.')
        .next()
        .unwrap_or_default()
        .to_string();
    DomainPackMetadata {
        family_id,
        version: format!("v{version_suffix}"),
        ..Default::default()
    }
}

/// Sanitized provider snapshot projected by package registration and discovery code.
///
/// This Memento intentionally carries only bounded descriptor fields.  It gives SDKs and shells
/// enough information to explain health and availability without exposing provider payloads,
/// secrets, raw manifests, package bytes, prompts, or business-domain responses.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainPackProviderSnapshot {
    pub pack_id: String,
    pub service_id: String,
    pub provider_class: String,
    pub health: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unavailable_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
}

/// Provider capability state used by discovery, diagnostics, and SDK surfaces.
///
/// This enum is deliberately provider-neutral.  It gives shells and admission reports one
/// bounded vocabulary for optional providers without forcing the OS to understand concrete
/// provider names, product plans, model names, host platforms, or business-domain states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DomainPackProviderCapabilityState {
    Available,
    Degraded,
    Preview,
    Unavailable,
    Unsupported,
    Retired,
}

impl Default for DomainPackProviderCapabilityState {
    fn default() -> Self {
        Self::Unavailable
    }
}

/// Sanitized provider capability report derived from descriptors and provider snapshots.
///
/// Reports are Mementos for discovery and audit.  They intentionally exclude raw provider
/// payloads, credentials, manifests, prompts, package bytes, account data, private keys, and
/// unbounded diagnostic output.  Provider adapters can publish richer health internally, but SDK
/// and shell surfaces should use this bounded shape when explaining whether a pack provider can be
/// called.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainPackProviderCapabilityReport {
    pub pack_id: String,
    pub service_id: String,
    pub provider_class: String,
    pub state: DomainPackProviderCapabilityState,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub reason_code: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
}

impl DomainPackProviderSnapshot {
    /// Build a healthy descriptor-only snapshot after a package provider registers.
    pub fn registered(
        pack_id: impl Into<String>,
        service_id: impl Into<String>,
        trace_id: impl Into<String>,
    ) -> Self {
        Self {
            pack_id: pack_id.into(),
            service_id: service_id.into(),
            provider_class: "package".into(),
            health: "registered".into(),
            unavailable_reason: None,
            trace_id: Some(trace_id.into()),
        }
    }

    /// Build a fail-closed unavailable snapshot for absent or incompatible packs.
    pub fn unavailable(
        pack_id: impl Into<String>,
        service_id: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            pack_id: pack_id.into(),
            service_id: service_id.into(),
            provider_class: "unavailable".into(),
            health: "unavailable".into(),
            unavailable_reason: Some(reason.into()),
            trace_id: None,
        }
    }

    /// Convert this bounded snapshot into a normalized provider capability report.
    ///
    /// The mapping is intentionally string-tolerant so older provider snapshots remain useful
    /// during serviceization.  Unknown health values fail closed as `unavailable`; adapters that
    /// need richer states should emit one of the normalized state labels in the snapshot health
    /// field until they migrate to first-class reports.
    pub fn capability_report(&self) -> DomainPackProviderCapabilityReport {
        let state = match self.health.as_str() {
            "registered" | "available" | "healthy" => DomainPackProviderCapabilityState::Available,
            "degraded" => DomainPackProviderCapabilityState::Degraded,
            "preview" => DomainPackProviderCapabilityState::Preview,
            "unsupported" => DomainPackProviderCapabilityState::Unsupported,
            "retired" => DomainPackProviderCapabilityState::Retired,
            _ => DomainPackProviderCapabilityState::Unavailable,
        };
        DomainPackProviderCapabilityReport {
            pack_id: self.pack_id.clone(),
            service_id: self.service_id.clone(),
            provider_class: self.provider_class.clone(),
            state,
            reason_code: self
                .unavailable_reason
                .clone()
                .unwrap_or_else(|| self.health.clone()),
            trace_id: self.trace_id.clone(),
        }
    }
}

/// SDK-facing unavailable explanation for one pack declaration.
///
/// Required and optional declarations share this DTO so admission, SDK tooling, and shells can
/// render the same bounded diagnostic while keeping required-pack blocking semantics explicit.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainPackUnavailableDiagnostic {
    pub pack_id: String,
    pub required: bool,
    pub reason_code: String,
    pub message: String,
}

impl DomainPackUnavailableDiagnostic {
    /// Create a diagnostic from already-sanitized pack resolution state.
    pub fn new(
        pack_id: impl Into<String>,
        required: bool,
        reason_code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            pack_id: pack_id.into(),
            required,
            reason_code: reason_code.into(),
            message: message.into(),
        }
    }
}
