use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::knowledge_common::{
    define_knowledge_command_wrappers, knowledge_pack_definition, knowledge_stable_hash,
    KnowledgeCommandEnvelope, KnowledgeError, KnowledgePackDescriptor, KnowledgePage,
    KnowledgeProviderClass,
};
use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};

pub const KNOWLEDGE_RETRIEVAL_PACK_ID: &str = "pack.knowledge.retrieval.v1";
pub const KNOWLEDGE_RETRIEVAL_SERVICE_ID: &str = "service.knowledge.retrieval";

/// Canonical command names described by `pack.knowledge.retrieval.v1`.
pub const KNOWLEDGE_RETRIEVAL_COMMANDS: &[&str] = &[
    "retrieval.register_collection",
    "retrieval.upsert_records",
    "retrieval.delete_records",
    "retrieval.retrieve",
    "retrieval.bulk_retrieve",
    "retrieval.retrieve_by_id",
    "retrieval.range_retrieve",
    "retrieval.rerank_context",
    "retrieval.expand_context",
    "retrieval.package_evidence",
    "retrieval.inspect_collection",
    "retrieval.inspect_record",
    "retrieval.refresh_collection",
    "retrieval.query_diagnostics",
];

const RETRIEVAL_PERMISSION_SCOPES: &[&str] = &[
    "retrieval.collection.manage",
    "retrieval.record.write",
    "retrieval.query",
    "retrieval.read",
    "retrieval.evidence",
    "retrieval.rerank",
    "retrieval.metadata.inspect",
    "retrieval.refresh",
];

const VECTOR_RETRIEVAL_METADATA: &[(&str, &str)] = &[
    ("dense_vectors", "true"),
    ("sparse_vectors", "false"),
    ("hybrid", "false"),
    ("rerank", "false"),
];
const HYBRID_RETRIEVAL_METADATA: &[(&str, &str)] = &[
    ("dense_vectors", "true"),
    ("sparse_vectors", "true"),
    ("hybrid", "true"),
    ("rerank", "true"),
];
const EVIDENCE_RETRIEVAL_METADATA: &[(&str, &str)] = &[
    ("dense_vectors", "true"),
    ("sparse_vectors", "true"),
    ("hybrid", "true"),
    ("evidence_packaging", "true"),
];
const RETRIEVAL_MOCK_METADATA: &[(&str, &str)] = &[
    ("dense_vectors", "true"),
    ("sparse_vectors", "true"),
    ("hybrid", "true"),
    ("rerank", "true"),
];
const RETRIEVAL_UNAVAILABLE_METADATA: &[(&str, &str)] = &[
    ("dense_vectors", "false"),
    ("sparse_vectors", "false"),
    ("hybrid", "false"),
    ("rerank", "false"),
];

const RETRIEVAL_PROVIDER_CLASSES: &[KnowledgeProviderClass<'_>] = &[
    KnowledgeProviderClass {
        provider_class: "vector-retrieval",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: VECTOR_RETRIEVAL_METADATA,
    },
    KnowledgeProviderClass {
        provider_class: "hybrid-retrieval",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: HYBRID_RETRIEVAL_METADATA,
    },
    KnowledgeProviderClass {
        provider_class: "evidence-retrieval",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: EVIDENCE_RETRIEVAL_METADATA,
    },
    KnowledgeProviderClass {
        provider_class: "mock",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: RETRIEVAL_MOCK_METADATA,
    },
    KnowledgeProviderClass {
        provider_class: "unavailable",
        availability: DomainPackProviderCapabilityState::Unavailable,
        metadata: RETRIEVAL_UNAVAILABLE_METADATA,
    },
];

pub fn knowledge_retrieval_pack_definition() -> DomainPackDefinition {
    knowledge_pack_definition(KnowledgePackDescriptor {
        pack_id: KNOWLEDGE_RETRIEVAL_PACK_ID,
        child_change_id: "openspec:add-pack-knowledge-retrieval",
        docs_slug: "retrieval",
        service_id: KNOWLEDGE_RETRIEVAL_SERVICE_ID,
        commands: KNOWLEDGE_RETRIEVAL_COMMANDS,
        permission_scopes: RETRIEVAL_PERMISSION_SCOPES,
        provider_classes: RETRIEVAL_PROVIDER_CLASSES,
        health_probe: "retrieval.inspect_collection",
        unavailable_reason: "knowledge_retrieval_provider_not_installed",
        replay_schema: "knowledge.retrieval.replay.v1",
        data_classification: "knowledge_retrieval_metadata",
        retention_policy: "records_chunks_vectors_and_private_corpus_content_by_reference",
        redaction_policy: "credentials_provider_payloads_raw_vectors_documents_and_prompts_redacted",
        examples: &[
            "Declare `pack.knowledge.retrieval.v1` as optional until a retrieval provider is installed.",
            "Use chunk, vector, and evidence references instead of raw corpus payloads.",
        ],
        migration_notes: &[
            "Retrieval becomes callable only after an approved retrieval service provider registers command schemas.",
            "Provider-native vector-store payloads, embeddings, and filters must stay behind provider adapters.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetrievalCollection {
    pub collection_id: String,
    pub namespace: RetrievalNamespace,
    pub vector_spaces: Vec<RetrievalVectorSpace>,
    pub acl_policy: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetrievalNamespace {
    pub tenant_scope: String,
    pub namespace_id: String,
    pub partition_id: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetrievalRecord {
    pub record_id: String,
    pub chunk_refs: Vec<String>,
    pub metadata: BTreeMap<String, String>,
    pub revision: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetrievalChunk {
    pub chunk_id: String,
    pub content_ref: String,
    pub token_start: u32,
    pub token_end: u32,
    pub redaction_policy: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetrievalVectorSpace {
    pub vector_space_id: String,
    pub dimensions: u32,
    pub metric: String,
    pub embedding_model_ref: String,
}

impl RetrievalVectorSpace {
    /// Check vector compatibility without exposing raw vector values.
    pub fn is_compatible_with(&self, other: &Self) -> bool {
        self.dimensions == other.dimensions
            && self.metric == other.metric
            && self.embedding_model_ref == other.embedding_model_ref
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetrievalQuery {
    pub query_ref: String,
    pub vector_space_id: String,
    pub filters: Vec<RetrievalMetadataFilter>,
    pub fusion: RetrievalFusionStrategy,
    pub top_k: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetrievalMetadataFilter {
    pub field: String,
    pub operator: String,
    pub value_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetrievalFusionStrategy {
    pub mode: String,
    pub weights: BTreeMap<String, u32>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetrievalCandidate {
    pub record_id: String,
    pub chunk_id: String,
    pub normalized_score_micros: u32,
    pub evidence_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetrievalEvidenceBundle {
    pub bundle_id: String,
    pub candidate_refs: Vec<String>,
    pub citation_refs: Vec<String>,
    pub redaction_policy: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetrievalCursor {
    pub cursor_hash: String,
    pub collection_id: String,
    pub expires_at_epoch_ms: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetrievalFreshness {
    pub indexed_at_epoch_ms: u64,
    pub source_revision: String,
    pub refresh_required: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetrievalProviderCapability {
    pub provider_class: String,
    pub vector_features: BTreeSet<String>,
    #[serde(default)]
    pub namespace_features: BTreeSet<String>,
    #[serde(default)]
    pub query_features: BTreeSet<String>,
    pub max_top_k: u32,
    #[serde(default)]
    pub max_filters: u32,
    pub supports_rerank: bool,
    pub supports_evidence: bool,
    #[serde(default)]
    pub rate_limited: bool,
    #[serde(default)]
    pub consistency_mode: String,
    pub state: DomainPackProviderCapabilityState,
}

define_knowledge_command_wrappers!(
    RetrievalRegisterCollectionCommand,
    RetrievalUpsertRecordsCommand,
    RetrievalDeleteRecordsCommand,
    RetrievalRetrieveCommand,
    RetrievalBulkRetrieveCommand,
    RetrievalRetrieveByIdCommand,
    RetrievalRangeRetrieveCommand,
    RetrievalRerankContextCommand,
    RetrievalExpandContextCommand,
    RetrievalPackageEvidenceCommand,
    RetrievalInspectCollectionCommand,
    RetrievalInspectRecordCommand,
    RetrievalRefreshCollectionCommand,
    RetrievalQueryDiagnosticsCommand,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetrievalResultStatus {
    Success,
    Page,
    AsyncHandle,
    EvidenceBundle,
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
pub struct RetrievalResultEnvelope<T> {
    pub status: RetrievalResultStatus,
    pub data: Option<T>,
    pub page: Option<KnowledgePage<T>>,
    pub error: Option<KnowledgeError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetrievalDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub descriptor_hash: String,
    pub provider_capability_schema_hash: String,
    pub unavailable_schema_hash: String,
}

pub fn knowledge_retrieval_descriptor_hashes() -> RetrievalDescriptorHashes {
    RetrievalDescriptorHashes {
        command_schema_hash: retrieval_stable_hash(&KNOWLEDGE_RETRIEVAL_COMMANDS),
        result_schema_hash: retrieval_stable_hash(&RetrievalResultStatus::Success),
        descriptor_hash: retrieval_stable_hash(&knowledge_retrieval_pack_definition()),
        provider_capability_schema_hash: retrieval_stable_hash(&RetrievalProviderCapability {
            provider_class: "mock".into(),
            vector_features: BTreeSet::from(["dense_vectors".into(), "hybrid".into()]),
            namespace_features: BTreeSet::from(["namespace".into()]),
            query_features: BTreeSet::from(["metadata_filter".into()]),
            max_top_k: 50,
            max_filters: 8,
            supports_rerank: true,
            supports_evidence: true,
            rate_limited: false,
            consistency_mode: "bounded_eventual".into(),
            state: DomainPackProviderCapabilityState::Preview,
        }),
        unavailable_schema_hash: retrieval_stable_hash(&KnowledgeError {
            code: "unavailable".into(),
            message: "knowledge retrieval provider is not installed".into(),
            retryable: false,
            trace_safe_detail: Some("knowledge_retrieval_provider_not_installed".into()),
        }),
    }
}

pub fn retrieval_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    knowledge_stable_hash(value)
}
