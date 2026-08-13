//! Provider-neutral admission Specification for document parsing.
//!
//! The runtime host supplies policy, scanning, entitlement, and metering facts
//! as bounded evidence. This gate rejects unsafe work before a parser adapter
//! can observe document bytes, OCR images, embedded content, or provider data.

use serde::{Deserialize, Serialize};

use super::knowledge_document_parsing::{
    knowledge_document_parsing_pack_definition, DocumentSource,
};
use super::pack_preflight::{
    DomainPackCommandPreflight, DomainPackCommandPreflightSpec, DomainPackPreflightRejection,
    DomainPackPreflightStatus,
};

/// Trace-safe host decisions required before document-parser dispatch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentParsingAdmissionEvidence {
    pub document_owned: bool,
    pub document_handle_valid: bool,
    pub media_type_allowed: bool,
    pub malware_scan_clean: bool,
    pub encryption_policy_allowed: bool,
    pub size_page_output_available: bool,
    pub ocr_policy_allowed: bool,
    pub embedded_resource_policy_allowed: bool,
    pub provider_capability_available: bool,
    pub timeout_available: bool,
    pub rate_limit_available: bool,
    pub resource_budget_available: bool,
    pub payload_redacted: bool,
    pub output_bounded: bool,
}

/// Descriptor-driven gate for every `document_parsing.*` command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentParsingDispatchPreflight {
    inner: DomainPackCommandPreflightSpec,
}

impl DocumentParsingDispatchPreflight {
    /// Construct a parser gate with host-selected approval requirements.
    pub fn new(approval_required_commands: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            inner: DomainPackCommandPreflightSpec::from_definition(
                &knowledge_document_parsing_pack_definition(),
                approval_required_commands,
            ),
        }
    }

    /// Validate source references and host evidence before provider dispatch.
    pub fn evaluate(
        &self,
        source: Option<&DocumentSource>,
        preflight: &DomainPackCommandPreflight,
        evidence: &DocumentParsingAdmissionEvidence,
    ) -> Result<(), DomainPackPreflightRejection> {
        self.inner.evaluate(preflight)?;
        if requested_scope(&preflight.command_name) != Some(preflight.requested_scope.as_str()) {
            return Err(reject(
                DomainPackPreflightStatus::Denied,
                "document_parsing_command_scope_mismatch",
            ));
        }
        if requires_source(&preflight.command_name) && !source.is_some_and(valid_source) {
            return Err(reject(
                DomainPackPreflightStatus::Denied,
                "document_parsing_source_invalid_or_unredacted",
            ));
        }
        for (allowed, status, reason) in checks(evidence) {
            if !allowed {
                return Err(reject(status, reason));
            }
        }
        Ok(())
    }

    /// Invoke a parser closure only after all admission checks pass.
    pub fn dispatch_after_preflight<T>(
        &self,
        source: Option<&DocumentSource>,
        preflight: &DomainPackCommandPreflight,
        evidence: &DocumentParsingAdmissionEvidence,
        dispatch: impl FnOnce() -> T,
    ) -> Result<T, DomainPackPreflightRejection> {
        self.evaluate(source, preflight, evidence)?;
        Ok(dispatch())
    }
}

fn requested_scope(command: &str) -> Option<&'static str> {
    match command {
        "document_parsing.detect_format"
        | "document_parsing.validate_document"
        | "document_parsing.parse_document"
        | "document_parsing.start_parse_job"
        | "document_parsing.get_parse_job"
        | "document_parsing.cancel_parse_job" => Some("document.parse"),
        "document_parsing.extract_text" => Some("document.extract.text"),
        "document_parsing.extract_layout" => Some("document.extract.layout"),
        "document_parsing.extract_tables" => Some("document.extract.table"),
        "document_parsing.extract_forms" => Some("document.extract.form"),
        "document_parsing.extract_metadata" => Some("document.extract.metadata"),
        "document_parsing.convert_to_canonical" => Some("document.convert"),
        "document_parsing.chunk_document" => Some("document.chunk"),
        "document_parsing.inspect_parser" => Some("document.parser.inspect"),
        _ => None,
    }
}

fn requires_source(command: &str) -> bool {
    !matches!(
        command,
        "document_parsing.inspect_parser"
            | "document_parsing.get_parse_job"
            | "document_parsing.cancel_parse_job"
    )
}

fn valid_source(source: &DocumentSource) -> bool {
    bounded(&source.document_ref)
        && bounded(&source.media_type)
        && source.size_bytes > 0
        && source.size_bytes <= 100_000_000
        && !sensitive(&source.document_ref)
        && !sensitive(&source.media_type)
        && matches!(source.malware_scan_state.as_str(), "clean" | "verified")
}

fn bounded(value: &str) -> bool {
    !value.is_empty() && value.len() <= 128 && !value.contains('\n')
}

fn sensitive(value: &str) -> bool {
    [
        "credential",
        "provider_payload",
        "raw_document",
        "raw_ocr_image",
        "raw_embedded_file",
        "private_signature",
        "private_corpus",
    ]
    .iter()
    .any(|marker| value.contains(marker))
}

fn checks(
    evidence: &DocumentParsingAdmissionEvidence,
) -> Vec<(bool, DomainPackPreflightStatus, &'static str)> {
    vec![
        (
            evidence.document_owned,
            DomainPackPreflightStatus::Denied,
            "document_parsing_ownership_denied",
        ),
        (
            evidence.document_handle_valid,
            DomainPackPreflightStatus::Denied,
            "document_parsing_handle_invalid",
        ),
        (
            evidence.media_type_allowed,
            DomainPackPreflightStatus::Unsupported,
            "document_parsing_media_type_unsupported",
        ),
        (
            evidence.malware_scan_clean,
            DomainPackPreflightStatus::Denied,
            "document_parsing_malware_scan_required",
        ),
        (
            evidence.encryption_policy_allowed,
            DomainPackPreflightStatus::Denied,
            "document_parsing_encryption_policy_denied",
        ),
        (
            evidence.size_page_output_available,
            DomainPackPreflightStatus::QuotaExceeded,
            "document_parsing_output_budget_exceeded",
        ),
        (
            evidence.ocr_policy_allowed,
            DomainPackPreflightStatus::Denied,
            "document_parsing_ocr_policy_denied",
        ),
        (
            evidence.embedded_resource_policy_allowed,
            DomainPackPreflightStatus::Denied,
            "document_parsing_embedded_resource_policy_denied",
        ),
        (
            evidence.provider_capability_available,
            DomainPackPreflightStatus::Unsupported,
            "document_parsing_provider_capability_unsupported",
        ),
        (
            evidence.timeout_available,
            DomainPackPreflightStatus::QuotaExceeded,
            "document_parsing_timeout_exceeded",
        ),
        (
            evidence.rate_limit_available,
            DomainPackPreflightStatus::QuotaExceeded,
            "document_parsing_rate_limited",
        ),
        (
            evidence.resource_budget_available,
            DomainPackPreflightStatus::QuotaExceeded,
            "document_parsing_resource_budget_exhausted",
        ),
        (
            evidence.payload_redacted,
            DomainPackPreflightStatus::Denied,
            "document_parsing_payload_redaction_required",
        ),
        (
            evidence.output_bounded,
            DomainPackPreflightStatus::Denied,
            "document_parsing_output_bound_required",
        ),
    ]
}

fn reject(status: DomainPackPreflightStatus, reason: &'static str) -> DomainPackPreflightRejection {
    DomainPackPreflightRejection {
        status,
        reason_code: reason.into(),
    }
}
