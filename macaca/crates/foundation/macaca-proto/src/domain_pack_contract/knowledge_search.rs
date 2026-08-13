use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::knowledge_common::{
    define_knowledge_command_wrappers, knowledge_pack_definition, knowledge_stable_hash,
    KnowledgeCommandEnvelope, KnowledgeError, KnowledgePackDescriptor, KnowledgePage,
    KnowledgeProviderClass,
};
use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};

pub const KNOWLEDGE_SEARCH_PACK_ID: &str = "pack.knowledge.search.v1";
pub const KNOWLEDGE_SEARCH_SERVICE_ID: &str = "service.knowledge.search";

/// Canonical command names described by `pack.knowledge.search.v1`.
///
/// These are descriptor-owned schema identifiers. Concrete index engines,
/// semantic rankers, web search bridges, mock providers, and unavailable
/// providers remain service-runtime implementations.
pub const KNOWLEDGE_SEARCH_COMMANDS: &[&str] = &[
    "search.register_corpus",
    "search.inspect_index",
    "search.search",
    "search.suggest",
    "search.autocomplete",
    "search.facets",
    "search.explain_ranking",
    "search.refresh_index",
    "search.index_stats",
    "search.query_diagnostics",
];

const SEARCH_PERMISSION_SCOPES: &[&str] = &[
    "knowledge.search.query",
    "knowledge.search.suggest",
    "knowledge.search.facets",
    "knowledge.search.explain",
    "knowledge.search.index.read",
    "knowledge.search.index.refresh",
    "knowledge.search.corpus.manage",
    "knowledge.search.stats",
];

const INDEX_SEARCH_METADATA: &[(&str, &str)] = &[
    ("query_ast", "true"),
    ("facets", "true"),
    ("semantic", "false"),
    ("hybrid", "false"),
];
const SEMANTIC_SEARCH_METADATA: &[(&str, &str)] = &[
    ("query_ast", "true"),
    ("facets", "true"),
    ("semantic", "true"),
    ("hybrid", "true"),
];
const FEDERATED_SEARCH_METADATA: &[(&str, &str)] = &[
    ("query_ast", "limited"),
    ("facets", "limited"),
    ("semantic", "false"),
    ("hybrid", "false"),
];
const SEARCH_MOCK_METADATA: &[(&str, &str)] = &[
    ("query_ast", "true"),
    ("facets", "true"),
    ("semantic", "true"),
    ("hybrid", "true"),
];
const SEARCH_UNAVAILABLE_METADATA: &[(&str, &str)] = &[
    ("query_ast", "false"),
    ("facets", "false"),
    ("semantic", "false"),
    ("hybrid", "false"),
];

const SEARCH_PROVIDER_CLASSES: &[KnowledgeProviderClass<'_>] = &[
    KnowledgeProviderClass {
        provider_class: "index-search",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: INDEX_SEARCH_METADATA,
    },
    KnowledgeProviderClass {
        provider_class: "semantic-search",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: SEMANTIC_SEARCH_METADATA,
    },
    KnowledgeProviderClass {
        provider_class: "federated-search",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: FEDERATED_SEARCH_METADATA,
    },
    KnowledgeProviderClass {
        provider_class: "mock",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: SEARCH_MOCK_METADATA,
    },
    KnowledgeProviderClass {
        provider_class: "unavailable",
        availability: DomainPackProviderCapabilityState::Unavailable,
        metadata: SEARCH_UNAVAILABLE_METADATA,
    },
];

/// Build the descriptor-only catalog entry for search.
pub fn knowledge_search_pack_definition() -> DomainPackDefinition {
    knowledge_pack_definition(KnowledgePackDescriptor {
        pack_id: KNOWLEDGE_SEARCH_PACK_ID,
        child_change_id: "openspec:add-pack-knowledge-search",
        docs_slug: "search",
        service_id: KNOWLEDGE_SEARCH_SERVICE_ID,
        commands: KNOWLEDGE_SEARCH_COMMANDS,
        permission_scopes: SEARCH_PERMISSION_SCOPES,
        provider_classes: SEARCH_PROVIDER_CLASSES,
        health_probe: "search.index_stats",
        unavailable_reason: "knowledge_search_provider_not_installed",
        replay_schema: "knowledge.search.replay.v1",
        data_classification: "knowledge_search_metadata",
        retention_policy: "corpus_content_by_reference_hits_and_scores_bounded",
        redaction_policy: "credentials_provider_payloads_raw_documents_and_query_tokens_redacted",
        examples: &[
            "Declare `pack.knowledge.search.v1` as optional until an index provider is installed.",
            "Use source and snippet references instead of raw indexed documents.",
        ],
        migration_notes: &[
            "Search becomes callable only after an approved search service provider registers command schemas.",
            "Provider-native query DSL, index mappings, and ranking payloads must stay behind provider adapters.",
        ],
    })
}

/// Corpus registration metadata. Content remains in external storage handles.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchCorpus {
    pub corpus_id: String,
    pub namespace: String,
    pub source_kind: String,
    pub acl_policy: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchIndexSchema {
    pub schema_id: String,
    pub version: String,
    pub fields: Vec<SearchField>,
    pub analyzer_profiles: Vec<SearchAnalyzerProfile>,
    pub ranking_profiles: Vec<SearchRankingProfile>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchField {
    pub name: String,
    pub value_kind: String,
    pub searchable: bool,
    pub filterable: bool,
    pub facetable: bool,
    pub sortable: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchAnalyzerProfile {
    pub analyzer_id: String,
    pub language: String,
    pub tokenization: String,
    pub normalization: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchSynonymSet {
    pub set_id: String,
    pub terms: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchRankingProfile {
    pub profile_id: String,
    pub weights: BTreeMap<String, u32>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchQuery {
    pub query_ref: String,
    pub ast_hash: String,
    pub filters: Vec<SearchFilter>,
    pub facets: Vec<SearchFacetRequest>,
    pub sort: Vec<SearchSort>,
    pub page_size: u32,
}

impl SearchQuery {
    /// Validate the bounded query shape before it reaches provider dispatch.
    pub fn is_bounded(&self, max_page_size: u32, max_filters: usize) -> bool {
        !self.query_ref.trim().is_empty()
            && self.page_size > 0
            && self.page_size <= max_page_size
            && self.filters.len() <= max_filters
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchFilter {
    pub field: String,
    pub operator: String,
    pub value_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchFacetRequest {
    pub field: String,
    pub limit: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchSort {
    pub field: String,
    pub direction: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchHit {
    pub document_ref: String,
    pub score_micros: u32,
    pub snippet_ref: Option<String>,
    pub source_attribution: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchCursor {
    pub cursor_hash: String,
    pub expires_at_epoch_ms: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchRankingExplanation {
    pub hit_ref: String,
    pub factors: BTreeMap<String, i32>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchProviderCapability {
    pub provider_class: String,
    pub query_features: BTreeSet<String>,
    pub max_page_size: u32,
    pub supports_semantic: bool,
    pub supports_hybrid: bool,
    /// Maximum provider-neutral explanation depth that callers may request.
    #[serde(default)]
    pub max_explain_depth: u32,
    /// Whether the provider accepts an asynchronous index refresh request.
    #[serde(default)]
    pub supports_refresh: bool,
    /// Opaque rate-limit bucket identifier, never a provider-native quota value.
    #[serde(default)]
    pub rate_limit_bucket: String,
    /// Bounded health capability reported through the canonical service runtime.
    #[serde(default)]
    pub supports_health: bool,
    pub state: DomainPackProviderCapabilityState,
}

define_knowledge_command_wrappers!(
    SearchRegisterCorpusCommand,
    SearchInspectIndexCommand,
    SearchSearchCommand,
    SearchSuggestCommand,
    SearchAutocompleteCommand,
    SearchFacetsCommand,
    SearchExplainRankingCommand,
    SearchRefreshIndexCommand,
    SearchIndexStatsCommand,
    SearchQueryDiagnosticsCommand,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchResultStatus {
    Success,
    Page,
    AsyncHandle,
    Denied,
    Unavailable,
    Unsupported,
    Conflict,
    Quota,
    Timeout,
    Validation,
    ProviderFailure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchResultEnvelope<T> {
    pub status: SearchResultStatus,
    pub data: Option<T>,
    pub page: Option<KnowledgePage<T>>,
    pub error: Option<KnowledgeError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub descriptor_hash: String,
    pub provider_capability_schema_hash: String,
    pub unavailable_schema_hash: String,
}

pub fn knowledge_search_descriptor_hashes() -> SearchDescriptorHashes {
    SearchDescriptorHashes {
        command_schema_hash: search_stable_hash(&KNOWLEDGE_SEARCH_COMMANDS),
        result_schema_hash: search_stable_hash(&SearchResultStatus::Success),
        descriptor_hash: search_stable_hash(&knowledge_search_pack_definition()),
        provider_capability_schema_hash: search_stable_hash(&SearchProviderCapability {
            provider_class: "mock".into(),
            query_features: BTreeSet::from(["query_ast".into(), "facets".into()]),
            max_page_size: 100,
            supports_semantic: true,
            supports_hybrid: true,
            max_explain_depth: 3,
            supports_refresh: true,
            rate_limit_bucket: "default".into(),
            supports_health: true,
            state: DomainPackProviderCapabilityState::Preview,
        }),
        unavailable_schema_hash: search_stable_hash(&KnowledgeError {
            code: "unavailable".into(),
            message: "knowledge search provider is not installed".into(),
            retryable: false,
            trace_safe_detail: Some("knowledge_search_provider_not_installed".into()),
        }),
    }
}

pub fn search_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    knowledge_stable_hash(value)
}
