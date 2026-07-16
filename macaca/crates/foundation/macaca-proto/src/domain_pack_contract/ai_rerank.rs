use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::ai_common::{
    ai_bounded_token, ai_pack_definition, ai_stable_hash, define_ai_command_wrappers,
    AiPackCommandEnvelope, AiPackDescriptor, AiPackError, AiPackPage, AiProviderClass,
};
use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};

pub const AI_RERANK_PACK_ID: &str = "pack.ai.rerank.v1";
pub const AI_RERANK_SERVICE_ID: &str = "service.ai.rerank";

/// Canonical command names described by `pack.ai.rerank.v1`.
pub const AI_RERANK_COMMANDS: &[&str] = &[
    "rerank.rerank",
    "rerank.batch_rerank",
    "rerank.explain_scores",
    "rerank.inspect_model",
];

const RERANK_PERMISSION_SCOPES: &[&str] = &["ai.rerank.invoke", "ai.rerank.explain"];

const CROSS_ENCODER_METADATA: &[(&str, &str)] = &[
    ("score_explanation", "optional"),
    ("batch", "true"),
    ("raw_candidates_in_trace", "false"),
];
const VECTOR_RERANK_METADATA: &[(&str, &str)] = &[
    ("score_normalization", "true"),
    ("stable_tie_breaker", "true"),
];
const PLUGIN_METADATA: &[(&str, &str)] = &[
    ("registration", "service_runtime"),
    ("policy_decorated", "true"),
];
const MOCK_METADATA: &[(&str, &str)] =
    &[("deterministic", "true"), ("score_payloads", "synthetic")];
const UNAVAILABLE_METADATA: &[(&str, &str)] =
    &[("callable", "false"), ("reason", "provider_not_installed")];

const RERANK_PROVIDER_CLASSES: &[AiProviderClass<'_>] = &[
    AiProviderClass {
        provider_class: "hosted-model",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: CROSS_ENCODER_METADATA,
    },
    AiProviderClass {
        provider_class: "local-runtime",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: VECTOR_RERANK_METADATA,
    },
    AiProviderClass {
        provider_class: "plugin",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: PLUGIN_METADATA,
    },
    AiProviderClass {
        provider_class: "mock",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: MOCK_METADATA,
    },
    AiProviderClass {
        provider_class: "unavailable",
        availability: DomainPackProviderCapabilityState::Unavailable,
        metadata: UNAVAILABLE_METADATA,
    },
];

/// Build the rerank pack descriptor without binding a concrete ranking provider.
pub fn ai_rerank_pack_definition() -> DomainPackDefinition {
    ai_pack_definition(AiPackDescriptor {
        pack_id: AI_RERANK_PACK_ID,
        child_change_id: "openspec:add-pack-ai-rerank",
        docs_slug: "rerank",
        sdk_slug: "rerank",
        service_id: AI_RERANK_SERVICE_ID,
        commands: AI_RERANK_COMMANDS,
        permission_scopes: RERANK_PERMISSION_SCOPES,
        provider_classes: RERANK_PROVIDER_CLASSES,
        health_probe: "rerank.inspect_model",
        unavailable_reason: "ai_rerank_provider_not_installed",
        replay_schema: "ai.rerank.replay.v1",
        data_classification: "ai_rerank_reference_metadata",
        retention_policy: "query_refs_candidate_refs_scores_explanations_and_eval_metadata_by_reference",
        redaction_policy: "raw_queries_candidates_model_names_credentials_and_provider_payloads_redacted",
        timeout_ms: 60_000,
        budget_units: 6,
        examples: &[
            "Declare `pack.ai.rerank.v1` as optional until a rerank provider is installed.",
            "Use query and candidate references with stable ids instead of raw documents.",
        ],
        migration_notes: &[
            "Rerank commands become callable only after an approved rerank service provider registers matching schemas.",
            "Concrete rankers, raw candidate text, scores payloads, and native responses stay behind adapters.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RerankRequest {
    pub request_ref: String,
    pub query: RerankQuery,
    pub candidates: Vec<RerankCandidate>,
    pub top_n: u32,
    pub score_normalization: String,
}

impl RerankRequest {
    /// Validate request bounds without reading private query or candidate bodies.
    pub fn is_bounded(&self, max_candidates: usize) -> bool {
        ai_bounded_token(&self.request_ref, 128)
            && self.query.is_reference_only()
            && !self.candidates.is_empty()
            && self.candidates.len() <= max_candidates
            && self.top_n > 0
            && self.top_n as usize <= self.candidates.len()
            && matches!(self.score_normalization.as_str(), "none" | "unit")
            && self
                .candidates
                .iter()
                .all(RerankCandidate::is_visible_reference)
            && self.candidate_refs_are_unique()
    }

    /// Ensure duplicate candidate ids cannot create ambiguous rank mappings.
    pub fn candidate_refs_are_unique(&self) -> bool {
        let refs = self
            .candidates
            .iter()
            .map(|candidate| candidate.candidate_ref.as_str())
            .collect::<BTreeSet<_>>();
        refs.len() == self.candidates.len()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RerankQuery {
    pub query_ref: String,
    pub text_ref: String,
    pub query_hash: String,
}

impl RerankQuery {
    /// Validate query metadata without carrying raw query text.
    pub fn is_reference_only(&self) -> bool {
        ai_bounded_token(&self.query_ref, 128)
            && ai_bounded_token(&self.text_ref, 256)
            && ai_bounded_token(&self.query_hash, 256)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RerankCandidate {
    pub candidate_ref: String,
    pub content_ref: String,
    pub content_hash: String,
    pub hidden: bool,
}

impl RerankCandidate {
    /// Validate candidate metadata and reject hidden candidates before provider dispatch.
    pub fn is_visible_reference(&self) -> bool {
        !self.hidden
            && ai_bounded_token(&self.candidate_ref, 128)
            && ai_bounded_token(&self.content_ref, 256)
            && ai_bounded_token(&self.content_hash, 256)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RerankResult {
    pub candidate_ref: String,
    pub rank: u32,
    pub score_micros: u32,
    pub explanation_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RerankExplanation {
    pub explanation_ref: String,
    pub redacted_summary_ref: String,
    pub score_basis_ref: String,
}

impl RerankExplanation {
    /// Ensure explanations use redacted references rather than raw candidate or query content.
    pub fn is_redacted(&self) -> bool {
        ai_bounded_token(&self.explanation_ref, 128)
            && ai_bounded_token(&self.redacted_summary_ref, 256)
            && ai_bounded_token(&self.score_basis_ref, 256)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RerankBatchResult {
    pub batch_ref: String,
    pub query_results: BTreeMap<String, Vec<RerankResult>>,
    pub failed_query_refs: Vec<String>,
}

impl RerankBatchResult {
    /// Preserve per-query result mappings when batch rerank partially fails.
    pub fn preserves_query_mapping(&self, query_refs: &BTreeSet<String>) -> bool {
        let result_refs = self.query_results.keys().cloned().collect::<BTreeSet<_>>();
        let failed_refs = self
            .failed_query_refs
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        !query_refs.is_empty()
            && result_refs.is_subset(query_refs)
            && failed_refs.is_subset(query_refs)
            && result_refs.is_disjoint(&failed_refs)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RerankEvalMetadata {
    pub eval_ref: String,
    pub metric: String,
    pub replay_schema: String,
    pub tie_breaker: String,
}

/// Validate deterministic rank ordering and normalized scores.
pub fn rerank_results_are_deterministic(results: &[RerankResult]) -> bool {
    !results.is_empty()
        && results.windows(2).all(|window| {
            let left = &window[0];
            let right = &window[1];
            left.rank < right.rank
                && left.score_micros <= 1_000_000
                && right.score_micros <= 1_000_000
                && (left.score_micros > right.score_micros
                    || (left.score_micros == right.score_micros
                        && left.candidate_ref <= right.candidate_ref))
        })
        && results.iter().all(|result| {
            ai_bounded_token(&result.candidate_ref, 128)
                && result
                    .explanation_ref
                    .as_ref()
                    .is_none_or(|reference| ai_bounded_token(reference, 256))
        })
}

define_ai_command_wrappers!(
    RerankRerankCommand,
    RerankBatchRerankCommand,
    RerankExplainScoresCommand,
    RerankInspectModelCommand,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RerankResultStatus {
    Success,
    Partial,
    Denied,
    Unavailable,
    Unsupported,
    Conflict,
    QuotaExceeded,
    DuplicateCandidate,
    CandidateLimitExceeded,
    ExplanationUnavailable,
    ProviderFailure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RerankResultEnvelope<T> {
    pub status: RerankResultStatus,
    pub data: Option<T>,
    pub page: Option<AiPackPage<T>>,
    pub error: Option<AiPackError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RerankDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub descriptor_hash: String,
    pub provider_capability_hash: String,
    pub request_hash: String,
    pub result_hash: String,
    pub explanation_hash: String,
    pub eval_hash: String,
}

pub fn ai_rerank_descriptor_hashes() -> RerankDescriptorHashes {
    let candidate = RerankCandidate {
        candidate_ref: "candidate".into(),
        content_ref: "content-ref".into(),
        content_hash: "content-hash".into(),
        hidden: false,
    };
    RerankDescriptorHashes {
        command_schema_hash: rerank_stable_hash(&AI_RERANK_COMMANDS),
        result_schema_hash: rerank_stable_hash(&RerankResultStatus::Success),
        descriptor_hash: rerank_stable_hash(&ai_rerank_pack_definition()),
        provider_capability_hash: rerank_stable_hash(&BTreeMap::from([(
            "provider_class".to_string(),
            "mock".to_string(),
        )])),
        request_hash: rerank_stable_hash(&RerankRequest {
            request_ref: "request".into(),
            query: RerankQuery {
                query_ref: "query".into(),
                text_ref: "query-ref".into(),
                query_hash: "query-hash".into(),
            },
            candidates: vec![candidate],
            top_n: 1,
            score_normalization: "unit".into(),
        }),
        result_hash: rerank_stable_hash(&RerankResult {
            candidate_ref: "candidate".into(),
            rank: 1,
            score_micros: 900_000,
            explanation_ref: Some("explanation".into()),
        }),
        explanation_hash: rerank_stable_hash(&RerankExplanation {
            explanation_ref: "explanation".into(),
            redacted_summary_ref: "summary-ref".into(),
            score_basis_ref: "basis-ref".into(),
        }),
        eval_hash: rerank_stable_hash(&RerankEvalMetadata {
            eval_ref: "eval".into(),
            metric: "ndcg".into(),
            replay_schema: "ai.rerank.replay.v1".into(),
            tie_breaker: "stable_candidate_ref".into(),
        }),
    }
}

pub fn rerank_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    ai_stable_hash(value)
}
