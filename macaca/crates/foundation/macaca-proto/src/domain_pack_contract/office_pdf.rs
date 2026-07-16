use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};
use super::office_common::{
    define_office_command_wrappers, office_pack_definition, office_stable_hash,
    OfficeCommandEnvelope, OfficeError, OfficePackDescriptor, OfficePage, OfficeProviderClass,
};

pub const OFFICE_PDF_PACK_ID: &str = "pack.office.pdf.v1";
pub const OFFICE_PDF_SERVICE_ID: &str = "service.office.pdf";

pub const OFFICE_PDF_COMMANDS: &[&str] = &[
    "pdf.inspect_provider",
    "pdf.import_document_request",
    "pdf.open_document",
    "pdf.inspect_metadata",
    "pdf.list_pages",
    "pdf.render_page",
    "pdf.extract_text",
    "pdf.extract_structure",
    "pdf.extract_tables",
    "pdf.extract_images",
    "pdf.inspect_forms",
    "pdf.inspect_annotations",
    "pdf.inspect_embedded_files",
    "pdf.inspect_signatures",
    "pdf.plan_edit",
    "pdf.edit_request",
    "pdf.plan_merge_split",
    "pdf.merge_split_request",
    "pdf.plan_export",
    "pdf.export_request",
    "pdf.get_artifact_handle",
];

const PDF_PERMISSION_SCOPES: &[&str] = &[
    "pdf.provider.inspect",
    "pdf.document.import",
    "pdf.document.open",
    "pdf.metadata.read",
    "pdf.page.read",
    "pdf.render",
    "pdf.text.extract",
    "pdf.structure.extract",
    "pdf.table.extract",
    "pdf.image.extract",
    "pdf.form.read",
    "pdf.form.write",
    "pdf.annotation.read",
    "pdf.annotation.write",
    "pdf.embedded_file.read",
    "pdf.signature.read",
    "pdf.document.write",
    "pdf.redaction.write",
    "pdf.merge_split",
    "pdf.export",
    "pdf.artifact.read",
];

const PDF_STRUCTURE_METADATA: &[(&str, &str)] = &[
    ("pages", "true"),
    ("extraction", "true"),
    ("tables", "true"),
    ("forms", "true"),
];
const PDF_RENDER_METADATA: &[(&str, &str)] = &[
    ("render", "true"),
    ("ocr_handoff", "true"),
    ("linearize", "true"),
    ("export", "true"),
];
const PDF_SECURITY_METADATA: &[(&str, &str)] = &[
    ("signatures", "true"),
    ("encryption", "true"),
    ("redaction", "true"),
    ("embedded_files", "true"),
];
const PDF_MOCK_METADATA: &[(&str, &str)] = &[
    ("pages", "true"),
    ("render", "true"),
    ("signatures", "true"),
    ("export", "true"),
];
const PDF_UNAVAILABLE_METADATA: &[(&str, &str)] = &[
    ("pages", "false"),
    ("render", "false"),
    ("signatures", "false"),
    ("export", "false"),
];

const PDF_PROVIDER_CLASSES: &[OfficeProviderClass<'_>] = &[
    OfficeProviderClass {
        provider_class: "pdf-structure",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: PDF_STRUCTURE_METADATA,
    },
    OfficeProviderClass {
        provider_class: "pdf-render",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: PDF_RENDER_METADATA,
    },
    OfficeProviderClass {
        provider_class: "pdf-security",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: PDF_SECURITY_METADATA,
    },
    OfficeProviderClass {
        provider_class: "mock",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: PDF_MOCK_METADATA,
    },
    OfficeProviderClass {
        provider_class: "unavailable",
        availability: DomainPackProviderCapabilityState::Unavailable,
        metadata: PDF_UNAVAILABLE_METADATA,
    },
];

/// Build the PDF descriptor without binding Adobe, PDF.js, PDFium, iText, or local providers.
pub fn office_pdf_pack_definition() -> DomainPackDefinition {
    office_pack_definition(OfficePackDescriptor {
        pack_id: OFFICE_PDF_PACK_ID,
        child_change_id: "openspec:add-pack-office-pdf",
        docs_slug: "pdf",
        service_id: OFFICE_PDF_SERVICE_ID,
        commands: OFFICE_PDF_COMMANDS,
        permission_scopes: PDF_PERMISSION_SCOPES,
        provider_classes: PDF_PROVIDER_CLASSES,
        health_probe: "pdf.inspect_provider",
        unavailable_reason: "office_pdf_provider_not_installed",
        replay_schema: "office.pdf.replay.v1",
        data_classification: "office_pdf_metadata",
        retention_policy: "pdf_pages_forms_annotations_signatures_and_exports_by_reference",
        redaction_policy: "credentials_passwords_keys_certificates_raw_pdf_bytes_extracted_text_and_rendered_pages_redacted",
        examples: &[
            "Declare `pack.office.pdf.v1` as optional until a PDF provider is installed.",
            "Use PDF handles, page anchors, plans, signature references, and artifacts instead of raw PDF bytes.",
        ],
        migration_notes: &[
            "PDF operations become callable only after an approved PDF service provider registers command schemas.",
            "Provider-native page trees, decrypted content, signatures, and rendered images must stay behind adapters.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PdfScope {
    pub tenant_scope: String,
    pub document_scope: String,
    pub permission_profile: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PdfProviderCapability {
    pub provider_class: String,
    pub profiles: BTreeSet<String>,
    pub features: BTreeSet<String>,
    pub max_document_bytes: u64,
    pub state: DomainPackProviderCapabilityState,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PdfDocumentHandle {
    pub document_id: String,
    pub version_hash: String,
    pub profile: String,
    pub encrypted: bool,
    pub scope: PdfScope,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PdfMetadata {
    pub document_id: String,
    pub page_count: u32,
    pub metadata_ref: String,
    pub properties: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PdfPageHandle {
    pub document_id: String,
    pub page_index: u32,
    pub page_anchor_hash: String,
}

impl PdfPageHandle {
    pub fn is_inside_page_count(&self, page_count: u32) -> bool {
        self.page_index < page_count
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PdfRenderPlan {
    pub page: PdfPageHandle,
    pub width_px: u32,
    pub height_px: u32,
    pub redaction_profile: String,
}

impl PdfRenderPlan {
    pub fn pixel_budget(&self) -> u64 {
        u64::from(self.width_px).saturating_mul(u64::from(self.height_px))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PdfExtractionPlan {
    pub document_id: String,
    pub page_refs: Vec<PdfPageHandle>,
    pub include_tables: bool,
    pub include_images: bool,
    pub ocr_handoff_allowed: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PdfTextSpan {
    pub span_id: String,
    pub page: PdfPageHandle,
    pub text_ref: String,
    pub geometry_hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PdfStructureElement {
    pub element_id: String,
    pub element_kind: String,
    pub page: PdfPageHandle,
    pub children_refs: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PdfTable {
    pub table_id: String,
    pub page: PdfPageHandle,
    pub row_count: u32,
    pub column_count: u32,
    pub data_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PdfImage {
    pub image_id: String,
    pub page: PdfPageHandle,
    pub image_ref: String,
    pub redaction_profile: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PdfFormField {
    pub field_id: String,
    pub field_kind: String,
    pub value_ref: Option<String>,
    pub redaction_profile: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PdfAnnotation {
    pub annotation_id: String,
    pub page: PdfPageHandle,
    pub annotation_kind: String,
    pub body_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PdfEmbeddedFile {
    pub file_id: String,
    pub file_name_hash: String,
    pub media_type: String,
    pub retention_policy: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PdfSignatureReference {
    pub signature_id: String,
    pub validation_state: String,
    pub certificate_ref: Option<String>,
    pub signed_range_hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PdfRedactionOperation {
    pub operation_id: String,
    pub page: PdfPageHandle,
    pub region_hash: String,
    pub reason_code: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PdfEditOperation {
    pub operation_id: String,
    pub operation_kind: String,
    pub payload_ref: Option<String>,
    pub redaction: Option<PdfRedactionOperation>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PdfEditPlan {
    pub plan_id: String,
    pub base_version_hash: String,
    pub operations: Vec<PdfEditOperation>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PdfMergeSplitPlan {
    pub plan_id: String,
    pub source_document_refs: Vec<String>,
    pub page_range_hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PdfExportPlan {
    pub export_id: String,
    pub target_format: String,
    pub preserve_signatures: bool,
    pub redaction_profile: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PdfArtifactHandle {
    pub artifact_id: String,
    pub artifact_kind: String,
    pub expires_at_epoch_ms: u64,
}

define_office_command_wrappers!(
    PdfInspectProviderCommand,
    PdfImportDocumentRequestCommand,
    PdfOpenDocumentCommand,
    PdfInspectMetadataCommand,
    PdfListPagesCommand,
    PdfRenderPageCommand,
    PdfExtractTextCommand,
    PdfExtractStructureCommand,
    PdfExtractTablesCommand,
    PdfExtractImagesCommand,
    PdfInspectFormsCommand,
    PdfInspectAnnotationsCommand,
    PdfInspectEmbeddedFilesCommand,
    PdfInspectSignaturesCommand,
    PdfPlanEditCommand,
    PdfEditRequestCommand,
    PdfPlanMergeSplitCommand,
    PdfMergeSplitRequestCommand,
    PdfPlanExportCommand,
    PdfExportRequestCommand,
    PdfGetArtifactHandleCommand,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PdfResultStatus {
    Success,
    Paged,
    Partial,
    Asynchronous,
    Denied,
    Unavailable,
    Unsupported,
    Conflict,
    StaleVersion,
    SchemaMismatch,
    FormatUnsupported,
    EncryptedDocument,
    PasswordRequired,
    SignatureInvalid,
    SignaturePolicyDenied,
    RedactionDenied,
    ExportDenied,
    WriteDenied,
    AttachmentDenied,
    Quota,
    Timeout,
    Cancellation,
    ApprovalRequired,
    Failure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PdfResultEnvelope<T> {
    pub status: PdfResultStatus,
    pub data: Option<T>,
    pub page: Option<OfficePage<T>>,
    pub error: Option<OfficeError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PdfDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub descriptor_hash: String,
    pub provider_capability_schema_hash: String,
    pub document_version_hash: String,
    pub unavailable_schema_hash: String,
}

pub fn office_pdf_descriptor_hashes() -> PdfDescriptorHashes {
    PdfDescriptorHashes {
        command_schema_hash: pdf_stable_hash(&OFFICE_PDF_COMMANDS),
        result_schema_hash: pdf_stable_hash(&PdfResultStatus::Success),
        descriptor_hash: pdf_stable_hash(&office_pdf_pack_definition()),
        provider_capability_schema_hash: pdf_stable_hash(&PdfProviderCapability {
            provider_class: "mock".into(),
            profiles: BTreeSet::from(["pdf".into(), "pdf-a".into()]),
            features: BTreeSet::from(["render".into(), "extract".into(), "signatures".into()]),
            max_document_bytes: 50_000_000,
            state: DomainPackProviderCapabilityState::Preview,
        }),
        document_version_hash: pdf_stable_hash(&PdfDocumentHandle {
            document_id: "pdf".into(),
            version_hash: "v1".into(),
            profile: "pdf".into(),
            encrypted: false,
            scope: PdfScope::default(),
        }),
        unavailable_schema_hash: pdf_stable_hash(&OfficeError {
            code: "unavailable".into(),
            message: "office PDF provider is not installed".into(),
            retryable: false,
            trace_safe_detail: Some("office_pdf_provider_not_installed".into()),
        }),
    }
}

pub fn pdf_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    office_stable_hash(value)
}
