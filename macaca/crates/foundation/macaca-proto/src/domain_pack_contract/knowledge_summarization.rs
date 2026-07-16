use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::knowledge_common::{
    define_knowledge_command_wrappers, knowledge_pack_definition, knowledge_stable_hash,
    KnowledgeCommandEnvelope, KnowledgeError, KnowledgePackDescriptor, KnowledgePage,
    KnowledgeProviderClass,
};
use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};

pub const KNOWLEDGE_SUMMARIZATION_PACK_ID: &str = "pack.knowledge.summarization.v1";
pub const KNOWLEDGE_SUMMARIZATION_SERVICE_ID: &str = "service.knowledge.summarization";

/// Canonical command names described by `pack.knowledge.summarization.v1`.
pub const KNOWLEDGE_SUMMARIZATION_COMMANDS: &[&str] = &[
    "summarization.plan",
    "summarization.validate_request",
    "summarization.summarize",
    "summarization.summarize_with_citations",
    "summarization.summarize_many",
    "summarization.summarize_conversation",
    "summarization.compress_context",
    "summarization.refine_summary",
    "summarization.compare_summaries",
    "summarization.evaluate_summary",
    "summarization.inspect_summary_evidence",
    "summarization.inspect_provider",
];

const SUMMARY_PERMISSION_SCOPES: &[&str] = &[
    "summarization.plan",
    "summarization.run",
    "summarization.citations",
    "summarization.context.compress",
    "summarization.conversation",
    "summarization.refine",
    "summarization.compare",
    "summarization.evaluate",
    "summarization.evidence.read",
    "summarization.provider.inspect",
];

const EXTRACTIVE_SUMMARY_METADATA: &[(&str, &str)] = &[
    ("extractive", "true"),
    ("abstractive", "false"),
    ("citations", "true"),
    ("evaluation", "false"),
];
const ABSTRACTIVE_SUMMARY_METADATA: &[(&str, &str)] = &[
    ("extractive", "false"),
    ("abstractive", "true"),
    ("citations", "optional"),
    ("evaluation", "true"),
];
const CONTEXT_COMPRESSION_METADATA: &[(&str, &str)] = &[
    ("context_compression", "true"),
    ("conversation", "true"),
    ("streaming", "true"),
    ("evaluation", "true"),
];
const SUMMARY_MOCK_METADATA: &[(&str, &str)] = &[
    ("extractive", "true"),
    ("abstractive", "true"),
    ("context_compression", "true"),
    ("evaluation", "true"),
];
const SUMMARY_UNAVAILABLE_METADATA: &[(&str, &str)] = &[
    ("extractive", "false"),
    ("abstractive", "false"),
    ("context_compression", "false"),
    ("evaluation", "false"),
];

const SUMMARY_PROVIDER_CLASSES: &[KnowledgeProviderClass<'_>] = &[
    KnowledgeProviderClass {
        provider_class: "extractive-summary",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: EXTRACTIVE_SUMMARY_METADATA,
    },
    KnowledgeProviderClass {
        provider_class: "abstractive-summary",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: ABSTRACTIVE_SUMMARY_METADATA,
    },
    KnowledgeProviderClass {
        provider_class: "context-compression",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: CONTEXT_COMPRESSION_METADATA,
    },
    KnowledgeProviderClass {
        provider_class: "mock",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: SUMMARY_MOCK_METADATA,
    },
    KnowledgeProviderClass {
        provider_class: "unavailable",
        availability: DomainPackProviderCapabilityState::Unavailable,
        metadata: SUMMARY_UNAVAILABLE_METADATA,
    },
];

pub fn knowledge_summarization_pack_definition() -> DomainPackDefinition {
    knowledge_pack_definition(KnowledgePackDescriptor {
        pack_id: KNOWLEDGE_SUMMARIZATION_PACK_ID,
        child_change_id: "openspec:add-pack-knowledge-summarization",
        docs_slug: "summarization",
        service_id: KNOWLEDGE_SUMMARIZATION_SERVICE_ID,
        commands: KNOWLEDGE_SUMMARIZATION_COMMANDS,
        permission_scopes: SUMMARY_PERMISSION_SCOPES,
        provider_classes: SUMMARY_PROVIDER_CLASSES,
        health_probe: "summarization.inspect_provider",
        unavailable_reason: "knowledge_summarization_provider_not_installed",
        replay_schema: "knowledge.summarization.replay.v1",
        data_classification: "knowledge_summary_metadata",
        retention_policy: "source_text_prompts_model_outputs_and_private_spans_by_reference",
        redaction_policy: "credentials_provider_payloads_raw_prompts_source_documents_and_model_outputs_redacted",
        examples: &[
            "Declare `pack.knowledge.summarization.v1` as optional until a summary provider is installed.",
            "Use source, summary, evidence, and quality handles instead of raw source text or model outputs.",
        ],
        migration_notes: &[
            "Summarization becomes callable only after an approved summarization provider registers command schemas.",
            "Provider-native prompts, model clients, and output payloads must stay behind provider adapters.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SummarySource {
    pub source_ref: String,
    pub source_kind: String,
    pub revision: String,
    pub sensitivity: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SummaryRequest {
    pub request_id: String,
    pub sources: Vec<SummarySource>,
    pub mode: String,
    pub target_tokens: u32,
    pub language: Option<String>,
}

impl SummaryRequest {
    /// Validate the request shape without reading private source content.
    pub fn is_bounded(&self, max_sources: usize, max_target_tokens: u32) -> bool {
        !self.sources.is_empty()
            && self.sources.len() <= max_sources
            && self.target_tokens > 0
            && self.target_tokens <= max_target_tokens
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SummaryPlan {
    pub plan_id: String,
    pub chunk_refs: Vec<String>,
    pub strategy: String,
    pub estimated_steps: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SummaryOutput {
    pub summary_id: String,
    pub output_ref: String,
    pub version_hash: String,
    pub claims: Vec<SummaryClaim>,
    pub evidence_links: Vec<SummaryEvidenceLink>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SummaryClaim {
    pub claim_id: String,
    pub text_ref: String,
    pub confidence_micros: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SummaryEvidenceLink {
    pub claim_id: String,
    pub source_ref: String,
    pub evidence_ref: String,
    pub citation_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompressionMap {
    pub map_id: String,
    pub source_refs: Vec<String>,
    pub retained_refs: Vec<String>,
    pub dropped_count: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SummaryComparisonReport {
    pub report_id: String,
    pub baseline_summary_ref: String,
    pub candidate_summary_ref: String,
    pub differences_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SummaryQualityReport {
    pub report_id: String,
    pub summary_ref: String,
    pub scores: BTreeMap<String, u32>,
    pub issue_refs: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SummaryProviderCapability {
    pub provider_class: String,
    pub modes: BTreeSet<String>,
    pub source_kinds: BTreeSet<String>,
    #[serde(default)]
    pub languages: BTreeSet<String>,
    pub max_sources: u32,
    pub max_output_tokens: u32,
    pub supports_streaming: bool,
    #[serde(default)]
    pub quota_limited: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnostic_code: Option<String>,
    pub state: DomainPackProviderCapabilityState,
}

define_knowledge_command_wrappers!(
    SummarizationPlanCommand,
    SummarizationValidateRequestCommand,
    SummarizationSummarizeCommand,
    SummarizationSummarizeWithCitationsCommand,
    SummarizationSummarizeManyCommand,
    SummarizationSummarizeConversationCommand,
    SummarizationCompressContextCommand,
    SummarizationRefineSummaryCommand,
    SummarizationCompareSummariesCommand,
    SummarizationEvaluateSummaryCommand,
    SummarizationInspectSummaryEvidenceCommand,
    SummarizationInspectProviderCommand,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SummarizationResultStatus {
    Success,
    Streaming,
    Paged,
    Partial,
    ValidationIssue,
    Denied,
    Unavailable,
    Unsupported,
    Conflict,
    Quota,
    Timeout,
    Cancellation,
    Failure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SummarizationResultEnvelope<T> {
    pub status: SummarizationResultStatus,
    pub data: Option<T>,
    pub page: Option<KnowledgePage<T>>,
    pub error: Option<KnowledgeError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SummarizationDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub descriptor_hash: String,
    pub provider_capability_schema_hash: String,
    pub summary_version_hash: String,
    pub source_inventory_hash: String,
    pub compression_map_hash: String,
    pub evidence_map_hash: String,
    pub unavailable_schema_hash: String,
}

pub fn knowledge_summarization_descriptor_hashes() -> SummarizationDescriptorHashes {
    let source = SummarySource {
        source_ref: "source".into(),
        source_kind: "document".into(),
        revision: "rev1".into(),
        sensitivity: "normal".into(),
    };
    let compression_map = CompressionMap {
        map_id: "map".into(),
        source_refs: vec!["source".into()],
        retained_refs: vec!["chunk".into()],
        dropped_count: 0,
    };
    let evidence = SummaryEvidenceLink {
        claim_id: "claim".into(),
        source_ref: "source".into(),
        evidence_ref: "evidence".into(),
        citation_ref: Some("citation".into()),
    };
    SummarizationDescriptorHashes {
        command_schema_hash: summarization_stable_hash(&KNOWLEDGE_SUMMARIZATION_COMMANDS),
        result_schema_hash: summarization_stable_hash(&SummarizationResultStatus::Success),
        descriptor_hash: summarization_stable_hash(&knowledge_summarization_pack_definition()),
        provider_capability_schema_hash: summarization_stable_hash(&SummaryProviderCapability {
            provider_class: "mock".into(),
            modes: BTreeSet::from(["extractive".into(), "abstractive".into(), "hybrid".into()]),
            source_kinds: BTreeSet::from(["document".into(), "conversation".into()]),
            languages: BTreeSet::from(["und".into()]),
            max_sources: 20,
            max_output_tokens: 4_096,
            supports_streaming: true,
            quota_limited: false,
            diagnostic_code: None,
            state: DomainPackProviderCapabilityState::Preview,
        }),
        summary_version_hash: summarization_stable_hash(&SummaryOutput {
            summary_id: "summary".into(),
            output_ref: "artifact:summary".into(),
            version_hash: "v1".into(),
            claims: Vec::new(),
            evidence_links: Vec::new(),
        }),
        source_inventory_hash: summarization_stable_hash(&vec![source]),
        compression_map_hash: summarization_stable_hash(&compression_map),
        evidence_map_hash: summarization_stable_hash(&evidence),
        unavailable_schema_hash: summarization_stable_hash(&KnowledgeError {
            code: "unavailable".into(),
            message: "knowledge summarization provider is not installed".into(),
            retryable: false,
            trace_safe_detail: Some("knowledge_summarization_provider_not_installed".into()),
        }),
    }
}

pub fn summarization_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    knowledge_stable_hash(value)
}
