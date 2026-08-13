//! Provider-neutral admission Specification for knowledge search.
//!
//! The runtime host supplies these already-sanitized decisions before provider
//! dispatch. This preserves the microkernel boundary: rejected requests never
//! expose credentials, documents, raw query tokens, or provider payloads to an
//! index adapter.

use serde::{Deserialize, Serialize};

use super::knowledge_search::{knowledge_search_pack_definition, SearchQuery};
use super::pack_preflight::{
    DomainPackCommandPreflight, DomainPackCommandPreflightSpec, DomainPackPreflightRejection,
    DomainPackPreflightStatus,
};

/// Host-evaluated facts required before a search provider invocation.
///
/// Fields contain decisions and bounded resource facts only. Raw corpus
/// content, provider query DSL, credentials, snippets, and result payloads are
/// intentionally excluded from this trace-safe contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchAdmissionEvidence {
    pub corpus_owned: bool,
    pub acl_trimmed: bool,
    pub source_attribution_available: bool,
    pub query_complexity_allowed: bool,
    pub page_and_cursor_valid: bool,
    pub timeout_available: bool,
    pub facet_cardinality_allowed: bool,
    pub snippet_redacted: bool,
    pub provider_capability_available: bool,
    pub rate_limit_available: bool,
    pub refresh_quota_available: bool,
    pub resource_budget_available: bool,
    pub payload_redacted: bool,
    pub output_bounded: bool,
}

/// Descriptor-driven gate for every `search.*` command.
///
/// The Specification maps command names to declared permissions and checks
/// bounded search evidence. It neither selects a provider nor interprets any
/// application-specific workflow.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchDispatchPreflight {
    inner: DomainPackCommandPreflightSpec,
}

impl SearchDispatchPreflight {
    /// Construct a search gate with host-selected approval requirements.
    pub fn new(approval_required_commands: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            inner: DomainPackCommandPreflightSpec::from_definition(
                &knowledge_search_pack_definition(),
                approval_required_commands,
            ),
        }
    }

    /// Evaluate query shape and host evidence before provider dispatch.
    pub fn evaluate(
        &self,
        query: Option<&SearchQuery>,
        preflight: &DomainPackCommandPreflight,
        evidence: &SearchAdmissionEvidence,
    ) -> Result<(), DomainPackPreflightRejection> {
        self.inner.evaluate(preflight)?;
        if requested_scope(&preflight.command_name) != Some(preflight.requested_scope.as_str()) {
            return Err(reject(
                DomainPackPreflightStatus::Denied,
                "search_command_scope_mismatch",
            ));
        }
        if requires_query(&preflight.command_name) && !query.is_some_and(valid_query) {
            return Err(reject(
                DomainPackPreflightStatus::Denied,
                "search_query_invalid_or_unredacted",
            ));
        }
        for (allowed, status, reason) in checks(evidence, &preflight.command_name) {
            if !allowed {
                return Err(reject(status, reason));
            }
        }
        Ok(())
    }

    /// Invoke a provider closure only after the complete admission succeeds.
    pub fn dispatch_after_preflight<T>(
        &self,
        query: Option<&SearchQuery>,
        preflight: &DomainPackCommandPreflight,
        evidence: &SearchAdmissionEvidence,
        dispatch: impl FnOnce() -> T,
    ) -> Result<T, DomainPackPreflightRejection> {
        self.evaluate(query, preflight, evidence)?;
        Ok(dispatch())
    }
}

fn requested_scope(command: &str) -> Option<&'static str> {
    match command {
        "search.register_corpus" => Some("knowledge.search.corpus.manage"),
        "search.inspect_index" => Some("knowledge.search.index.read"),
        "search.search" | "search.query_diagnostics" => Some("knowledge.search.query"),
        "search.suggest" | "search.autocomplete" => Some("knowledge.search.suggest"),
        "search.facets" => Some("knowledge.search.facets"),
        "search.explain_ranking" => Some("knowledge.search.explain"),
        "search.refresh_index" => Some("knowledge.search.index.refresh"),
        "search.index_stats" => Some("knowledge.search.stats"),
        _ => None,
    }
}

fn requires_query(command: &str) -> bool {
    matches!(
        command,
        "search.search"
            | "search.suggest"
            | "search.autocomplete"
            | "search.facets"
            | "search.explain_ranking"
            | "search.query_diagnostics"
    )
}

fn valid_query(query: &SearchQuery) -> bool {
    query.is_bounded(100, 32)
        && bounded(&query.query_ref)
        && bounded(&query.ast_hash)
        && query.facets.len() <= 16
        && query.sort.len() <= 8
        && query.filters.iter().all(|filter| {
            bounded(&filter.field)
                && bounded(&filter.operator)
                && bounded(&filter.value_ref)
                && !sensitive(&filter.value_ref)
        })
        && query
            .facets
            .iter()
            .all(|facet| bounded(&facet.field) && facet.limit > 0 && facet.limit <= 100)
        && query
            .sort
            .iter()
            .all(|sort| bounded(&sort.field) && matches!(sort.direction.as_str(), "asc" | "desc"))
        && !sensitive(&query.query_ref)
        && !sensitive(&query.ast_hash)
}

fn bounded(value: &str) -> bool {
    !value.is_empty() && value.len() <= 128 && !value.contains('\n')
}

fn sensitive(value: &str) -> bool {
    [
        "credential",
        "provider_payload",
        "raw_document",
        "raw_query",
        "private_corpus",
        "snippet=",
    ]
    .iter()
    .any(|marker| value.contains(marker))
}

fn checks(
    evidence: &SearchAdmissionEvidence,
    command: &str,
) -> Vec<(bool, DomainPackPreflightStatus, &'static str)> {
    let mut checks = vec![
        (
            evidence.corpus_owned,
            DomainPackPreflightStatus::Denied,
            "search_corpus_ownership_denied",
        ),
        (
            evidence.acl_trimmed,
            DomainPackPreflightStatus::Denied,
            "search_acl_trimming_required",
        ),
        (
            evidence.source_attribution_available,
            DomainPackPreflightStatus::Denied,
            "search_source_attribution_required",
        ),
        (
            evidence.query_complexity_allowed,
            DomainPackPreflightStatus::QuotaExceeded,
            "search_query_complexity_exceeded",
        ),
        (
            evidence.page_and_cursor_valid,
            DomainPackPreflightStatus::Denied,
            "search_page_or_cursor_invalid",
        ),
        (
            evidence.timeout_available,
            DomainPackPreflightStatus::QuotaExceeded,
            "search_timeout_exceeded",
        ),
        (
            evidence.facet_cardinality_allowed,
            DomainPackPreflightStatus::QuotaExceeded,
            "search_facet_cardinality_exceeded",
        ),
        (
            evidence.snippet_redacted,
            DomainPackPreflightStatus::Denied,
            "search_snippet_redaction_required",
        ),
        (
            evidence.provider_capability_available,
            DomainPackPreflightStatus::Unsupported,
            "search_provider_capability_unsupported",
        ),
        (
            evidence.rate_limit_available,
            DomainPackPreflightStatus::QuotaExceeded,
            "search_rate_limited",
        ),
        (
            evidence.resource_budget_available,
            DomainPackPreflightStatus::QuotaExceeded,
            "search_resource_budget_exhausted",
        ),
        (
            evidence.payload_redacted,
            DomainPackPreflightStatus::Denied,
            "search_payload_redaction_required",
        ),
        (
            evidence.output_bounded,
            DomainPackPreflightStatus::Denied,
            "search_output_bound_required",
        ),
    ];
    if command == "search.refresh_index" {
        checks.push((
            evidence.refresh_quota_available,
            DomainPackPreflightStatus::QuotaExceeded,
            "search_refresh_quota_exceeded",
        ));
    }
    checks
}

fn reject(status: DomainPackPreflightStatus, reason: &'static str) -> DomainPackPreflightRejection {
    DomainPackPreflightRejection {
        status,
        reason_code: reason.into(),
    }
}
