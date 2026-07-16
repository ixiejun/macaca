use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};
use super::office_common::{
    define_office_command_wrappers, office_pack_definition, office_stable_hash,
    OfficeCommandEnvelope, OfficeError, OfficePackDescriptor, OfficePage, OfficeProviderClass,
};

pub const OFFICE_DOCUMENT_PACK_ID: &str = "pack.office.document.v1";
pub const OFFICE_DOCUMENT_SERVICE_ID: &str = "service.office.document";

pub const OFFICE_DOCUMENT_COMMANDS: &[&str] = &[
    "document.inspect_provider",
    "document.create_document_request",
    "document.import_document_request",
    "document.open_document",
    "document.inspect_structure",
    "document.read_range",
    "document.inspect_styles",
    "document.inspect_comments",
    "document.inspect_revisions",
    "document.plan_edit",
    "document.edit_request",
    "document.comment_request",
    "document.redline_request",
    "document.plan_revision_resolution",
    "document.revision_resolution_request",
    "document.plan_export",
    "document.export_request",
    "document.inspect_events",
    "document.get_artifact_handle",
];

const DOCUMENT_PERMISSION_SCOPES: &[&str] = &[
    "document.provider.inspect",
    "document.create",
    "document.import",
    "document.open",
    "document.structure.read",
    "document.range.read",
    "document.style.read",
    "document.comment.read",
    "document.comment.write",
    "document.revision.read",
    "document.revision.write",
    "document.edit",
    "document.export",
    "document.events.read",
    "document.artifact.read",
];

const STRUCTURED_DOCUMENT_METADATA: &[(&str, &str)] = &[
    ("structure", "true"),
    ("ranges", "true"),
    ("comments", "true"),
    ("revisions", "true"),
];
const PACKAGE_DOCUMENT_METADATA: &[(&str, &str)] = &[
    ("formats", "openxml"),
    ("structure", "true"),
    ("collaboration", "false"),
    ("export", "true"),
];
const DOCUMENT_MOCK_METADATA: &[(&str, &str)] = &[
    ("structure", "true"),
    ("ranges", "true"),
    ("comments", "true"),
    ("export", "true"),
];
const DOCUMENT_UNAVAILABLE_METADATA: &[(&str, &str)] = &[
    ("structure", "false"),
    ("ranges", "false"),
    ("comments", "false"),
    ("export", "false"),
];

const DOCUMENT_PROVIDER_CLASSES: &[OfficeProviderClass<'_>] = &[
    OfficeProviderClass {
        provider_class: "structured-document",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: STRUCTURED_DOCUMENT_METADATA,
    },
    OfficeProviderClass {
        provider_class: "package-document",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: PACKAGE_DOCUMENT_METADATA,
    },
    OfficeProviderClass {
        provider_class: "mock",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: DOCUMENT_MOCK_METADATA,
    },
    OfficeProviderClass {
        provider_class: "unavailable",
        availability: DomainPackProviderCapabilityState::Unavailable,
        metadata: DOCUMENT_UNAVAILABLE_METADATA,
    },
];

pub fn office_document_pack_definition() -> DomainPackDefinition {
    office_pack_definition(OfficePackDescriptor {
        pack_id: OFFICE_DOCUMENT_PACK_ID,
        child_change_id: "openspec:add-pack-office-document",
        docs_slug: "document",
        service_id: OFFICE_DOCUMENT_SERVICE_ID,
        commands: OFFICE_DOCUMENT_COMMANDS,
        permission_scopes: DOCUMENT_PERMISSION_SCOPES,
        provider_classes: DOCUMENT_PROVIDER_CLASSES,
        health_probe: "document.inspect_provider",
        unavailable_reason: "office_document_provider_not_installed",
        replay_schema: "office.document.replay.v1",
        data_classification: "office_document_metadata",
        retention_policy: "document_content_comments_revisions_and_exports_by_reference",
        redaction_policy: "credentials_provider_payloads_private_comments_personal_data_and_full_text_redacted",
        examples: &[
            "Declare `pack.office.document.v1` as optional until a document provider is installed.",
            "Use document, range, edit-plan, export, and artifact handles instead of raw document data.",
        ],
        migration_notes: &[
            "Documents become callable only after an approved document service provider registers command schemas.",
            "Provider-native document trees, batch updates, and package internals must stay behind provider adapters.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentScope {
    pub tenant_scope: String,
    pub workspace_ref: String,
    pub permission_profile: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentProviderCapability {
    pub provider_class: String,
    pub formats: BTreeSet<String>,
    pub features: BTreeSet<String>,
    pub max_document_bytes: u64,
    pub state: DomainPackProviderCapabilityState,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentHandle {
    pub document_id: String,
    pub version_hash: String,
    pub format: String,
    pub scope: DocumentScope,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentStructure {
    pub document_id: String,
    pub section_count: u32,
    pub block_refs: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentRange {
    pub range_id: String,
    pub anchor_hash: String,
    pub start_offset: u64,
    pub end_offset: u64,
}

impl DocumentRange {
    pub fn is_bounded(&self, max_span: u64) -> bool {
        self.end_offset >= self.start_offset && self.end_offset - self.start_offset <= max_span
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentParagraph {
    pub paragraph_id: String,
    pub range: DocumentRange,
    pub style_id: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentRun {
    pub run_id: String,
    pub range: DocumentRange,
    pub text_ref: String,
    pub style_id: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentTable {
    pub table_id: String,
    pub row_count: u32,
    pub column_count: u32,
    pub range: DocumentRange,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentStyle {
    pub style_id: String,
    pub style_kind: String,
    pub attributes: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentComment {
    pub comment_id: String,
    pub range: DocumentRange,
    pub body_ref: String,
    pub visibility: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentRevision {
    pub revision_id: String,
    pub range: DocumentRange,
    pub revision_kind: String,
    pub author_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentEditOperation {
    pub operation_id: String,
    pub operation_kind: String,
    pub range: Option<DocumentRange>,
    pub payload_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentEditPlan {
    pub plan_id: String,
    pub base_version_hash: String,
    pub operations: Vec<DocumentEditOperation>,
    pub approval_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentExportPlan {
    pub export_id: String,
    pub target_format: String,
    pub redaction_profile: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentArtifactHandle {
    pub artifact_id: String,
    pub artifact_kind: String,
    pub expires_at_epoch_ms: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentCollaborationEvent {
    pub event_id: String,
    pub document_id: String,
    pub event_kind: String,
    pub cursor_hash: Option<String>,
}

define_office_command_wrappers!(
    DocumentInspectProviderCommand,
    DocumentCreateDocumentRequestCommand,
    DocumentImportDocumentRequestCommand,
    DocumentOpenDocumentCommand,
    DocumentInspectStructureCommand,
    DocumentReadRangeCommand,
    DocumentInspectStylesCommand,
    DocumentInspectCommentsCommand,
    DocumentInspectRevisionsCommand,
    DocumentPlanEditCommand,
    DocumentEditRequestCommand,
    DocumentCommentRequestCommand,
    DocumentRedlineRequestCommand,
    DocumentPlanRevisionResolutionCommand,
    DocumentRevisionResolutionRequestCommand,
    DocumentPlanExportCommand,
    DocumentExportRequestCommand,
    DocumentInspectEventsCommand,
    DocumentGetArtifactHandleCommand,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentResultStatus {
    Success,
    Paged,
    Partial,
    Denied,
    Unavailable,
    Unsupported,
    Conflict,
    StaleVersion,
    SchemaMismatch,
    FormatUnsupported,
    ExportDenied,
    WriteDenied,
    RevisionUnsupported,
    Quota,
    Timeout,
    Cancellation,
    ApprovalRequired,
    Failure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentResultEnvelope<T> {
    pub status: DocumentResultStatus,
    pub data: Option<T>,
    pub page: Option<OfficePage<T>>,
    pub error: Option<OfficeError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub descriptor_hash: String,
    pub provider_capability_schema_hash: String,
    pub document_version_hash: String,
    pub unavailable_schema_hash: String,
}

pub fn office_document_descriptor_hashes() -> DocumentDescriptorHashes {
    DocumentDescriptorHashes {
        command_schema_hash: document_stable_hash(&OFFICE_DOCUMENT_COMMANDS),
        result_schema_hash: document_stable_hash(&DocumentResultStatus::Success),
        descriptor_hash: document_stable_hash(&office_document_pack_definition()),
        provider_capability_schema_hash: document_stable_hash(&DocumentProviderCapability {
            provider_class: "mock".into(),
            formats: BTreeSet::from(["docx".into(), "html".into()]),
            features: BTreeSet::from(["structure".into(), "comments".into(), "export".into()]),
            max_document_bytes: 20_000_000,
            state: DomainPackProviderCapabilityState::Preview,
        }),
        document_version_hash: document_stable_hash(&DocumentHandle {
            document_id: "doc".into(),
            version_hash: "v1".into(),
            format: "docx".into(),
            scope: DocumentScope::default(),
        }),
        unavailable_schema_hash: document_stable_hash(&OfficeError {
            code: "unavailable".into(),
            message: "office document provider is not installed".into(),
            retryable: false,
            trace_safe_detail: Some("office_document_provider_not_installed".into()),
        }),
    }
}

pub fn document_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    office_stable_hash(value)
}
