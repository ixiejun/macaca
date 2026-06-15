//! Provider-neutral WASM artifact supply-chain contracts.
//!
//! These DTOs describe the proof material that package admission may inspect
//! before a WASM component is considered industrial-ready. They intentionally
//! store references, identifiers, digest values, and provenance labels only;
//! raw signatures, public keys, certificates, build logs, source archives, and
//! component bytes stay outside manifests, telemetry, and certification
//! reports. This keeps the data contract portable across future Store, CI,
//! KMS, Sigstore, or offline signing integrations while preserving safe audit
//! trails today.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::WasmArtifactDigest;

/// Reference to a detached artifact signature.
///
/// The structure follows the Adapter pattern: callers can point to Sigstore,
/// TUF, KMS, or internal signing evidence without leaking provider-specific
/// key material into Macaca public contracts. Admission uses `signer_id` and
/// `signature_ref` as stable audit identifiers; real cryptographic validation
/// can be added behind a verifier implementation without changing this DTO.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WasmArtifactSignature {
    pub algorithm: String,
    pub signer_id: String,
    pub signature_ref: String,
    pub metadata: BTreeMap<String, String>,
}

impl WasmArtifactSignature {
    /// Build a detached signature reference after trimming human input.
    pub fn detached(
        algorithm: impl Into<String>,
        signer_id: impl Into<String>,
        signature_ref: impl Into<String>,
    ) -> Self {
        Self {
            algorithm: algorithm.into().trim().to_string(),
            signer_id: signer_id.into().trim().to_string(),
            signature_ref: signature_ref.into().trim().to_string(),
            metadata: BTreeMap::new(),
        }
    }

    /// Return whether the signature has enough shape for deterministic
    /// admission. This does not perform cryptographic verification.
    pub fn is_present(&self) -> bool {
        !self.algorithm.is_empty() && !self.signer_id.is_empty() && !self.signature_ref.is_empty()
    }
}

/// Build provenance for the signed artifact.
///
/// `source_origin` is a stable trust-domain label rather than a raw repository
/// URL. This allows an organization to define policies such as "company-ci"
/// or "trusted-store" without exposing internal source paths in reports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WasmBuildProvenance {
    pub builder_id: String,
    pub source_origin: String,
    pub build_id: String,
    pub issued_at_epoch_seconds: u64,
    pub expires_at_epoch_seconds: Option<u64>,
    pub metadata: BTreeMap<String, String>,
}

impl WasmBuildProvenance {
    /// Create deterministic provenance for admission and test fixtures.
    pub fn new(
        builder_id: impl Into<String>,
        source_origin: impl Into<String>,
        build_id: impl Into<String>,
        issued_at_epoch_seconds: u64,
    ) -> Self {
        Self {
            builder_id: builder_id.into().trim().to_string(),
            source_origin: source_origin.into().trim().to_string(),
            build_id: build_id.into().trim().to_string(),
            issued_at_epoch_seconds,
            expires_at_epoch_seconds: None,
            metadata: BTreeMap::new(),
        }
    }

    /// Attach an explicit expiry for policies that require freshness.
    pub fn expires_at(mut self, expires_at_epoch_seconds: u64) -> Self {
        self.expires_at_epoch_seconds = Some(expires_at_epoch_seconds);
        self
    }
}

/// Complete supply-chain attestation attached to one artifact descriptor.
///
/// The artifact digest is repeated inside the attestation so admission can
/// detect descriptor/proof drift without loading component bytes. The report
/// id links this proof to a sanitized certification result; it is optional so
/// development fixtures can still exercise unsigned or partially signed paths.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WasmSupplyChainAttestation {
    pub artifact_digest: WasmArtifactDigest,
    pub signature: WasmArtifactSignature,
    pub provenance: WasmBuildProvenance,
    pub certification_report_id: Option<String>,
    pub metadata: BTreeMap<String, String>,
}

impl WasmSupplyChainAttestation {
    /// Build a complete attestation from deterministic proof references.
    pub fn new(
        artifact_digest: WasmArtifactDigest,
        signature: WasmArtifactSignature,
        provenance: WasmBuildProvenance,
    ) -> Self {
        Self {
            artifact_digest,
            signature,
            provenance,
            certification_report_id: None,
            metadata: BTreeMap::new(),
        }
    }

    /// Attach a sanitized certification report id.
    pub fn certification_report(mut self, report_id: impl Into<String>) -> Self {
        let report_id = report_id.into().trim().to_string();
        if !report_id.is_empty() {
            self.certification_report_id = Some(report_id);
        }
        self
    }
}

/// Provider-neutral trust policy consumed by admission.
///
/// This Specification input is data-only. Store, CI, or deployment tooling can
/// construct it from their own trust roots, while the application framework can
/// enforce signer/origin/freshness/certification rules without importing crypto
/// engines or runtime-host providers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WasmSupplyChainTrustPolicy {
    pub trusted_signers: BTreeSet<String>,
    pub accepted_origins: BTreeSet<String>,
    pub max_provenance_age_seconds: Option<u64>,
    pub evaluation_epoch_seconds: Option<u64>,
    pub require_certification: bool,
}

impl Default for WasmSupplyChainTrustPolicy {
    fn default() -> Self {
        Self {
            trusted_signers: BTreeSet::new(),
            accepted_origins: BTreeSet::new(),
            max_provenance_age_seconds: None,
            evaluation_epoch_seconds: None,
            require_certification: true,
        }
    }
}

impl WasmSupplyChainTrustPolicy {
    /// Add one trusted signer id and keep deterministic set ordering.
    pub fn trusted_signer(mut self, signer_id: impl Into<String>) -> Self {
        let signer_id = signer_id.into().trim().to_string();
        if !signer_id.is_empty() {
            self.trusted_signers.insert(signer_id);
        }
        self
    }

    /// Add one accepted source-origin label.
    pub fn accepted_origin(mut self, origin: impl Into<String>) -> Self {
        let origin = origin.into().trim().to_string();
        if !origin.is_empty() {
            self.accepted_origins.insert(origin);
        }
        self
    }

    /// Require provenance no older than the supplied age at evaluation time.
    pub fn max_age(mut self, seconds: u64, evaluation_epoch_seconds: u64) -> Self {
        self.max_provenance_age_seconds = Some(seconds);
        self.evaluation_epoch_seconds = Some(evaluation_epoch_seconds);
        self
    }

    /// Allow development fixtures to skip certification-report conformance.
    pub fn allow_without_certification(mut self) -> Self {
        self.require_certification = false;
        self
    }
}

/// Sanitized outcome of supply-chain verification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WasmSupplyChainVerificationStatus {
    Verified,
    Rejected,
}

/// Memento-style verification report safe for audit and telemetry.
///
/// The report records stable ids and reason codes only. It never includes raw
/// signature bytes, certificate chains, source URLs, environment values, build
/// logs, or component payloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WasmSupplyChainVerificationReport {
    pub artifact_id: String,
    pub status: WasmSupplyChainVerificationStatus,
    pub reason_codes: Vec<String>,
    pub signer_id: Option<String>,
    pub source_origin: Option<String>,
    pub build_id: Option<String>,
    pub certification_report_id: Option<String>,
    pub metadata: BTreeMap<String, String>,
}

impl WasmSupplyChainVerificationReport {
    /// Create an optimistic report; verification rules may later reject it.
    pub fn verified(artifact_id: impl Into<String>) -> Self {
        Self {
            artifact_id: artifact_id.into().trim().to_string(),
            status: WasmSupplyChainVerificationStatus::Verified,
            reason_codes: Vec::new(),
            signer_id: None,
            source_origin: None,
            build_id: None,
            certification_report_id: None,
            metadata: BTreeMap::new(),
        }
    }

    /// Add one stable reason code and mark the report rejected.
    pub fn reject(&mut self, code: impl Into<String>) {
        self.status = WasmSupplyChainVerificationStatus::Rejected;
        self.reason_codes.push(code.into());
        self.reason_codes.sort();
        self.reason_codes.dedup();
    }

    /// Return whether the report avoids known raw or sensitive material.
    pub fn is_sanitized(&self) -> bool {
        serde_json::to_string(self)
            .map(|encoded| {
                let encoded = encoded.to_ascii_lowercase();
                ![
                    "raw wasm",
                    "raw_signature",
                    "private key",
                    "secret",
                    "api_key",
                    "apikey",
                    "env",
                    "build log",
                    "source archive",
                ]
                .iter()
                .any(|marker| encoded.contains(marker))
            })
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::{
        WasmArtifactDigest, WasmArtifactSignature, WasmBuildProvenance, WasmSupplyChainAttestation,
        WasmSupplyChainTrustPolicy, WasmSupplyChainVerificationReport,
    };

    #[test]
    fn wasm_supply_chain_attestation_serializes_stably() {
        let attestation = WasmSupplyChainAttestation::new(
            WasmArtifactDigest::sha256("abc123"),
            WasmArtifactSignature::detached("sigstore", "signer.fixture", "sig://fixture"),
            WasmBuildProvenance::new("builder.fixture", "origin.fixture", "build.fixture", 100),
        )
        .certification_report("cert.fixture");

        let encoded = serde_json::to_value(&attestation).unwrap();

        assert_eq!(encoded["artifact_digest"]["algorithm"], json!("sha256"));
        assert_eq!(encoded["signature"]["signer_id"], json!("signer.fixture"));
        assert_eq!(
            encoded["provenance"]["source_origin"],
            json!("origin.fixture")
        );
        assert_eq!(encoded["certification_report_id"], json!("cert.fixture"));
    }

    #[test]
    fn wasm_supply_chain_policy_serializes_deterministic_sets() {
        let policy = WasmSupplyChainTrustPolicy::default()
            .trusted_signer("signer.b")
            .trusted_signer("signer.a")
            .accepted_origin("origin.b")
            .accepted_origin("origin.a")
            .max_age(30, 100);

        let encoded = serde_json::to_value(&policy).unwrap();

        assert_eq!(encoded["trusted_signers"][0], json!("signer.a"));
        assert_eq!(encoded["accepted_origins"][0], json!("origin.a"));
        assert_eq!(encoded["max_provenance_age_seconds"], json!(30));
    }

    #[test]
    fn wasm_supply_chain_report_is_sanitized() {
        let mut report = WasmSupplyChainVerificationReport::verified("artifact.fixture");
        report.reject("missing_signature");
        report
            .metadata
            .insert("diagnostic".into(), "redacted verifier branch".into());

        let encoded = serde_json::to_string(&report).unwrap().to_lowercase();

        assert!(report.is_sanitized());
        assert!(!encoded.contains("raw_signature"));
        assert!(!encoded.contains("private key"));
    }
}
