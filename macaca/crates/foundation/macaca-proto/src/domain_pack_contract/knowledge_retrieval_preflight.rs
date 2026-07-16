//! Provider-neutral admission Specification for knowledge retrieval.
//!
//! The host evaluates ownership, policy, entitlement, resource, and redaction
//! facts before this gate runs. The gate therefore receives only bounded
//! references and booleans, preventing a concrete vector-store adapter from
//! observing rejected queries, credentials, vectors, or private corpus data.

use serde::{Deserialize, Serialize};

use super::knowledge_retrieval::{
    knowledge_retrieval_pack_definition, RetrievalMetadataFilter, RetrievalQuery,
};
use super::pack_preflight::{
    DomainPackCommandPreflight, DomainPackCommandPreflightSpec, DomainPackPreflightRejection,
    DomainPackPreflightStatus,
};

/// Host-evaluated facts required before a retrieval provider invocation.
///
/// All fields are decisions or bounded resource facts. They deliberately omit
/// credentials, vectors, record content, raw filters, prompts, and provider payloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetrievalAdmissionEvidence {
    pub collection_accessible: bool,
    pub secret_reference_valid: bool,
    pub namespace_isolated: bool,
    pub vector_space_compatible: bool,
    pub embedding_model_compatible: bool,
    pub acl_allows: bool,
    pub filters_valid: bool,
    pub top_k_available: bool,
    pub threshold_and_range_valid: bool,
    pub context_window_available: bool,
    pub query_complexity_allowed: bool,
    pub timeout_available: bool,
    pub provider_capability_available: bool,
    pub rate_limit_available: bool,
    pub refresh_quota_available: bool,
    pub resource_budget_available: bool,
    pub payload_redacted: bool,
    pub output_bounded: bool,
}

/// Descriptor-driven gate for every `retrieval.*` command.
///
/// This Specification maps only retrieval commands to retrieval permission
/// scopes. It never selects a backend, interprets an application workflow, or
/// receives provider-native query data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetrievalDispatchPreflight {
    inner: DomainPackCommandPreflightSpec,
}

impl RetrievalDispatchPreflight {
    /// Construct a retrieval gate with host-selected mutation approval commands.
    pub fn new(approval_required_commands: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            inner: DomainPackCommandPreflightSpec::from_definition(
                &knowledge_retrieval_pack_definition(),
                approval_required_commands,
            ),
        }
    }

    /// Evaluate bounded query and host evidence before provider dispatch.
    pub fn evaluate(
        &self,
        query: Option<&RetrievalQuery>,
        preflight: &DomainPackCommandPreflight,
        evidence: &RetrievalAdmissionEvidence,
    ) -> Result<(), DomainPackPreflightRejection> {
        self.inner.evaluate(preflight)?;
        if requested_scope(&preflight.command_name) != Some(preflight.requested_scope.as_str()) {
            return Err(reject(
                DomainPackPreflightStatus::Denied,
                "retrieval_command_scope_mismatch",
            ));
        }
        if requires_query(&preflight.command_name) && !query.is_some_and(valid_query) {
            return Err(reject(
                DomainPackPreflightStatus::Denied,
                "retrieval_query_invalid_or_unredacted",
            ));
        }
        for (allowed, status, reason) in checks(evidence, &preflight.command_name) {
            if !allowed {
                return Err(reject(status, reason));
            }
        }
        Ok(())
    }

    /// Invoke a provider closure only after all admission checks are accepted.
    pub fn dispatch_after_preflight<T>(
        &self,
        query: Option<&RetrievalQuery>,
        preflight: &DomainPackCommandPreflight,
        evidence: &RetrievalAdmissionEvidence,
        dispatch: impl FnOnce() -> T,
    ) -> Result<T, DomainPackPreflightRejection> {
        self.evaluate(query, preflight, evidence)?;
        Ok(dispatch())
    }
}

fn requested_scope(command: &str) -> Option<&'static str> {
    match command {
        "retrieval.register_collection" => Some("retrieval.collection.manage"),
        "retrieval.upsert_records" | "retrieval.delete_records" => Some("retrieval.record.write"),
        "retrieval.retrieve"
        | "retrieval.bulk_retrieve"
        | "retrieval.range_retrieve"
        | "retrieval.query_diagnostics" => Some("retrieval.query"),
        "retrieval.retrieve_by_id" | "retrieval.expand_context" => Some("retrieval.read"),
        "retrieval.package_evidence" => Some("retrieval.evidence"),
        "retrieval.rerank_context" => Some("retrieval.rerank"),
        "retrieval.inspect_collection" | "retrieval.inspect_record" => {
            Some("retrieval.metadata.inspect")
        }
        "retrieval.refresh_collection" => Some("retrieval.refresh"),
        _ => None,
    }
}

fn requires_query(command: &str) -> bool {
    matches!(
        command,
        "retrieval.retrieve"
            | "retrieval.bulk_retrieve"
            | "retrieval.range_retrieve"
            | "retrieval.rerank_context"
    )
}

fn valid_query(query: &RetrievalQuery) -> bool {
    bounded(&query.query_ref)
        && bounded(&query.vector_space_id)
        && query.top_k > 0
        && query.top_k <= 100
        && query.filters.len() <= 32
        && query.filters.iter().all(valid_filter)
        && bounded(&query.fusion.mode)
        && !contains_sensitive_marker(&query.query_ref)
}

fn valid_filter(filter: &RetrievalMetadataFilter) -> bool {
    bounded(&filter.field)
        && bounded(&filter.operator)
        && bounded(&filter.value_ref)
        && !contains_sensitive_marker(&filter.value_ref)
}

fn bounded(value: &str) -> bool {
    !value.is_empty() && value.len() <= 128 && !value.contains('\n')
}

fn contains_sensitive_marker(value: &str) -> bool {
    [
        "credential",
        "raw_vector",
        "raw_document",
        "raw_chunk",
        "prompt",
        "private_corpus",
    ]
    .iter()
    .any(|marker| value.contains(marker))
}

fn checks(
    evidence: &RetrievalAdmissionEvidence,
    command: &str,
) -> Vec<(bool, DomainPackPreflightStatus, &'static str)> {
    let mut result = vec![
        (
            evidence.collection_accessible,
            DomainPackPreflightStatus::Denied,
            "retrieval_collection_access_denied",
        ),
        (
            evidence.secret_reference_valid,
            DomainPackPreflightStatus::Denied,
            "retrieval_secret_reference_invalid",
        ),
        (
            evidence.namespace_isolated,
            DomainPackPreflightStatus::Denied,
            "retrieval_namespace_isolation_denied",
        ),
        (
            evidence.vector_space_compatible,
            DomainPackPreflightStatus::Unsupported,
            "retrieval_vector_space_incompatible",
        ),
        (
            evidence.embedding_model_compatible,
            DomainPackPreflightStatus::Unsupported,
            "retrieval_embedding_model_incompatible",
        ),
        (
            evidence.acl_allows,
            DomainPackPreflightStatus::Denied,
            "retrieval_acl_denied",
        ),
        (
            evidence.filters_valid,
            DomainPackPreflightStatus::Denied,
            "retrieval_filter_invalid",
        ),
        (
            evidence.top_k_available,
            DomainPackPreflightStatus::QuotaExceeded,
            "retrieval_top_k_exceeded",
        ),
        (
            evidence.threshold_and_range_valid,
            DomainPackPreflightStatus::Denied,
            "retrieval_range_invalid",
        ),
        (
            evidence.context_window_available,
            DomainPackPreflightStatus::QuotaExceeded,
            "retrieval_context_window_exceeded",
        ),
        (
            evidence.query_complexity_allowed,
            DomainPackPreflightStatus::QuotaExceeded,
            "retrieval_query_complexity_exceeded",
        ),
        (
            evidence.timeout_available,
            DomainPackPreflightStatus::QuotaExceeded,
            "retrieval_timeout_exceeded",
        ),
        (
            evidence.provider_capability_available,
            DomainPackPreflightStatus::Unsupported,
            "retrieval_provider_capability_unsupported",
        ),
        (
            evidence.rate_limit_available,
            DomainPackPreflightStatus::QuotaExceeded,
            "retrieval_rate_limited",
        ),
        (
            evidence.resource_budget_available,
            DomainPackPreflightStatus::QuotaExceeded,
            "retrieval_resource_budget_exhausted",
        ),
        (
            evidence.payload_redacted,
            DomainPackPreflightStatus::Denied,
            "retrieval_payload_redaction_required",
        ),
        (
            evidence.output_bounded,
            DomainPackPreflightStatus::Denied,
            "retrieval_output_bound_required",
        ),
    ];
    if command == "retrieval.refresh_collection" {
        result.push((
            evidence.refresh_quota_available,
            DomainPackPreflightStatus::QuotaExceeded,
            "retrieval_refresh_quota_exceeded",
        ));
    }
    result
}

fn reject(status: DomainPackPreflightStatus, reason: &'static str) -> DomainPackPreflightRejection {
    DomainPackPreflightRejection {
        status,
        reason_code: reason.into(),
    }
}
