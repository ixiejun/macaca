use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::knowledge_common::{
    define_knowledge_command_wrappers, knowledge_pack_definition, knowledge_stable_hash,
    KnowledgeCommandEnvelope, KnowledgeError, KnowledgePackDescriptor, KnowledgePage,
    KnowledgeProviderClass,
};
use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};

pub const KNOWLEDGE_CITATIONS_PACK_ID: &str = "pack.knowledge.citations.v1";
pub const KNOWLEDGE_CITATIONS_SERVICE_ID: &str = "service.knowledge.citations";

/// Canonical command names described by `pack.knowledge.citations.v1`.
pub const KNOWLEDGE_CITATIONS_COMMANDS: &[&str] = &[
    "citations.create_citation",
    "citations.resolve_identifier",
    "citations.link_source_span",
    "citations.verify_citation",
    "citations.format_citation",
    "citations.format_bibliography",
    "citations.list_citations",
    "citations.update_citation",
    "citations.import_citations",
    "citations.export_citations",
    "citations.inspect_source_anchor",
    "citations.inspect_provider",
];

const CITATION_PERMISSION_SCOPES: &[&str] = &[
    "citation.create",
    "citation.read",
    "citation.update",
    "citation.source.link",
    "citation.resolve",
    "citation.verify",
    "citation.format",
    "citation.import_export",
    "citation.evidence.read",
];

const IDENTIFIER_RESOLVER_METADATA: &[(&str, &str)] = &[
    ("doi", "true"),
    ("datacite", "true"),
    ("crossref", "true"),
    ("formatting", "false"),
];
const STYLE_RENDERER_METADATA: &[(&str, &str)] = &[
    ("csl", "true"),
    ("bibliography", "true"),
    ("verification", "false"),
    ("formatting", "true"),
];
const SOURCE_LINKER_METADATA: &[(&str, &str)] = &[
    ("selectors", "true"),
    ("annotations", "true"),
    ("verification", "true"),
    ("formatting", "false"),
];
const CITATION_MOCK_METADATA: &[(&str, &str)] = &[
    ("doi", "true"),
    ("csl", "true"),
    ("selectors", "true"),
    ("verification", "true"),
];
const CITATION_UNAVAILABLE_METADATA: &[(&str, &str)] = &[
    ("doi", "false"),
    ("csl", "false"),
    ("selectors", "false"),
    ("verification", "false"),
];

const CITATION_PROVIDER_CLASSES: &[KnowledgeProviderClass<'_>] = &[
    KnowledgeProviderClass {
        provider_class: "identifier-resolver",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: IDENTIFIER_RESOLVER_METADATA,
    },
    KnowledgeProviderClass {
        provider_class: "style-renderer",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: STYLE_RENDERER_METADATA,
    },
    KnowledgeProviderClass {
        provider_class: "source-linker",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: SOURCE_LINKER_METADATA,
    },
    KnowledgeProviderClass {
        provider_class: "mock",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: CITATION_MOCK_METADATA,
    },
    KnowledgeProviderClass {
        provider_class: "unavailable",
        availability: DomainPackProviderCapabilityState::Unavailable,
        metadata: CITATION_UNAVAILABLE_METADATA,
    },
];

pub fn knowledge_citations_pack_definition() -> DomainPackDefinition {
    knowledge_pack_definition(KnowledgePackDescriptor {
        pack_id: KNOWLEDGE_CITATIONS_PACK_ID,
        child_change_id: "openspec:add-pack-knowledge-citations",
        docs_slug: "citations",
        service_id: KNOWLEDGE_CITATIONS_SERVICE_ID,
        commands: KNOWLEDGE_CITATIONS_COMMANDS,
        permission_scopes: CITATION_PERMISSION_SCOPES,
        provider_classes: CITATION_PROVIDER_CLASSES,
        health_probe: "citations.inspect_provider",
        unavailable_reason: "knowledge_citations_provider_not_installed",
        replay_schema: "knowledge.citations.replay.v1",
        data_classification: "knowledge_citation_metadata",
        retention_policy: "citation_metadata_and_source_anchors_without_raw_source_text",
        redaction_policy: "credentials_provider_payloads_source_documents_quotes_and_styles_redacted",
        examples: &[
            "Declare `pack.knowledge.citations.v1` as optional until a citation provider is installed.",
            "Use source anchors and quote references instead of raw source text.",
        ],
        migration_notes: &[
            "Citations become callable only after an approved citation service provider registers command schemas.",
            "Provider-native resolver payloads, CSL files, and source documents must stay behind provider adapters.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CitationItem {
    pub citation_id: String,
    pub title_ref: String,
    pub identifiers: Vec<CitationIdentifier>,
    pub contributors: Vec<CitationContributor>,
    pub source_anchor: Option<CitationSourceAnchor>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CitationIdentifier {
    pub scheme: String,
    pub normalized_value: String,
}

impl CitationIdentifier {
    /// Normalize identifier text without consulting provider resolvers.
    pub fn normalize(scheme: impl Into<String>, value: impl AsRef<str>) -> Self {
        Self {
            scheme: scheme.into().trim().to_lowercase(),
            normalized_value: value.as_ref().trim().to_lowercase(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CitationContributor {
    pub name_ref: String,
    pub role: String,
    pub order: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CitationSourceAnchor {
    pub source_ref: String,
    pub selectors: Vec<CitationSelector>,
    pub quote_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CitationSelector {
    pub selector_kind: String,
    pub start_offset: u64,
    pub end_offset: u64,
    pub checksum: Option<String>,
}

impl CitationSelector {
    /// Validate a bounded W3C-style selector range before provider calls.
    pub fn is_bounded(&self, max_span: u64) -> bool {
        self.end_offset >= self.start_offset && self.end_offset - self.start_offset <= max_span
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CitationEvidence {
    pub evidence_id: String,
    pub anchor: CitationSourceAnchor,
    pub verification_status: String,
    pub checked_at_epoch_ms: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BibliographyStyle {
    pub style_id: String,
    pub csl_compatibility: String,
    pub locale: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormattedCitation {
    pub citation_id: String,
    pub formatted_ref: String,
    pub style_id: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CitationVerificationResult {
    pub citation_id: String,
    pub status: String,
    pub issues: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CitationImportResult {
    pub imported_count: u32,
    pub skipped_count: u32,
    pub issue_refs: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CitationExportResult {
    pub export_ref: String,
    pub format: String,
    pub item_count: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CitationProviderCapability {
    pub provider_class: String,
    pub identifier_schemes: BTreeSet<String>,
    pub style_formats: BTreeSet<String>,
    pub selector_support: BTreeSet<String>,
    pub verification_depth: String,
    /// Bounded maximum number of citation items accepted by import or export.
    #[serde(default)]
    pub max_items: u32,
    /// Opaque rate-limit bucket, never a provider-native quota payload.
    #[serde(default)]
    pub rate_limit_bucket: String,
    /// Whether this capability can report health through the service runtime.
    #[serde(default)]
    pub supports_health: bool,
    pub state: DomainPackProviderCapabilityState,
}

define_knowledge_command_wrappers!(
    CitationsCreateCitationCommand,
    CitationsResolveIdentifierCommand,
    CitationsLinkSourceSpanCommand,
    CitationsVerifyCitationCommand,
    CitationsFormatCitationCommand,
    CitationsFormatBibliographyCommand,
    CitationsListCitationsCommand,
    CitationsUpdateCitationCommand,
    CitationsImportCitationsCommand,
    CitationsExportCitationsCommand,
    CitationsInspectSourceAnchorCommand,
    CitationsInspectProviderCommand,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CitationResultStatus {
    Success,
    Page,
    FormattedOutput,
    Verification,
    ImportExport,
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
pub struct CitationResultEnvelope<T> {
    pub status: CitationResultStatus,
    pub data: Option<T>,
    pub page: Option<KnowledgePage<T>>,
    pub error: Option<KnowledgeError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CitationDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub descriptor_hash: String,
    pub provider_capability_schema_hash: String,
    pub unavailable_schema_hash: String,
}

pub fn knowledge_citations_descriptor_hashes() -> CitationDescriptorHashes {
    CitationDescriptorHashes {
        command_schema_hash: citations_stable_hash(&KNOWLEDGE_CITATIONS_COMMANDS),
        result_schema_hash: citations_stable_hash(&CitationResultStatus::Success),
        descriptor_hash: citations_stable_hash(&knowledge_citations_pack_definition()),
        provider_capability_schema_hash: citations_stable_hash(&CitationProviderCapability {
            provider_class: "mock".into(),
            identifier_schemes: BTreeSet::from(["doi".into(), "datacite".into()]),
            style_formats: BTreeSet::from(["csl".into(), "bibtex".into()]),
            selector_support: BTreeSet::from(["text_position".into(), "text_quote".into()]),
            verification_depth: "metadata_and_anchor".into(),
            max_items: 100,
            rate_limit_bucket: "default".into(),
            supports_health: true,
            state: DomainPackProviderCapabilityState::Preview,
        }),
        unavailable_schema_hash: citations_stable_hash(&KnowledgeError {
            code: "unavailable".into(),
            message: "knowledge citations provider is not installed".into(),
            retryable: false,
            trace_safe_detail: Some("knowledge_citations_provider_not_installed".into()),
        }),
    }
}

pub fn citations_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    knowledge_stable_hash(value)
}
