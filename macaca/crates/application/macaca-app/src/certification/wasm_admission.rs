//! WASM package admission specifications and reports.
//!
//! This module is the Application Framework control-plane admission layer for
//! WASM component packages.  It uses Specification checks for artifact
//! metadata, ABI negotiation, required import permissions, runtime capability
//! matching, and report sanitization.  The output is a Memento-style report:
//! it captures stable identifiers and reason codes for audit without retaining
//! raw WASM bytes, raw manifests, raw host payloads, secrets, environment
//! values, API keys, private keys, prompts, raw signatures, or provider output.

use std::collections::BTreeSet;

use macaca_proto::{
    ApplicationManifestV1, PackageDescriptor, PackageRuntimeKind, WasmAbiNegotiationResult,
    WasmComponentArtifactDescriptor, WasmEngineCapabilities, WasmSupplyChainVerificationReport,
    WasmSupplyChainVerificationStatus,
};

use super::types::{ApplicationCertificationContext, ApplicationCertificationFixture};
use super::wasm_supply_chain::WasmSupplyChainAdmissionSpec;

/// Final status for a WASM package admission report.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WasmPackageAdmissionStatus {
    Accepted,
    Rejected,
    Unavailable,
}

/// Safe diagnostic emitted by one WASM admission specification.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WasmPackageAdmissionDiagnostic {
    pub code: String,
    pub subject: String,
    pub message: String,
}

impl WasmPackageAdmissionDiagnostic {
    fn new(
        code: impl Into<String>,
        subject: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            subject: subject.into(),
            message: sanitize_text(message),
        }
    }
}

/// Sanitized Memento produced by WASM package admission.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WasmPackageAdmissionReport {
    pub package_id: String,
    pub runtime_kind: PackageRuntimeKind,
    pub abi_version: String,
    pub artifact_id: Option<String>,
    pub status: WasmPackageAdmissionStatus,
    pub reason_codes: Vec<String>,
    pub trace_id: Option<String>,
    pub supply_chain: Option<WasmSupplyChainVerificationReport>,
    pub diagnostics: Vec<WasmPackageAdmissionDiagnostic>,
}

impl WasmPackageAdmissionReport {
    fn new(manifest: &ApplicationManifestV1, trace_id: Option<String>) -> Self {
        Self {
            package_id: manifest.package_id.to_string(),
            runtime_kind: manifest.runtime.kind.clone(),
            abi_version: manifest.runtime.abi_version.clone(),
            artifact_id: None,
            status: WasmPackageAdmissionStatus::Accepted,
            reason_codes: Vec::new(),
            trace_id,
            supply_chain: None,
            diagnostics: Vec::new(),
        }
    }

    fn reject(
        &mut self,
        code: impl Into<String>,
        subject: impl Into<String>,
        message: impl Into<String>,
    ) {
        let code = code.into();
        self.reason_codes.push(code.clone());
        self.reason_codes.sort();
        self.reason_codes.dedup();
        self.status = WasmPackageAdmissionStatus::Rejected;
        self.diagnostics
            .push(WasmPackageAdmissionDiagnostic::new(code, subject, message));
    }

    fn unavailable(
        &mut self,
        code: impl Into<String>,
        subject: impl Into<String>,
        message: impl Into<String>,
    ) {
        let code = code.into();
        self.reason_codes.push(code.clone());
        self.reason_codes.sort();
        self.reason_codes.dedup();
        if self.status == WasmPackageAdmissionStatus::Accepted {
            self.status = WasmPackageAdmissionStatus::Unavailable;
        }
        self.diagnostics
            .push(WasmPackageAdmissionDiagnostic::new(code, subject, message));
    }

    /// Return whether the report avoids known sensitive/raw material.
    pub fn is_sanitized(&self) -> bool {
        serde_json::to_string(self)
            .map(|encoded| {
                let encoded = encoded.to_ascii_lowercase();
                !FORBIDDEN_REPORT_MARKERS
                    .iter()
                    .any(|marker| encoded.contains(marker))
            })
            .unwrap_or(false)
    }

    /// Build the report emitted by the legacy metadata-only WASM descriptor.
    ///
    /// This Adapter path intentionally reports `Unavailable` instead of
    /// `Accepted`: the package metadata can be inspected, but Macaca still has
    /// no approved real WASM runtime in this slice.  The report captures only
    /// identifiers and reason codes so legacy callers become traceable without
    /// exposing raw package manifests or component bytes.
    pub fn legacy_metadata_only(package: &PackageDescriptor, trace_id: Option<String>) -> Self {
        let mut report = Self {
            package_id: package.manifest.id.to_string(),
            runtime_kind: package
                .manifest
                .runtime
                .kind
                .clone()
                .unwrap_or(PackageRuntimeKind::WasmComponent),
            abi_version: package.manifest.runtime.abi_version.clone(),
            artifact_id: Some(
                package
                    .manifest
                    .metadata
                    .get("wasm.artifact.id")
                    .cloned()
                    .unwrap_or_else(|| package.manifest.id.to_string()),
            ),
            status: WasmPackageAdmissionStatus::Unavailable,
            reason_codes: vec!["metadata_only_runtime_unavailable".into()],
            trace_id,
            supply_chain: None,
            diagnostics: Vec::new(),
        };
        report.diagnostics.push(WasmPackageAdmissionDiagnostic::new(
            "metadata_only_runtime_unavailable",
            package.manifest.id.as_str(),
            "WASM metadata was admitted without executing component code",
        ));
        report
    }
}

/// Facade for composable WASM package admission specifications.
#[derive(Debug, Clone)]
pub struct WasmPackageAdmissionSpec {
    supported_abi_versions: BTreeSet<String>,
}

impl Default for WasmPackageAdmissionSpec {
    fn default() -> Self {
        Self {
            supported_abi_versions: BTreeSet::from(["0".into()]),
        }
    }
}

impl WasmPackageAdmissionSpec {
    /// Evaluate a fixture and produce a sanitized admission report.
    ///
    /// The method never reads artifact bytes or dispatches host imports.  It
    /// only compares metadata DTOs, manifest declarations, and provider-neutral
    /// runtime capabilities supplied by the caller.
    pub fn evaluate(
        &self,
        fixture: &ApplicationCertificationFixture,
        context: &ApplicationCertificationContext,
    ) -> WasmPackageAdmissionReport {
        let mut report =
            WasmPackageAdmissionReport::new(&fixture.manifest, context.trace_id.clone());
        let Some(artifact) = &fixture.wasm_artifact else {
            report.reject(
                "artifact_missing",
                fixture.manifest.package_id.as_str(),
                "WASM package admission requires an artifact descriptor",
            );
            self.log(&report);
            return report;
        };
        report.artifact_id = Some(artifact.artifact_id.clone());
        self.check_artifact(artifact, &mut report);
        self.check_supply_chain(artifact, context, &mut report);
        self.check_abi(artifact, context, &mut report);
        self.check_import_permissions(&fixture.manifest, artifact, &mut report);
        self.check_report_sanitization(artifact, &mut report);
        self.log(&report);
        report
    }

    fn check_artifact(
        &self,
        artifact: &WasmComponentArtifactDescriptor,
        report: &mut WasmPackageAdmissionReport,
    ) {
        if artifact.artifact_id.trim().is_empty() || artifact.artifact_ref.is_empty() {
            report.reject(
                "artifact_reference_missing",
                artifact.artifact_id.clone(),
                "WASM artifact id and reference are required",
            );
        }
        if !artifact.digest.is_present() {
            report.reject(
                "artifact_digest_missing",
                artifact.artifact_id.clone(),
                "WASM artifact digest metadata is required",
            );
        }
    }

    fn check_supply_chain(
        &self,
        artifact: &WasmComponentArtifactDescriptor,
        context: &ApplicationCertificationContext,
        report: &mut WasmPackageAdmissionReport,
    ) {
        let Some(policy) = &context.wasm_supply_chain_policy else {
            tracing::debug!(
                artifact_id = %artifact.artifact_id,
                "WASM supply-chain policy not provided; verification skipped for non-industrial profile"
            );
            return;
        };
        let supply_chain = WasmSupplyChainAdmissionSpec.verify(artifact, policy);
        if supply_chain.status != WasmSupplyChainVerificationStatus::Verified {
            for code in &supply_chain.reason_codes {
                report.reject(
                    code.clone(),
                    artifact.artifact_id.clone(),
                    "WASM artifact supply-chain verification failed",
                );
            }
        }
        report.supply_chain = Some(supply_chain);
    }

    fn check_abi(
        &self,
        artifact: &WasmComponentArtifactDescriptor,
        context: &ApplicationCertificationContext,
        report: &mut WasmPackageAdmissionReport,
    ) {
        let supported = if context.supported_wasm_abi_versions.is_empty() {
            self.supported_abi_versions.iter().cloned().collect()
        } else {
            context
                .supported_wasm_abi_versions
                .iter()
                .cloned()
                .collect()
        };
        let capabilities = context
            .wasm_runtime_capabilities
            .clone()
            .unwrap_or_else(WasmEngineCapabilities::unavailable);
        let negotiation =
            WasmAbiNegotiationResult::negotiate(artifact.abi.clone(), supported, capabilities);
        for code in negotiation.reason_codes {
            if code == "runtime_capability_missing" {
                report.unavailable(
                    code,
                    artifact.artifact_id.clone(),
                    "WASM runtime capabilities do not satisfy package requirements",
                );
            } else {
                report.reject(
                    code,
                    artifact.artifact_id.clone(),
                    "WASM ABI requirement is incompatible with this host",
                );
            }
        }
    }

    fn check_import_permissions(
        &self,
        manifest: &ApplicationManifestV1,
        artifact: &WasmComponentArtifactDescriptor,
        report: &mut WasmPackageAdmissionReport,
    ) {
        let declared_permissions: BTreeSet<String> = manifest
            .permissions
            .iter()
            .map(|permission| permission.name.clone())
            .chain(
                manifest
                    .abilities
                    .iter()
                    .flat_map(|ability| ability.permissions.iter())
                    .map(|permission| permission.name.clone()),
            )
            .collect();
        for import in &artifact.required_imports {
            if import.optional {
                continue;
            }
            if let Some(permission) = &import.permission {
                if !declared_permissions.contains(permission) {
                    report.reject(
                        "import_permission_missing",
                        import.import.to_string(),
                        "Required WASM host import permission is not declared",
                    );
                }
            }
        }
    }

    fn check_report_sanitization(
        &self,
        artifact: &WasmComponentArtifactDescriptor,
        report: &mut WasmPackageAdmissionReport,
    ) {
        for key in artifact.metadata.keys() {
            let lower = key.to_ascii_lowercase();
            if FORBIDDEN_REPORT_MARKERS
                .iter()
                .any(|marker| lower.contains(marker))
            {
                report.reject(
                    "unsafe_wasm_metadata",
                    artifact.artifact_id.clone(),
                    "Unsafe WASM artifact metadata category is not allowed in admission reports",
                );
            }
        }
    }

    fn log(&self, report: &WasmPackageAdmissionReport) {
        tracing::info!(
            package_id = %report.package_id,
            runtime_kind = %report.runtime_kind,
            abi_version = %report.abi_version,
            status = ?report.status,
            reason_codes = ?report.reason_codes,
            trace_id = report.trace_id.as_deref().unwrap_or(""),
            "WASM package admission completed"
        );
    }
}

const FORBIDDEN_REPORT_MARKERS: &[&str] = &[
    "raw wasm",
    "raw_manifest",
    "raw manifest",
    "raw payload",
    "secret",
    "api_key",
    "apikey",
    "private key",
    "env",
    "prompt",
];

fn sanitize_text(value: impl Into<String>) -> String {
    let mut sanitized = value.into();
    for marker in FORBIDDEN_REPORT_MARKERS {
        sanitized = sanitized.replace(marker, "[redacted]");
        sanitized = sanitized.replace(&marker.to_uppercase(), "[redacted]");
    }
    sanitized
}

#[cfg(test)]
#[path = "wasm_admission_tests.rs"]
mod wasm_admission_tests;
