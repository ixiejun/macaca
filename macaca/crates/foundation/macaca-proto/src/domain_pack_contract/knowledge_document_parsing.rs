use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::knowledge_common::{
    define_knowledge_command_wrappers, knowledge_pack_definition, knowledge_stable_hash,
    KnowledgeCommandEnvelope, KnowledgeError, KnowledgePackDescriptor, KnowledgePage,
    KnowledgeProviderClass,
};
use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};

pub const KNOWLEDGE_DOCUMENT_PARSING_PACK_ID: &str = "pack.knowledge.document.parsing.v1";
pub const KNOWLEDGE_DOCUMENT_PARSING_SERVICE_ID: &str = "service.knowledge.document_parsing";

/// Canonical command names described by `pack.knowledge.document.parsing.v1`.
pub const KNOWLEDGE_DOCUMENT_PARSING_COMMANDS: &[&str] = &[
    "document_parsing.detect_format",
    "document_parsing.validate_document",
    "document_parsing.parse_document",
    "document_parsing.start_parse_job",
    "document_parsing.get_parse_job",
    "document_parsing.cancel_parse_job",
    "document_parsing.extract_text",
    "document_parsing.extract_layout",
    "document_parsing.extract_tables",
    "document_parsing.extract_forms",
    "document_parsing.extract_metadata",
    "document_parsing.convert_to_canonical",
    "document_parsing.chunk_document",
    "document_parsing.inspect_parser",
];

const DOCUMENT_PERMISSION_SCOPES: &[&str] = &[
    "document.parse",
    "document.ocr",
    "document.extract.text",
    "document.extract.layout",
    "document.extract.table",
    "document.extract.form",
    "document.extract.metadata",
    "document.extract.embedded",
    "document.convert",
    "document.chunk",
    "document.parser.inspect",
];

const OCR_PARSER_METADATA: &[(&str, &str)] = &[
    ("ocr", "true"),
    ("layout", "true"),
    ("tables", "false"),
    ("forms", "false"),
];
const STRUCTURED_PARSER_METADATA: &[(&str, &str)] = &[
    ("ocr", "true"),
    ("layout", "true"),
    ("tables", "true"),
    ("forms", "true"),
];
const CANONICALIZER_METADATA: &[(&str, &str)] = &[
    ("canonical_conversion", "true"),
    ("chunking", "true"),
    ("async_jobs", "false"),
    ("ocr", "false"),
];
const DOCUMENT_MOCK_METADATA: &[(&str, &str)] = &[
    ("ocr", "true"),
    ("layout", "true"),
    ("tables", "true"),
    ("forms", "true"),
];
const DOCUMENT_UNAVAILABLE_METADATA: &[(&str, &str)] = &[
    ("ocr", "false"),
    ("layout", "false"),
    ("tables", "false"),
    ("forms", "false"),
];

const DOCUMENT_PROVIDER_CLASSES: &[KnowledgeProviderClass<'_>] = &[
    KnowledgeProviderClass {
        provider_class: "ocr-parser",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: OCR_PARSER_METADATA,
    },
    KnowledgeProviderClass {
        provider_class: "structured-parser",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: STRUCTURED_PARSER_METADATA,
    },
    KnowledgeProviderClass {
        provider_class: "canonicalizer",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: CANONICALIZER_METADATA,
    },
    KnowledgeProviderClass {
        provider_class: "mock",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: DOCUMENT_MOCK_METADATA,
    },
    KnowledgeProviderClass {
        provider_class: "unavailable",
        availability: DomainPackProviderCapabilityState::Unavailable,
        metadata: DOCUMENT_UNAVAILABLE_METADATA,
    },
];

pub fn knowledge_document_parsing_pack_definition() -> DomainPackDefinition {
    knowledge_pack_definition(KnowledgePackDescriptor {
        pack_id: KNOWLEDGE_DOCUMENT_PARSING_PACK_ID,
        child_change_id: "openspec:add-pack-knowledge-document-parsing",
        docs_slug: "document-parsing",
        service_id: KNOWLEDGE_DOCUMENT_PARSING_SERVICE_ID,
        commands: KNOWLEDGE_DOCUMENT_PARSING_COMMANDS,
        permission_scopes: DOCUMENT_PERMISSION_SCOPES,
        provider_classes: DOCUMENT_PROVIDER_CLASSES,
        health_probe: "document_parsing.inspect_parser",
        unavailable_reason: "knowledge_document_parsing_provider_not_installed",
        replay_schema: "knowledge.document_parsing.replay.v1",
        data_classification: "knowledge_document_parse_metadata",
        retention_policy: "document_content_images_and_embedded_files_by_reference_only",
        redaction_policy: "credentials_provider_payloads_raw_documents_ocr_images_and_full_text_redacted",
        examples: &[
            "Declare `pack.knowledge.document.parsing.v1` as optional until a parser provider is installed.",
            "Use document, page, element, and chunk handles instead of raw document bytes.",
        ],
        migration_notes: &[
            "Document parsing becomes callable only after an approved parser provider registers command schemas.",
            "Provider-native OCR blocks, model payloads, and raw files must stay behind provider adapters.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentSource {
    pub document_ref: String,
    pub media_type: String,
    pub size_bytes: u64,
    pub malware_scan_state: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParseJob {
    pub job_id: String,
    pub source: DocumentSource,
    pub status: String,
    pub progress_millis: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParserProfile {
    pub profile_id: String,
    pub requested_features: BTreeSet<String>,
    pub ocr_languages: BTreeSet<String>,
    pub output_limit_bytes: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentPage {
    pub page_id: String,
    pub page_number: u32,
    pub width_units: u32,
    pub height_units: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentElement {
    pub element_id: String,
    pub page_id: String,
    pub element_kind: String,
    pub text_span: Option<DocumentTextSpan>,
    pub geometry: DocumentGeometry,
    pub confidence: DocumentConfidence,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentTextSpan {
    pub text_ref: String,
    pub start_offset: u64,
    pub end_offset: u64,
    pub language: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentOcrToken {
    pub token_id: String,
    pub text_ref: String,
    pub geometry: DocumentGeometry,
    pub confidence: DocumentConfidence,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentTable {
    pub table_id: String,
    pub cells: Vec<DocumentTableCell>,
    pub page_id: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentTableCell {
    pub row_index: u32,
    pub column_index: u32,
    pub row_span: u32,
    pub column_span: u32,
    pub text_span: Option<DocumentTextSpan>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentFormField {
    pub field_id: String,
    pub name_ref: String,
    pub value_ref: Option<String>,
    pub confidence: DocumentConfidence,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentEntity {
    pub entity_id: String,
    pub entity_kind: String,
    pub value_ref: String,
    pub confidence: DocumentConfidence,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub title_ref: Option<String>,
    pub author_ref: Option<String>,
    pub page_count: u32,
    pub language: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentEmbeddedResource {
    pub resource_id: String,
    pub media_type: String,
    pub content_ref: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentChunk {
    pub chunk_id: String,
    pub source_element_ids: Vec<String>,
    pub content_ref: String,
    pub token_count: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentGeometry {
    pub page_id: String,
    pub polygon_hash: String,
    pub coordinate_system: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentConfidence {
    pub score_micros: u32,
    pub model_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentParseResult {
    pub result_id: String,
    pub pages: Vec<DocumentPage>,
    pub elements: Vec<DocumentElement>,
    pub metadata: DocumentMetadata,
    pub chunks: Vec<DocumentChunk>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentParserCapability {
    pub provider_class: String,
    pub supported_formats: BTreeSet<String>,
    pub supported_features: BTreeSet<String>,
    pub max_bytes: u64,
    pub max_pages: u32,
    /// Maximum reference-only output represented by a parse result envelope.
    #[serde(default)]
    pub max_output_bytes: u64,
    /// Opaque rate-limit bucket, never a parser-native quota payload.
    #[serde(default)]
    pub rate_limit_bucket: String,
    /// Whether this capability exposes health through the service runtime.
    #[serde(default)]
    pub supports_health: bool,
    pub state: DomainPackProviderCapabilityState,
}

define_knowledge_command_wrappers!(
    DocumentParsingDetectFormatCommand,
    DocumentParsingValidateDocumentCommand,
    DocumentParsingParseDocumentCommand,
    DocumentParsingStartParseJobCommand,
    DocumentParsingGetParseJobCommand,
    DocumentParsingCancelParseJobCommand,
    DocumentParsingExtractTextCommand,
    DocumentParsingExtractLayoutCommand,
    DocumentParsingExtractTablesCommand,
    DocumentParsingExtractFormsCommand,
    DocumentParsingExtractMetadataCommand,
    DocumentParsingConvertToCanonicalCommand,
    DocumentParsingChunkDocumentCommand,
    DocumentParsingInspectParserCommand,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentParsingResultStatus {
    Success,
    AsyncJob,
    PartialResult,
    Page,
    Table,
    Form,
    Denied,
    Unavailable,
    Unsupported,
    Validation,
    Conflict,
    Quota,
    Timeout,
    Canceled,
    ProviderFailure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentParsingResultEnvelope<T> {
    pub status: DocumentParsingResultStatus,
    pub data: Option<T>,
    pub page: Option<KnowledgePage<T>>,
    pub error: Option<KnowledgeError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentParsingDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub descriptor_hash: String,
    pub provider_capability_schema_hash: String,
    pub unavailable_schema_hash: String,
}

pub fn knowledge_document_parsing_descriptor_hashes() -> DocumentParsingDescriptorHashes {
    DocumentParsingDescriptorHashes {
        command_schema_hash: document_parsing_stable_hash(&KNOWLEDGE_DOCUMENT_PARSING_COMMANDS),
        result_schema_hash: document_parsing_stable_hash(&DocumentParsingResultStatus::Success),
        descriptor_hash: document_parsing_stable_hash(&knowledge_document_parsing_pack_definition()),
        provider_capability_schema_hash: document_parsing_stable_hash(&DocumentParserCapability {
            provider_class: "mock".into(),
            supported_formats: BTreeSet::from(["application/pdf".into(), "image/png".into()]),
            supported_features: BTreeSet::from(["ocr".into(), "layout".into(), "tables".into()]),
            max_bytes: 10_000_000,
            max_pages: 100,
            max_output_bytes: 1_000_000,
            rate_limit_bucket: "default".into(),
            supports_health: true,
            state: DomainPackProviderCapabilityState::Preview,
        }),
        unavailable_schema_hash: document_parsing_stable_hash(&KnowledgeError {
            code: "unavailable".into(),
            message: "knowledge document parsing provider is not installed".into(),
            retryable: false,
            trace_safe_detail: Some("knowledge_document_parsing_provider_not_installed".into()),
        }),
    }
}

pub fn document_parsing_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    knowledge_stable_hash(value)
}
