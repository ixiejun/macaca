//! WASM supply-chain admission specification.
//!
//! This module keeps industrial artifact trust checks separate from the main
//! package-admission flow.  The separation follows the Specification pattern:
//! each rule evaluates deterministic DTOs and returns a sanitized Memento
//! report.  No rule loads component bytes, opens a key store, calls a runtime
//! provider, or logs raw signature/certificate material.

use macaca_proto::{
    WasmComponentArtifactDescriptor, WasmSupplyChainTrustPolicy, WasmSupplyChainVerificationReport,
};

/// Deterministic verifier for provider-neutral supply-chain metadata.
#[derive(Debug, Clone, Default)]
pub struct WasmSupplyChainAdmissionSpec;

impl WasmSupplyChainAdmissionSpec {
    /// Verify one artifact descriptor against a caller-supplied trust policy.
    ///
    /// This method is deliberately deterministic.  It compares descriptor
    /// digest, detached signature shape, signer allowlist, source-origin
    /// allowlist, provenance freshness, and certification compatibility from
    /// data already present in the fixture.  A later production cryptographic
    /// verifier can be injected behind this Specification without changing the
    /// report shape or admission orchestration.
    pub fn verify(
        &self,
        artifact: &WasmComponentArtifactDescriptor,
        policy: &WasmSupplyChainTrustPolicy,
    ) -> WasmSupplyChainVerificationReport {
        let mut report = WasmSupplyChainVerificationReport::verified(&artifact.artifact_id);
        let Some(attestation) = &artifact.supply_chain else {
            report.reject("missing_signature");
            self.log(&report);
            return report;
        };

        report.signer_id = Some(attestation.signature.signer_id.clone());
        report.source_origin = Some(attestation.provenance.source_origin.clone());
        report.build_id = Some(attestation.provenance.build_id.clone());
        report.certification_report_id = attestation.certification_report_id.clone();

        if !attestation.signature.is_present() {
            report.reject("missing_signature");
        }
        if attestation.artifact_digest != artifact.digest {
            report.reject("digest_mismatch");
        }
        if !policy.trusted_signers.is_empty()
            && !policy
                .trusted_signers
                .contains(&attestation.signature.signer_id)
        {
            report.reject("untrusted_signer");
        }
        if !policy.accepted_origins.is_empty()
            && !policy
                .accepted_origins
                .contains(&attestation.provenance.source_origin)
        {
            report.reject("origin_mismatch");
        }
        if self.is_stale(policy, attestation.provenance.issued_at_epoch_seconds) {
            report.reject("stale_provenance");
        }
        if let (Some(now), Some(expires_at)) = (
            policy.evaluation_epoch_seconds,
            attestation.provenance.expires_at_epoch_seconds,
        ) {
            if now > expires_at {
                report.reject("stale_provenance");
            }
        }
        if policy.require_certification && attestation.certification_report_id.is_none() {
            report.reject("incompatible_certification");
        }

        self.log(&report);
        report
    }

    /// Check max-age freshness without depending on wall-clock time.
    ///
    /// The caller supplies `evaluation_epoch_seconds` in the trust policy so
    /// tests, offline certification, and CI admission can reproduce the exact
    /// same decision later for audit.
    fn is_stale(&self, policy: &WasmSupplyChainTrustPolicy, issued_at: u64) -> bool {
        let (Some(max_age), Some(now)) = (
            policy.max_provenance_age_seconds,
            policy.evaluation_epoch_seconds,
        ) else {
            return false;
        };
        now.saturating_sub(issued_at) > max_age
    }

    fn log(&self, report: &WasmSupplyChainVerificationReport) {
        tracing::info!(
            artifact_id = %report.artifact_id,
            status = ?report.status,
            reason_codes = ?report.reason_codes,
            signer_id = report.signer_id.as_deref().unwrap_or(""),
            source_origin = report.source_origin.as_deref().unwrap_or(""),
            "WASM supply-chain verification completed"
        );
    }
}

#[cfg(test)]
#[path = "wasm_supply_chain_tests.rs"]
mod wasm_supply_chain_tests;
