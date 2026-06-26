use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

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
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub service_command_schemas: BTreeMap<String, BTreeSet<String>>,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub permission_scopes: BTreeSet<String>,
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
}

/// Data-only catalog entry for one domain pack.
#[derive(Debug, Clone, Default)]
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
        let metadata = minimal_metadata_from_pack_id(&pack_id);
        Self {
            pack_id,
            metadata,
            services: services.into_iter().collect(),
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
