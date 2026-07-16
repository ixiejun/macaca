//! Provider-neutral admission Specification for knowledge summarization.
//!
//! The host evaluates identity, source access, policy, entitlement, approval,
//! and resource facts before constructing this evidence. This module combines
//! those bounded facts with descriptor-owned command and scope rules, so a
//! provider closure cannot observe a rejected request or private source content.

use serde::{Deserialize, Serialize};

use super::knowledge_summarization::{
    knowledge_summarization_pack_definition, SummaryRequest, SummarySource,
};
use super::pack_preflight::{
    DomainPackCommandPreflight, DomainPackCommandPreflightSpec, DomainPackPreflightRejection,
    DomainPackPreflightStatus,
};

/// Host-evaluated facts required for a summarization provider invocation.
///
/// The fields are decision outcomes only, not raw documents, prompts, tokens,
/// policy records, credentials, or provider payloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SummarizationAdmissionEvidence {
    pub source_handles_accessible: bool,
    pub source_kinds_supported: bool,
    pub mode_allowed: bool,
    pub output_schema_allowed: bool,
    pub target_length_allowed: bool,
    pub language_allowed: bool,
    pub evidence_allowed: bool,
    pub quote_allowed: bool,
    pub freshness_allowed: bool,
    pub sensitivity_allowed: bool,
    pub compression_retention_allowed: bool,
    pub quality_threshold_met: bool,
    pub chunk_limit_available: bool,
    pub streaming_available: bool,
    pub timeout_available: bool,
    pub memory_available: bool,
    pub storage_available: bool,
    pub network_allowed: bool,
    pub provider_quota_available: bool,
    pub evaluation_budget_available: bool,
    pub snapshot_capacity_available: bool,
    pub citation_support_available: bool,
    pub compression_support_available: bool,
    pub evaluation_support_available: bool,
}

/// Descriptor-driven gate for every `summarization.*` command.
///
/// The gate applies the Specification pattern over shared preflight rules. It
/// maps only pack-owned commands to pack-owned scopes and never selects a model
/// or interprets an application's business behavior.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SummarizationDispatchPreflight {
    inner: DomainPackCommandPreflightSpec,
}

impl SummarizationDispatchPreflight {
    /// Construct a gate with host-selected commands that require approval.
    pub fn new(approval_required_commands: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            inner: DomainPackCommandPreflightSpec::from_definition(
                &knowledge_summarization_pack_definition(),
                approval_required_commands,
            ),
        }
    }

    /// Evaluate descriptor, source-reference, policy, and host evidence before dispatch.
    pub fn evaluate(
        &self,
        request: Option<&SummaryRequest>,
        preflight: &DomainPackCommandPreflight,
        evidence: &SummarizationAdmissionEvidence,
    ) -> Result<(), DomainPackPreflightRejection> {
        self.inner.evaluate(preflight)?;
        if requested_scope(&preflight.command_name) != Some(preflight.requested_scope.as_str()) {
            return Err(reject(
                DomainPackPreflightStatus::Denied,
                "summarization_command_scope_mismatch",
            ));
        }
        if requires_sources(&preflight.command_name) {
            let request = request.ok_or_else(|| {
                reject(
                    DomainPackPreflightStatus::Denied,
                    "summarization_source_request_required",
                )
            })?;
            if !request.is_bounded(32, 4_096) || !request.sources.iter().all(safe_source) {
                return Err(reject(
                    DomainPackPreflightStatus::Denied,
                    "summarization_source_request_invalid",
                ));
            }
        }
        for (allowed, status, reason) in evidence_checks(evidence, &preflight.command_name) {
            if !allowed {
                return Err(reject(status, reason));
            }
        }
        Ok(())
    }

    /// Invoke the provider closure only after all bounded admission checks pass.
    pub fn dispatch_after_preflight<T>(
        &self,
        request: Option<&SummaryRequest>,
        preflight: &DomainPackCommandPreflight,
        evidence: &SummarizationAdmissionEvidence,
        dispatch: impl FnOnce() -> T,
    ) -> Result<T, DomainPackPreflightRejection> {
        self.evaluate(request, preflight, evidence)?;
        Ok(dispatch())
    }
}

fn requested_scope(command: &str) -> Option<&'static str> {
    match command {
        "summarization.plan" | "summarization.validate_request" => Some("summarization.plan"),
        "summarization.summarize" | "summarization.summarize_many" => Some("summarization.run"),
        "summarization.summarize_with_citations" => Some("summarization.citations"),
        "summarization.summarize_conversation" => Some("summarization.conversation"),
        "summarization.compress_context" => Some("summarization.context.compress"),
        "summarization.refine_summary" => Some("summarization.refine"),
        "summarization.compare_summaries" => Some("summarization.compare"),
        "summarization.evaluate_summary" => Some("summarization.evaluate"),
        "summarization.inspect_summary_evidence" => Some("summarization.evidence.read"),
        "summarization.inspect_provider" => Some("summarization.provider.inspect"),
        _ => None,
    }
}

fn requires_sources(command: &str) -> bool {
    matches!(
        command,
        "summarization.plan"
            | "summarization.validate_request"
            | "summarization.summarize"
            | "summarization.summarize_with_citations"
            | "summarization.summarize_many"
            | "summarization.summarize_conversation"
            | "summarization.compress_context"
    )
}

fn safe_source(source: &SummarySource) -> bool {
    matches!(
        source.source_kind.as_str(),
        "document"
            | "retrieval"
            | "citation"
            | "graph"
            | "message"
            | "transcript"
            | "prior_summary"
    ) && bounded(&source.source_ref)
        && bounded(&source.revision)
        && bounded(&source.sensitivity)
}

fn bounded(value: &str) -> bool {
    !value.is_empty() && value.len() <= 128 && !value.contains('\n')
}

fn evidence_checks(
    evidence: &SummarizationAdmissionEvidence,
    command: &str,
) -> Vec<(bool, DomainPackPreflightStatus, &'static str)> {
    let mut checks = vec![
        check(
            evidence.source_handles_accessible,
            DomainPackPreflightStatus::Denied,
            "summarization_source_access_denied",
        ),
        check(
            evidence.source_kinds_supported,
            DomainPackPreflightStatus::Unsupported,
            "summarization_source_kind_unsupported",
        ),
        check(
            evidence.mode_allowed,
            DomainPackPreflightStatus::Denied,
            "summarization_mode_denied",
        ),
        check(
            evidence.output_schema_allowed,
            DomainPackPreflightStatus::Denied,
            "summarization_output_schema_denied",
        ),
        check(
            evidence.target_length_allowed,
            DomainPackPreflightStatus::QuotaExceeded,
            "summarization_target_length_exceeded",
        ),
        check(
            evidence.language_allowed,
            DomainPackPreflightStatus::Unsupported,
            "summarization_language_unsupported",
        ),
        check(
            evidence.evidence_allowed,
            DomainPackPreflightStatus::Denied,
            "summarization_evidence_denied",
        ),
        check(
            evidence.quote_allowed,
            DomainPackPreflightStatus::Denied,
            "summarization_quote_denied",
        ),
        check(
            evidence.freshness_allowed,
            DomainPackPreflightStatus::Denied,
            "summarization_freshness_denied",
        ),
        check(
            evidence.sensitivity_allowed,
            DomainPackPreflightStatus::Denied,
            "summarization_sensitivity_denied",
        ),
        check(
            evidence.compression_retention_allowed,
            DomainPackPreflightStatus::Denied,
            "summarization_retention_denied",
        ),
        check(
            evidence.quality_threshold_met,
            DomainPackPreflightStatus::Denied,
            "summarization_quality_threshold_unmet",
        ),
        check(
            evidence.chunk_limit_available,
            DomainPackPreflightStatus::QuotaExceeded,
            "summarization_chunk_limit_exceeded",
        ),
        check(
            evidence.streaming_available,
            DomainPackPreflightStatus::Unavailable,
            "summarization_streaming_unavailable",
        ),
        check(
            evidence.timeout_available,
            DomainPackPreflightStatus::QuotaExceeded,
            "summarization_timeout_exceeded",
        ),
        check(
            evidence.memory_available,
            DomainPackPreflightStatus::QuotaExceeded,
            "summarization_memory_unavailable",
        ),
        check(
            evidence.storage_available,
            DomainPackPreflightStatus::QuotaExceeded,
            "summarization_storage_unavailable",
        ),
        check(
            evidence.network_allowed,
            DomainPackPreflightStatus::Denied,
            "summarization_network_denied",
        ),
        check(
            evidence.provider_quota_available,
            DomainPackPreflightStatus::QuotaExceeded,
            "summarization_provider_quota_exceeded",
        ),
        check(
            evidence.evaluation_budget_available,
            DomainPackPreflightStatus::QuotaExceeded,
            "summarization_evaluation_budget_exceeded",
        ),
        check(
            evidence.snapshot_capacity_available,
            DomainPackPreflightStatus::QuotaExceeded,
            "summarization_snapshot_capacity_exceeded",
        ),
    ];
    if command == "summarization.summarize_with_citations" {
        checks.push(check(
            evidence.citation_support_available,
            DomainPackPreflightStatus::Unavailable,
            "summarization_citation_support_unavailable",
        ));
    }
    if command == "summarization.compress_context" {
        checks.push(check(
            evidence.compression_support_available,
            DomainPackPreflightStatus::Unavailable,
            "summarization_compression_support_unavailable",
        ));
    }
    if command == "summarization.evaluate_summary" {
        checks.push(check(
            evidence.evaluation_support_available,
            DomainPackPreflightStatus::Unavailable,
            "summarization_evaluation_support_unavailable",
        ));
    }
    checks
}

fn check(
    allowed: bool,
    status: DomainPackPreflightStatus,
    reason: &'static str,
) -> (bool, DomainPackPreflightStatus, &'static str) {
    (allowed, status, reason)
}

fn reject(status: DomainPackPreflightStatus, reason_code: &str) -> DomainPackPreflightRejection {
    DomainPackPreflightRejection {
        status,
        reason_code: reason_code.into(),
    }
}
