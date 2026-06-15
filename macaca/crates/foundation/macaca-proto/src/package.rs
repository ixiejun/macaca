//! Protocol-level package manifest contracts for Macaca Agent OS.
//!
//! A package is the smallest distributable software unit.  These
//! contracts are data-only: they describe what a package is, what it needs,
//! what it provides, and how it wants to run.  Actual loading, guard checks,
//! Store entitlement, payment, and runtime execution remain outside this
//! module so the protocol can stay stable across applications, skills,
//! plugins, drivers, MCP packages, system modules, and future UI packs.

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::{CapabilityId, CommerceMetadata, KernelServiceId};

/// Stable package identity.
///
/// The value is string-backed so package ids can use reverse-DNS, workspace
/// local ids, Store-assigned ids, or future decentralized identities without a
/// protocol enum edit.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PackageId(String);

impl PackageId {
    /// Create a package id after trimming surrounding whitespace.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into().trim().to_string())
    }

    /// Return the raw package id.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PackageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Stable developer identity attached to package metadata.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DeveloperId(String);

impl DeveloperId {
    /// Create a developer id after trimming surrounding whitespace.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into().trim().to_string())
    }

    /// Return the raw developer id.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DeveloperId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Supported package categories.
///
/// `Custom` keeps the package ecosystem open to future third-party package
/// kinds without requiring a kernel/protocol source edit.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackageType {
    Application,
    Skill,
    Plugin,
    Mcp,
    Driver,
    SystemModule,
    UiComponentPack,
    Custom(String),
}

impl fmt::Display for PackageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Application => f.write_str("application"),
            Self::Skill => f.write_str("skill"),
            Self::Plugin => f.write_str("plugin"),
            Self::Mcp => f.write_str("mcp"),
            Self::Driver => f.write_str("driver"),
            Self::SystemModule => f.write_str("system_module"),
            Self::UiComponentPack => f.write_str("ui_component_pack"),
            Self::Custom(value) => f.write_str(value),
        }
    }
}

/// Runtime category requested by a package.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PackageRuntimeKind {
    Yaml,
    WasmComponent,
    NativeAdapter,
    RemoteService,
    EncryptedTextBundle,
    Custom(String),
}

impl fmt::Display for PackageRuntimeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Yaml => f.write_str("yaml"),
            Self::WasmComponent => f.write_str("wasm_component"),
            Self::NativeAdapter => f.write_str("native_adapter"),
            Self::RemoteService => f.write_str("remote_service"),
            Self::EncryptedTextBundle => f.write_str("encrypted_text_bundle"),
            Self::Custom(value) => f.write_str(value),
        }
    }
}

/// Signature metadata carried by package manifests.
///
/// Phase 04 validates metadata shape only.  Cryptographic verification,
/// Store trust roots, and paid package enforcement belong to later phases.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageSignature {
    pub algorithm: String,
    pub key_id: String,
    pub digest: String,
}

/// Runtime declaration for a package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageRuntime {
    pub kind: Option<PackageRuntimeKind>,
    pub abi_version: String,
}

impl PackageRuntime {
    /// Create a runtime declaration for a known runtime kind.
    pub fn new(kind: PackageRuntimeKind, abi_version: impl Into<String>) -> Self {
        Self {
            kind: Some(kind),
            abi_version: abi_version.into(),
        }
    }
}

/// Entry declaration for a package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageEntry {
    pub kind: String,
    pub value: String,
}

/// Permission requested by a package before activation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackagePermission {
    pub name: String,
    pub reason: String,
}

/// Service dependency declared by a package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageServiceRequirement {
    pub service: KernelServiceId,
    pub capability: Option<CapabilityId>,
    pub reason: String,
}

/// Capability provided by a package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageCapability {
    pub id: CapabilityId,
    pub description: String,
}

/// Host requirement facts evaluated by runtime guards.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageHostRequirements {
    pub min_os_version: Option<String>,
    pub supported_abi_versions: Vec<String>,
}

impl Default for PackageHostRequirements {
    fn default() -> Self {
        Self {
            min_os_version: None,
            supported_abi_versions: vec!["0".into()],
        }
    }
}

/// Canonical Package Manifest v0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageManifest {
    pub id: PackageId,
    pub package_type: PackageType,
    pub version: String,
    pub developer: DeveloperId,
    pub signature: Option<PackageSignature>,
    pub runtime: PackageRuntime,
    pub entry: Option<PackageEntry>,
    pub permissions: Vec<PackagePermission>,
    pub required_services: Vec<PackageServiceRequirement>,
    pub optional_services: Vec<PackageServiceRequirement>,
    pub provides: Vec<PackageCapability>,
    pub commerce: CommerceMetadata,
    pub host_requirements: PackageHostRequirements,
    pub metadata: BTreeMap<String, String>,
}

impl PackageManifest {
    /// Build a minimal manifest suitable for adapters and tests.
    pub fn new(
        id: PackageId,
        package_type: PackageType,
        version: impl Into<String>,
        developer: DeveloperId,
        runtime: PackageRuntime,
    ) -> Self {
        Self {
            id,
            package_type,
            version: version.into(),
            developer,
            signature: None,
            runtime,
            entry: None,
            permissions: Vec::new(),
            required_services: Vec::new(),
            optional_services: Vec::new(),
            provides: Vec::new(),
            commerce: CommerceMetadata::default(),
            host_requirements: PackageHostRequirements::default(),
            metadata: BTreeMap::new(),
        }
    }
}

/// Availability state for optional service requirements after guard checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PackageServiceAvailability {
    Available {
        service: KernelServiceId,
    },
    Unavailable {
        service: KernelServiceId,
        reason: String,
    },
}

/// Guard outcome attached to package descriptor diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PackageGuardDecision {
    Accepted,
    AcceptedWithUnavailableOptionalServices,
    Rejected { error: PackageGuardError },
}

/// Canonical package descriptor after adapter normalization and guard checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageDescriptor {
    pub manifest: PackageManifest,
    pub optional_service_status: Vec<PackageServiceAvailability>,
    pub decision: PackageGuardDecision,
    pub trace_events: Vec<String>,
}

impl PackageDescriptor {
    /// Create a descriptor with no optional-service degradations.
    pub fn new(manifest: PackageManifest) -> Self {
        Self {
            manifest,
            optional_service_status: Vec::new(),
            decision: PackageGuardDecision::Accepted,
            trace_events: Vec::new(),
        }
    }
}

/// Structured errors emitted by package guards and loaders.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
pub enum PackageGuardError {
    #[error("missing runtime kind")]
    MissingRuntimeKind,

    #[error("unsupported package type: {0}")]
    UnsupportedPackageType(String),

    #[error("unsupported runtime kind: {0}")]
    UnsupportedRuntimeKind(String),

    #[error("ABI rejected: required {required}, supported {supported}")]
    AbiRejected { required: String, supported: String },

    #[error("missing required service: {0}")]
    MissingRequiredService(String),

    #[error("permission denied: {0}")]
    PermissionDenied(String),

    #[error("invalid signature metadata: {0}")]
    InvalidSignatureMetadata(String),

    #[error("runtime unavailable: {0}")]
    RuntimeUnavailable(String),

    #[error("invalid manifest: {0}")]
    InvalidManifest(String),
}

/// Result alias for package contract operations.
pub type PackageResult<T> = Result<T, PackageGuardError>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LicenseType;

    fn fixture_manifest(
        package_type: PackageType,
        runtime_kind: PackageRuntimeKind,
    ) -> PackageManifest {
        let mut manifest = PackageManifest::new(
            PackageId::new("pkg.test"),
            package_type,
            "1.0.0",
            DeveloperId::new("dev.test"),
            PackageRuntime::new(runtime_kind, "1"),
        );
        manifest.entry = Some(PackageEntry {
            kind: "agent".into(),
            value: "entry".into(),
        });
        manifest.permissions.push(PackagePermission {
            name: "workspace.read".into(),
            reason: "Read package workspace".into(),
        });
        manifest.required_services.push(PackageServiceRequirement {
            service: KernelServiceId::new("service.required"),
            capability: Some(CapabilityId::new("cap.required")),
            reason: "Required service".into(),
        });
        manifest.optional_services.push(PackageServiceRequirement {
            service: KernelServiceId::new("service.optional"),
            capability: None,
            reason: "Optional service".into(),
        });
        manifest.provides.push(PackageCapability {
            id: CapabilityId::new("cap.provided"),
            description: "Provided capability".into(),
        });
        manifest.commerce.license_type = LicenseType::free();
        manifest
            .metadata
            .insert("format".into(), "package-manifest-v0".into());
        manifest
    }

    #[test]
    fn package_manifest_round_trips() {
        let manifest = fixture_manifest(PackageType::Application, PackageRuntimeKind::Yaml);

        let encoded = serde_json::to_vec(&manifest).unwrap();
        let decoded: PackageManifest = serde_json::from_slice(&encoded).unwrap();

        assert_eq!(decoded, manifest);
        assert_eq!(decoded.id.as_str(), "pkg.test");
        assert_eq!(decoded.developer.as_str(), "dev.test");
    }

    #[test]
    fn every_required_package_type_round_trips() {
        let package_types = vec![
            PackageType::Application,
            PackageType::Skill,
            PackageType::Plugin,
            PackageType::Mcp,
            PackageType::Driver,
            PackageType::SystemModule,
            PackageType::UiComponentPack,
        ];

        for package_type in package_types {
            let manifest = fixture_manifest(package_type, PackageRuntimeKind::Yaml);
            let encoded = serde_json::to_string(&manifest).unwrap();
            let decoded: PackageManifest = serde_json::from_str(&encoded).unwrap();
            assert_eq!(
                decoded.package_type.to_string(),
                manifest.package_type.to_string()
            );
        }
    }

    #[test]
    fn future_package_and_runtime_kinds_are_structured_data() {
        let manifest = fixture_manifest(
            PackageType::Custom("future_pack".into()),
            PackageRuntimeKind::Custom("future_runtime".into()),
        );

        let encoded = serde_json::to_string(&manifest).unwrap();
        let decoded: PackageManifest = serde_json::from_str(&encoded).unwrap();

        assert_eq!(
            decoded.package_type,
            PackageType::Custom("future_pack".into())
        );
        assert_eq!(
            decoded.runtime.kind,
            Some(PackageRuntimeKind::Custom("future_runtime".into()))
        );
    }
}
