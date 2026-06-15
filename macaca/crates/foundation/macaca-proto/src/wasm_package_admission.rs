//! WASM package admission and ABI negotiation contracts.
//!
//! This module owns control-plane DTOs for admitting WASM component
//! application packages.  It deliberately stores artifact references, digests,
//! ABI requirements, import requirements, export declarations, and negotiation
//! outcomes as data only.  It never stores raw WASM bytes, raw WIT bodies, raw
//! manifests, raw signatures, secrets, environment values, API keys, prompts,
//! or host payloads.  Runtime execution remains behind `wasm_runtime_provider`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{
    ApplicationExport, ApplicationImport, WasmEngineCapabilities, WasmRuntimeArtifactRef,
    WasmSupplyChainAttestation,
};

/// Stable artifact digest metadata used during WASM package admission.
///
/// The digest stores an algorithm label and digest value, never artifact bytes.
/// Admission can therefore prove that a package declared immutable artifact
/// metadata without moving the component body into manifests, logs, traces, or
/// reports. Full cryptographic chain validation remains a later Store/trust
/// root concern; this contract slice validates shape and auditability.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct WasmArtifactDigest {
    pub algorithm: String,
    pub value: String,
}

impl WasmArtifactDigest {
    /// Build a SHA-256 digest declaration after trimming surrounding space.
    pub fn sha256(value: impl Into<String>) -> Self {
        Self {
            algorithm: "sha256".into(),
            value: value.into().trim().to_string(),
        }
    }

    /// Return whether the digest carries enough metadata for admission.
    pub fn is_present(&self) -> bool {
        !self.algorithm.trim().is_empty() && !self.value.trim().is_empty()
    }
}

/// ABI requirement declared by a WASM package before negotiation.
///
/// The version is string-backed so Macaca can start with simple semantic
/// equality while preserving room for later ranges, aliases, or host profiles.
/// profiles. Capability flags are generic strings instead of provider names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WasmAbiRequirement {
    pub version: String,
    pub required_features: Vec<String>,
    pub metadata: BTreeMap<String, String>,
}

impl WasmAbiRequirement {
    /// Create a requirement for one exact ABI version.
    pub fn new(version: impl Into<String>) -> Self {
        Self {
            version: version.into().trim().to_string(),
            required_features: Vec::new(),
            metadata: BTreeMap::new(),
        }
    }

    /// Attach one provider-neutral feature flag and keep flags sorted.
    pub fn feature(mut self, feature: impl Into<String>) -> Self {
        let feature = feature.into().trim().to_string();
        if !feature.is_empty() {
            self.required_features.push(feature);
            self.required_features.sort();
            self.required_features.dedup();
        }
        self
    }
}

/// Host import requirement declared by a WASM artifact.
///
/// Imports are ABI labels from `ApplicationImport`. A required import may map
/// to a manifest permission. Admission uses this DTO to fail closed before
/// runtime creation when a guest asks for host authority the manifest did not
/// declare.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WasmImportRequirement {
    pub import: ApplicationImport,
    pub permission: Option<String>,
    pub optional: bool,
    pub metadata: BTreeMap<String, String>,
}

impl WasmImportRequirement {
    /// Create a required import that must be backed by a permission.
    pub fn permissioned(import: ApplicationImport, permission: impl Into<String>) -> Self {
        Self {
            import,
            permission: Some(permission.into().trim().to_string()),
            optional: false,
            metadata: BTreeMap::new(),
        }
    }

    /// Return the stable sort key used for deterministic descriptors.
    fn sort_key(&self) -> String {
        format!(
            "{}:{}:{}",
            self.import,
            self.permission.as_deref().unwrap_or(""),
            self.optional
        )
    }
}

/// Export declaration connecting a WASM export to an application ability.
///
/// This is metadata only. It does not prove an export exists in component
/// bytes; it records what the package declares so admission/certification can
/// negotiate against WIT/ABI expectations before a real runtime appears.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WasmExportDeclaration {
    pub export: ApplicationExport,
    pub ability_id: String,
    pub metadata: BTreeMap<String, String>,
}

impl WasmExportDeclaration {
    /// Create an export declaration for one application ability.
    pub fn ability(export: ApplicationExport, ability_id: impl Into<String>) -> Self {
        Self {
            export,
            ability_id: ability_id.into().trim().to_string(),
            metadata: BTreeMap::new(),
        }
    }

    /// Return the stable sort key used for deterministic descriptors.
    fn sort_key(&self) -> String {
        format!("{}:{}", self.export, self.ability_id)
    }
}

/// Metadata-only descriptor for one WASM component artifact.
///
/// The descriptor is an Adapter target for package metadata and future
/// Store metadata. It carries artifact id, artifact reference, digest,
/// signature reference, ABI requirement, imports, exports, and safe metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WasmComponentArtifactDescriptor {
    pub artifact_id: String,
    pub artifact_ref: WasmRuntimeArtifactRef,
    pub digest: WasmArtifactDigest,
    pub signature_algorithm: Option<String>,
    pub signature_ref: Option<String>,
    pub supply_chain: Option<WasmSupplyChainAttestation>,
    pub abi: WasmAbiRequirement,
    pub required_imports: Vec<WasmImportRequirement>,
    pub exports: Vec<WasmExportDeclaration>,
    pub metadata: BTreeMap<String, String>,
}

impl WasmComponentArtifactDescriptor {
    /// Create a descriptor with a default Application ABI v0 requirement.
    pub fn new(
        artifact_id: impl Into<String>,
        artifact_ref: WasmRuntimeArtifactRef,
        digest: WasmArtifactDigest,
    ) -> Self {
        Self {
            artifact_id: artifact_id.into().trim().to_string(),
            artifact_ref,
            digest,
            signature_algorithm: None,
            signature_ref: None,
            supply_chain: None,
            abi: WasmAbiRequirement::new("0"),
            required_imports: Vec::new(),
            exports: Vec::new(),
            metadata: BTreeMap::new(),
        }
    }

    /// Attach bounded signature metadata without storing the raw signature.
    pub fn signature(
        mut self,
        algorithm: impl Into<String>,
        signature_ref: impl Into<String>,
    ) -> Self {
        self.signature_algorithm = Some(algorithm.into().trim().to_string());
        self.signature_ref = Some(signature_ref.into().trim().to_string());
        self
    }

    /// Attach provider-neutral supply-chain proof metadata.
    ///
    /// The attestation stores detached proof references and provenance labels,
    /// not raw signatures or key material. Admission can therefore enforce
    /// industrial trust policy while preserving safe manifest/report contents.
    pub fn supply_chain(mut self, attestation: WasmSupplyChainAttestation) -> Self {
        self.supply_chain = Some(attestation);
        self
    }

    /// Replace the ABI requirement.
    pub fn abi(mut self, abi: WasmAbiRequirement) -> Self {
        self.abi = abi;
        self
    }

    /// Add one import requirement and sort for stable serialization.
    pub fn required_import(mut self, import: WasmImportRequirement) -> Self {
        self.required_imports.push(import);
        self.required_imports
            .sort_by_key(WasmImportRequirement::sort_key);
        self
    }

    /// Add one export declaration and sort for stable serialization.
    pub fn export(mut self, export: WasmExportDeclaration) -> Self {
        self.exports.push(export);
        self.exports.sort_by_key(WasmExportDeclaration::sort_key);
        self
    }
}

/// Result of provider-neutral WASM ABI negotiation.
///
/// Negotiation is fail-closed. It records the requested version, selected
/// supported version when admitted, runtime capability snapshot, stable
/// reason codes, and safe metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WasmAbiNegotiationResult {
    pub admitted: bool,
    pub requested_version: String,
    pub selected_version: Option<String>,
    pub capabilities: WasmEngineCapabilities,
    pub reason_codes: Vec<String>,
    pub metadata: BTreeMap<String, String>,
}

impl WasmAbiNegotiationResult {
    /// Negotiate an exact ABI version against the host-supported set.
    pub fn negotiate(
        requirement: WasmAbiRequirement,
        mut supported_versions: Vec<String>,
        capabilities: WasmEngineCapabilities,
    ) -> Self {
        supported_versions.sort();
        supported_versions.dedup();
        let selected_version = supported_versions
            .iter()
            .find(|version| *version == &requirement.version)
            .cloned();
        let mut reason_codes = Vec::new();
        if selected_version.is_none() {
            reason_codes.push("abi_version_mismatch".into());
        }
        if !capabilities.can_execute
            || !capabilities.supports_component_model
            || !capabilities.supports_host_import_bridge
        {
            reason_codes.push("runtime_capability_missing".into());
        }
        reason_codes.sort();
        reason_codes.dedup();
        Self {
            admitted: selected_version.is_some() && reason_codes.is_empty(),
            requested_version: requirement.version,
            selected_version,
            capabilities,
            reason_codes,
            metadata: BTreeMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::{
        ApplicationExport, ApplicationImport, WasmAbiNegotiationResult, WasmAbiRequirement,
        WasmArtifactDigest, WasmComponentArtifactDescriptor, WasmEngineCapabilities,
        WasmExportDeclaration, WasmImportRequirement, WasmRuntimeArtifactRef,
    };

    #[test]
    fn wasm_abi_artifact_descriptor_serializes_stably() {
        let descriptor = WasmComponentArtifactDescriptor::new(
            "artifact.fixture",
            WasmRuntimeArtifactRef::new("pkg://fixture/component.wasm"),
            WasmArtifactDigest::sha256("abc123"),
        )
        .signature("sigstore", "signature-ref")
        .abi(WasmAbiRequirement::new("0"))
        .required_import(WasmImportRequirement::permissioned(
            ApplicationImport::StorageSet,
            "storage.write",
        ))
        .export(WasmExportDeclaration::ability(
            ApplicationExport::Start,
            "ability.fixture",
        ));

        let encoded = serde_json::to_value(&descriptor).unwrap();

        assert_eq!(encoded["artifact_id"], json!("artifact.fixture"));
        assert_eq!(encoded["digest"]["algorithm"], json!("sha256"));
        assert_eq!(
            encoded["required_imports"][0]["permission"],
            json!("storage.write")
        );
    }

    #[test]
    fn wasm_abi_negotiation_fails_closed_on_version_mismatch() {
        let result = WasmAbiNegotiationResult::negotiate(
            WasmAbiRequirement::new("99"),
            vec!["0".into(), "1".into()],
            WasmEngineCapabilities::unavailable(),
        );

        assert!(!result.admitted);
        assert!(result
            .reason_codes
            .iter()
            .any(|code| code == "abi_version_mismatch"));
    }

    #[test]
    fn wasm_abi_negotiation_matches_supported_version() {
        let mut capabilities = WasmEngineCapabilities::unavailable();
        capabilities.can_execute = true;
        capabilities.supports_component_model = true;
        capabilities.supports_host_import_bridge = true;
        let result = WasmAbiNegotiationResult::negotiate(
            WasmAbiRequirement::new("0"),
            vec!["0".into(), "1".into()],
            capabilities,
        );

        assert!(result.admitted);
        assert_eq!(result.selected_version.as_deref(), Some("0"));
    }
}
