use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::ai_common::{
    ai_bounded_token, ai_pack_definition, ai_stable_hash, define_ai_command_wrappers,
    AiPackCommandEnvelope, AiPackDescriptor, AiPackError, AiPackPage, AiProviderClass,
};
use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};

pub const AI_EMBEDDING_PACK_ID: &str = "pack.ai.embedding.v1";
pub const AI_EMBEDDING_SERVICE_ID: &str = "service.ai.embedding";

/// Canonical command names described by `pack.ai.embedding.v1`.
pub const AI_EMBEDDING_COMMANDS: &[&str] = &[
    "embedding.embed_text",
    "embedding.embed_image",
    "embedding.batch_embed",
    "embedding.inspect_vector_schema",
    "embedding.estimate_cost",
];

const EMBEDDING_PERMISSION_SCOPES: &[&str] = &["ai.embedding.invoke", "ai.embedding.batch"];

const VECTOR_MODEL_METADATA: &[(&str, &str)] = &[
    ("text", "true"),
    ("image", "true"),
    ("batch", "true"),
    ("raw_vectors_in_trace", "false"),
];
const LOCAL_RUNTIME_METADATA: &[(&str, &str)] = &[
    ("network_required", "false"),
    ("schema_inspection", "true"),
    ("raw_inputs_in_trace", "false"),
];
const REMOTE_SERVICE_METADATA: &[(&str, &str)] =
    &[("network_required", "policy_bound"), ("batch", "true")];
const MOCK_METADATA: &[(&str, &str)] =
    &[("deterministic", "true"), ("vector_payloads", "synthetic")];
const UNAVAILABLE_METADATA: &[(&str, &str)] =
    &[("callable", "false"), ("reason", "provider_not_installed")];

const EMBEDDING_PROVIDER_CLASSES: &[AiProviderClass<'_>] = &[
    AiProviderClass {
        provider_class: "hosted-model",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: VECTOR_MODEL_METADATA,
    },
    AiProviderClass {
        provider_class: "local-runtime",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: LOCAL_RUNTIME_METADATA,
    },
    AiProviderClass {
        provider_class: "remote-service",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: REMOTE_SERVICE_METADATA,
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

/// Build the embedding pack descriptor without binding a concrete embedding provider.
pub fn ai_embedding_pack_definition() -> DomainPackDefinition {
    ai_pack_definition(AiPackDescriptor {
        pack_id: AI_EMBEDDING_PACK_ID,
        child_change_id: "openspec:add-pack-ai-embedding",
        docs_slug: "embedding",
        sdk_slug: "embedding",
        service_id: AI_EMBEDDING_SERVICE_ID,
        commands: AI_EMBEDDING_COMMANDS,
        permission_scopes: EMBEDDING_PERMISSION_SCOPES,
        provider_classes: EMBEDDING_PROVIDER_CLASSES,
        health_probe: "embedding.inspect_vector_schema",
        unavailable_reason: "ai_embedding_provider_not_installed",
        replay_schema: "ai.embedding.replay.v1",
        data_classification: "ai_embedding_reference_metadata",
        retention_policy: "input_references_vector_schema_batch_ids_usage_and_cost_by_reference",
        redaction_policy: "raw_inputs_vectors_model_names_credentials_and_provider_payloads_redacted",
        timeout_ms: 90_000,
        budget_units: 8,
        examples: &[
            "Declare `pack.ai.embedding.v1` as optional until an embedding provider is installed.",
            "Use input references, vector schema descriptors, and item ids instead of raw content or vectors.",
        ],
        migration_notes: &[
            "Embedding commands become callable only after an approved embedding service provider registers matching schemas.",
            "Concrete vector models, vector payloads, and native provider responses stay behind service adapters.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbeddingInput {
    pub input_ref: String,
    pub modality: String,
    pub content_ref: String,
    pub content_hash: String,
    pub truncation_policy: String,
}

impl EmbeddingInput {
    /// Validate that an embedding input is reference-only and uses a supported modality marker.
    pub fn is_reference_only(&self) -> bool {
        ai_bounded_token(&self.input_ref, 128)
            && matches!(self.modality.as_str(), "text" | "image")
            && ai_bounded_token(&self.content_ref, 256)
            && ai_bounded_token(&self.content_hash, 256)
            && matches!(
                self.truncation_policy.as_str(),
                "none" | "head" | "tail" | "balanced" | "bounded"
            )
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbeddingBatchRequest {
    pub batch_ref: String,
    pub inputs: Vec<EmbeddingInput>,
    pub schema: VectorSchemaDescriptor,
    pub idempotency_key: String,
}

impl EmbeddingBatchRequest {
    /// Validate batch shape while keeping raw text, images, and vectors outside traces.
    pub fn is_bounded(&self, max_items: usize) -> bool {
        !self.inputs.is_empty()
            && self.inputs.len() <= max_items
            && self.inputs.iter().all(EmbeddingInput::is_reference_only)
            && self.schema.is_compatible()
            && ai_bounded_token(&self.idempotency_key, 128)
    }

    /// Check whether a result preserves one output or bounded failure per input item id.
    pub fn result_preserves_item_mapping(&self, result: &EmbeddingBatchResult) -> bool {
        let input_refs = self
            .inputs
            .iter()
            .map(|input| input.input_ref.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        let vector_refs = result
            .vectors
            .iter()
            .map(|vector| vector.item_ref.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        let failed_refs = result
            .failed_item_refs
            .iter()
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>();

        !input_refs.is_empty()
            && vector_refs.len() == result.vectors.len()
            && failed_refs.len() == result.failed_item_refs.len()
            && vector_refs.is_subset(&input_refs)
            && failed_refs.is_subset(&input_refs)
            && vector_refs.is_disjoint(&failed_refs)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbeddingVector {
    pub item_ref: String,
    pub vector_ref: String,
    pub dimension: u32,
    pub numeric_type: String,
    pub normalized: bool,
}

impl EmbeddingVector {
    /// Validate vector metadata against the declared schema without storing vector values.
    pub fn matches_schema(&self, schema: &VectorSchemaDescriptor) -> bool {
        ai_bounded_token(&self.item_ref, 128)
            && ai_bounded_token(&self.vector_ref, 256)
            && self.dimension == schema.dimension
            && self.numeric_type == schema.numeric_type
            && self.normalized == (schema.normalization == "unit")
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbeddingBatchResult {
    pub batch_ref: String,
    pub vectors: Vec<EmbeddingVector>,
    pub failed_item_refs: Vec<String>,
    pub usage: EmbeddingUsage,
}

impl EmbeddingBatchResult {
    /// Validate per-item diagnostics using only bounded item references and counters.
    pub fn diagnostics_are_bounded(&self, request: &EmbeddingBatchRequest) -> bool {
        let expected_count = self.vectors.len() + self.failed_item_refs.len();
        expected_count <= request.inputs.len()
            && self
                .vectors
                .iter()
                .all(|vector| vector.matches_schema(&request.schema))
            && self
                .failed_item_refs
                .iter()
                .all(|reference| ai_bounded_token(reference, 128))
            && self.usage.input_count as usize == request.inputs.len()
            && self.usage.accepted_count as usize == self.vectors.len()
            && self.usage.rejected_count as usize == self.failed_item_refs.len()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VectorSchemaDescriptor {
    pub schema_ref: String,
    pub dimension: u32,
    pub numeric_type: String,
    pub metric: String,
    pub normalization: String,
}

impl VectorSchemaDescriptor {
    /// Validate vector shape and compatibility metadata without exposing vectors.
    pub fn is_compatible(&self) -> bool {
        ai_bounded_token(&self.schema_ref, 128)
            && self.dimension > 0
            && matches!(self.numeric_type.as_str(), "float32" | "float16" | "int8")
            && matches!(self.metric.as_str(), "cosine" | "dot" | "l2")
            && matches!(self.normalization.as_str(), "none" | "unit")
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbeddingUsage {
    pub input_count: u32,
    pub accepted_count: u32,
    pub rejected_count: u32,
    pub cost_micros: u64,
}

define_ai_command_wrappers!(
    EmbeddingEmbedTextCommand,
    EmbeddingEmbedImageCommand,
    EmbeddingBatchEmbedCommand,
    EmbeddingInspectVectorSchemaCommand,
    EmbeddingEstimateCostCommand,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmbeddingResultStatus {
    Success,
    Partial,
    Denied,
    Unavailable,
    Unsupported,
    Conflict,
    QuotaExceeded,
    DimensionMismatch,
    UnsupportedModality,
    BatchTooLarge,
    ProviderFailure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbeddingResultEnvelope<T> {
    pub status: EmbeddingResultStatus,
    pub data: Option<T>,
    pub page: Option<AiPackPage<T>>,
    pub error: Option<AiPackError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbeddingDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub descriptor_hash: String,
    pub provider_capability_hash: String,
    pub input_hash: String,
    pub batch_hash: String,
    pub schema_hash: String,
    pub usage_hash: String,
}

pub fn ai_embedding_descriptor_hashes() -> EmbeddingDescriptorHashes {
    let schema = VectorSchemaDescriptor {
        schema_ref: "schema".into(),
        dimension: 384,
        numeric_type: "float32".into(),
        metric: "cosine".into(),
        normalization: "unit".into(),
    };
    let input = EmbeddingInput {
        input_ref: "input".into(),
        modality: "text".into(),
        content_ref: "content-ref".into(),
        content_hash: "content-hash".into(),
        truncation_policy: "bounded".into(),
    };
    EmbeddingDescriptorHashes {
        command_schema_hash: embedding_stable_hash(&AI_EMBEDDING_COMMANDS),
        result_schema_hash: embedding_stable_hash(&EmbeddingResultStatus::Success),
        descriptor_hash: embedding_stable_hash(&ai_embedding_pack_definition()),
        provider_capability_hash: embedding_stable_hash(&BTreeMap::from([(
            "provider_class".to_string(),
            "mock".to_string(),
        )])),
        input_hash: embedding_stable_hash(&input),
        batch_hash: embedding_stable_hash(&EmbeddingBatchRequest {
            batch_ref: "batch".into(),
            inputs: vec![input],
            schema: schema.clone(),
            idempotency_key: "idem".into(),
        }),
        schema_hash: embedding_stable_hash(&schema),
        usage_hash: embedding_stable_hash(&EmbeddingUsage {
            input_count: 1,
            accepted_count: 1,
            rejected_count: 0,
            cost_micros: 1,
        }),
    }
}

pub fn embedding_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    ai_stable_hash(value)
}
