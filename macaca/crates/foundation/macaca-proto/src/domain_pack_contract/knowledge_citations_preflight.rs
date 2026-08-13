//! Provider-neutral admission Specification for knowledge citations.
//!
//! Host-owned policy and resource services reduce their decisions to bounded
//! evidence before this module runs. Rejected requests therefore never reach a
//! resolver, formatter, or source adapter with raw source or provider data.

use serde::{Deserialize, Serialize};

use super::knowledge_citations::{
    knowledge_citations_pack_definition, CitationItem, CitationSourceAnchor,
};
use super::pack_preflight::{
    DomainPackCommandPreflight, DomainPackCommandPreflightSpec, DomainPackPreflightRejection,
    DomainPackPreflightStatus,
};

/// Bounded host decisions required before citation provider dispatch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CitationAdmissionEvidence {
    pub source_accessible: bool,
    pub source_anchor_valid: bool,
    pub identifier_scheme_supported: bool,
    pub resolver_policy_allowed: bool,
    pub style_supported: bool,
    pub import_export_within_limit: bool,
    pub quote_redacted: bool,
    pub output_bounded: bool,
    pub rate_limit_available: bool,
    pub timeout_available: bool,
    pub resource_budget_available: bool,
    pub provider_capability_available: bool,
    pub payload_redacted: bool,
}

/// Descriptor-driven gate for every `citations.*` command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CitationDispatchPreflight {
    inner: DomainPackCommandPreflightSpec,
}

impl CitationDispatchPreflight {
    /// Construct a citation gate with host-selected approval requirements.
    pub fn new(approval_required_commands: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            inner: DomainPackCommandPreflightSpec::from_definition(
                &knowledge_citations_pack_definition(),
                approval_required_commands,
            ),
        }
    }

    /// Validate bounded citation references and host decisions before dispatch.
    pub fn evaluate(
        &self,
        item: Option<&CitationItem>,
        preflight: &DomainPackCommandPreflight,
        evidence: &CitationAdmissionEvidence,
    ) -> Result<(), DomainPackPreflightRejection> {
        self.inner.evaluate(preflight)?;
        if requested_scope(&preflight.command_name) != Some(preflight.requested_scope.as_str()) {
            return Err(reject(
                DomainPackPreflightStatus::Denied,
                "citation_command_scope_mismatch",
            ));
        }
        if requires_item(&preflight.command_name) && !item.is_some_and(valid_item) {
            return Err(reject(
                DomainPackPreflightStatus::Denied,
                "citation_input_invalid_or_unredacted",
            ));
        }
        for (allowed, status, reason) in checks(evidence) {
            if !allowed {
                return Err(reject(status, reason));
            }
        }
        Ok(())
    }

    /// Invoke the provider closure only after complete admission acceptance.
    pub fn dispatch_after_preflight<T>(
        &self,
        item: Option<&CitationItem>,
        preflight: &DomainPackCommandPreflight,
        evidence: &CitationAdmissionEvidence,
        dispatch: impl FnOnce() -> T,
    ) -> Result<T, DomainPackPreflightRejection> {
        self.evaluate(item, preflight, evidence)?;
        Ok(dispatch())
    }
}

fn requested_scope(command: &str) -> Option<&'static str> {
    match command {
        "citations.create_citation" => Some("citation.create"),
        "citations.resolve_identifier" => Some("citation.resolve"),
        "citations.link_source_span" => Some("citation.source.link"),
        "citations.verify_citation" => Some("citation.verify"),
        "citations.format_citation" | "citations.format_bibliography" => Some("citation.format"),
        "citations.list_citations" | "citations.inspect_provider" => Some("citation.read"),
        "citations.update_citation" => Some("citation.update"),
        "citations.import_citations" | "citations.export_citations" => {
            Some("citation.import_export")
        }
        "citations.inspect_source_anchor" => Some("citation.evidence.read"),
        _ => None,
    }
}

fn requires_item(command: &str) -> bool {
    matches!(
        command,
        "citations.create_citation"
            | "citations.link_source_span"
            | "citations.verify_citation"
            | "citations.format_citation"
            | "citations.update_citation"
    )
}

fn valid_item(item: &CitationItem) -> bool {
    bounded(&item.citation_id)
        && bounded(&item.title_ref)
        && !sensitive(&item.title_ref)
        && item.identifiers.len() <= 16
        && item.identifiers.iter().all(|identifier| {
            bounded(&identifier.scheme)
                && bounded(&identifier.normalized_value)
                && !sensitive(&identifier.normalized_value)
        })
        && item.source_anchor.as_ref().is_none_or(valid_anchor)
}

fn valid_anchor(anchor: &CitationSourceAnchor) -> bool {
    bounded(&anchor.source_ref)
        && !sensitive(&anchor.source_ref)
        && anchor
            .quote_ref
            .as_ref()
            .is_none_or(|quote| bounded(quote) && !sensitive(quote))
        && anchor.selectors.len() <= 16
        && anchor
            .selectors
            .iter()
            .all(|selector| bounded(&selector.selector_kind) && selector.is_bounded(16_384))
}

fn bounded(value: &str) -> bool {
    !value.is_empty() && value.len() <= 128 && !value.contains('\n')
}

fn sensitive(value: &str) -> bool {
    [
        "credential",
        "provider_payload",
        "raw_document",
        "private_quote",
        "raw_style",
        "private_corpus",
    ]
    .iter()
    .any(|marker| value.contains(marker))
}

fn checks(
    evidence: &CitationAdmissionEvidence,
) -> Vec<(bool, DomainPackPreflightStatus, &'static str)> {
    vec![
        (
            evidence.source_accessible,
            DomainPackPreflightStatus::Denied,
            "citation_source_access_denied",
        ),
        (
            evidence.source_anchor_valid,
            DomainPackPreflightStatus::Denied,
            "citation_source_anchor_invalid",
        ),
        (
            evidence.identifier_scheme_supported,
            DomainPackPreflightStatus::Unsupported,
            "citation_identifier_scheme_unsupported",
        ),
        (
            evidence.resolver_policy_allowed,
            DomainPackPreflightStatus::Denied,
            "citation_resolver_policy_denied",
        ),
        (
            evidence.style_supported,
            DomainPackPreflightStatus::Unsupported,
            "citation_style_unsupported",
        ),
        (
            evidence.import_export_within_limit,
            DomainPackPreflightStatus::QuotaExceeded,
            "citation_import_export_limit_exceeded",
        ),
        (
            evidence.quote_redacted,
            DomainPackPreflightStatus::Denied,
            "citation_quote_redaction_required",
        ),
        (
            evidence.output_bounded,
            DomainPackPreflightStatus::Denied,
            "citation_output_bound_required",
        ),
        (
            evidence.rate_limit_available,
            DomainPackPreflightStatus::QuotaExceeded,
            "citation_rate_limited",
        ),
        (
            evidence.timeout_available,
            DomainPackPreflightStatus::QuotaExceeded,
            "citation_timeout_exceeded",
        ),
        (
            evidence.resource_budget_available,
            DomainPackPreflightStatus::QuotaExceeded,
            "citation_resource_budget_exhausted",
        ),
        (
            evidence.provider_capability_available,
            DomainPackPreflightStatus::Unsupported,
            "citation_provider_capability_unsupported",
        ),
        (
            evidence.payload_redacted,
            DomainPackPreflightStatus::Denied,
            "citation_payload_redaction_required",
        ),
    ]
}

fn reject(status: DomainPackPreflightStatus, reason: &'static str) -> DomainPackPreflightRejection {
    DomainPackPreflightRejection {
        status,
        reason_code: reason.into(),
    }
}
