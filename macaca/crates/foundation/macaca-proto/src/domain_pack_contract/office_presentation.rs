use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};
use super::office_common::{
    define_office_command_wrappers, office_pack_definition, office_stable_hash,
    OfficeCommandEnvelope, OfficeError, OfficePackDescriptor, OfficePage, OfficeProviderClass,
};

pub const OFFICE_PRESENTATION_PACK_ID: &str = "pack.office.presentation.v1";
pub const OFFICE_PRESENTATION_SERVICE_ID: &str = "service.office.presentation";

pub const OFFICE_PRESENTATION_COMMANDS: &[&str] = &[
    "presentation.inspect_provider",
    "presentation.create_deck_request",
    "presentation.import_deck_request",
    "presentation.open_deck",
    "presentation.list_slides",
    "presentation.inspect_structure",
    "presentation.inspect_slide",
    "presentation.inspect_assets",
    "presentation.inspect_notes",
    "presentation.inspect_reviews",
    "presentation.plan_edit",
    "presentation.edit_request",
    "presentation.plan_export",
    "presentation.export_request",
    "presentation.inspect_events",
    "presentation.get_artifact_handle",
];

const PRESENTATION_PERMISSION_SCOPES: &[&str] = &[
    "presentation.provider.inspect",
    "presentation.deck.create",
    "presentation.deck.import",
    "presentation.deck.open",
    "presentation.slide.read",
    "presentation.structure.read",
    "presentation.asset.read",
    "presentation.notes.read",
    "presentation.review.read",
    "presentation.edit",
    "presentation.export",
    "presentation.events.read",
    "presentation.artifact.read",
];

const DECK_PROVIDER_METADATA: &[(&str, &str)] = &[
    ("slides", "true"),
    ("themes", "true"),
    ("notes", "true"),
    ("export", "true"),
];
const PRESENTATION_MOCK_METADATA: &[(&str, &str)] = &[
    ("slides", "true"),
    ("assets", "true"),
    ("reviews", "true"),
    ("export", "true"),
];
const PRESENTATION_UNAVAILABLE_METADATA: &[(&str, &str)] = &[
    ("slides", "false"),
    ("assets", "false"),
    ("reviews", "false"),
    ("export", "false"),
];

const PRESENTATION_PROVIDER_CLASSES: &[OfficeProviderClass<'_>] = &[
    OfficeProviderClass {
        provider_class: "deck-structure",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: DECK_PROVIDER_METADATA,
    },
    OfficeProviderClass {
        provider_class: "mock",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: PRESENTATION_MOCK_METADATA,
    },
    OfficeProviderClass {
        provider_class: "unavailable",
        availability: DomainPackProviderCapabilityState::Unavailable,
        metadata: PRESENTATION_UNAVAILABLE_METADATA,
    },
];

pub fn office_presentation_pack_definition() -> DomainPackDefinition {
    office_pack_definition(OfficePackDescriptor {
        pack_id: OFFICE_PRESENTATION_PACK_ID,
        child_change_id: "openspec:add-pack-office-presentation",
        docs_slug: "presentation",
        service_id: OFFICE_PRESENTATION_SERVICE_ID,
        commands: OFFICE_PRESENTATION_COMMANDS,
        permission_scopes: PRESENTATION_PERMISSION_SCOPES,
        provider_classes: PRESENTATION_PROVIDER_CLASSES,
        health_probe: "presentation.inspect_provider",
        unavailable_reason: "office_presentation_provider_not_installed",
        replay_schema: "office.presentation.replay.v1",
        data_classification: "office_presentation_metadata",
        retention_policy: "deck_slides_assets_notes_reviews_and_exports_by_reference",
        redaction_policy: "credentials_provider_payloads_private_notes_customer_data_media_and_exports_redacted",
        examples: &[
            "Declare `pack.office.presentation.v1` as optional until a presentation provider is installed.",
            "Use deck, slide, asset, edit-plan, export, and artifact handles instead of raw deck data.",
        ],
        migration_notes: &[
            "Presentations become callable only after an approved presentation service provider registers command schemas.",
            "Provider-native slide trees, media payloads, and export data must stay behind provider adapters.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PresentationScope {
    pub tenant_scope: String,
    pub deck_scope: String,
    pub permission_profile: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PresentationProviderCapability {
    pub provider_class: String,
    pub formats: BTreeSet<String>,
    pub features: BTreeSet<String>,
    pub max_slides: u32,
    pub state: DomainPackProviderCapabilityState,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeckHandle {
    pub deck_id: String,
    pub version_hash: String,
    pub format: String,
    pub scope: PresentationScope,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlideHandle {
    pub slide_id: String,
    pub deck_id: String,
    pub index: u32,
    pub anchor_hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PresentationStructure {
    pub deck_id: String,
    pub slide_refs: Vec<String>,
    pub theme_ref: Option<String>,
    pub master_refs: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlideElement {
    pub element_id: String,
    pub slide_id: String,
    pub element_kind: String,
    pub geometry_hash: String,
    pub content_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PresentationAsset {
    pub asset_id: String,
    pub asset_kind: String,
    pub media_ref: String,
    pub redaction_profile: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PresentationNotes {
    pub slide_id: String,
    pub notes_ref: String,
    pub visibility: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PresentationReview {
    pub review_id: String,
    pub slide_id: String,
    pub body_ref: String,
    pub status: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PresentationEditOperation {
    pub operation_id: String,
    pub operation_kind: String,
    pub slide_id: Option<String>,
    pub payload_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PresentationEditPlan {
    pub plan_id: String,
    pub base_version_hash: String,
    pub operations: Vec<PresentationEditOperation>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PresentationExportPlan {
    pub export_id: String,
    pub target_format: String,
    pub redaction_profile: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PresentationArtifactHandle {
    pub artifact_id: String,
    pub artifact_kind: String,
    pub expires_at_epoch_ms: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PresentationCollaborationEvent {
    pub event_id: String,
    pub deck_id: String,
    pub event_kind: String,
    pub cursor_hash: Option<String>,
}

define_office_command_wrappers!(
    PresentationInspectProviderCommand,
    PresentationCreateDeckRequestCommand,
    PresentationImportDeckRequestCommand,
    PresentationOpenDeckCommand,
    PresentationListSlidesCommand,
    PresentationInspectStructureCommand,
    PresentationInspectSlideCommand,
    PresentationInspectAssetsCommand,
    PresentationInspectNotesCommand,
    PresentationInspectReviewsCommand,
    PresentationPlanEditCommand,
    PresentationEditRequestCommand,
    PresentationPlanExportCommand,
    PresentationExportRequestCommand,
    PresentationInspectEventsCommand,
    PresentationGetArtifactHandleCommand,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PresentationResultStatus {
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
    AssetDenied,
    NotesRedacted,
    ExportDenied,
    WriteDenied,
    Quota,
    Timeout,
    Cancellation,
    ApprovalRequired,
    Failure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PresentationResultEnvelope<T> {
    pub status: PresentationResultStatus,
    pub data: Option<T>,
    pub page: Option<OfficePage<T>>,
    pub error: Option<OfficeError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PresentationDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub descriptor_hash: String,
    pub provider_capability_schema_hash: String,
    pub deck_version_hash: String,
    pub unavailable_schema_hash: String,
}

pub fn office_presentation_descriptor_hashes() -> PresentationDescriptorHashes {
    PresentationDescriptorHashes {
        command_schema_hash: presentation_stable_hash(&OFFICE_PRESENTATION_COMMANDS),
        result_schema_hash: presentation_stable_hash(&PresentationResultStatus::Success),
        descriptor_hash: presentation_stable_hash(&office_presentation_pack_definition()),
        provider_capability_schema_hash: presentation_stable_hash(
            &PresentationProviderCapability {
                provider_class: "mock".into(),
                formats: BTreeSet::from(["pptx".into(), "html".into()]),
                features: BTreeSet::from(["slides".into(), "assets".into(), "export".into()]),
                max_slides: 500,
                state: DomainPackProviderCapabilityState::Preview,
            },
        ),
        deck_version_hash: presentation_stable_hash(&DeckHandle {
            deck_id: "deck".into(),
            version_hash: "v1".into(),
            format: "pptx".into(),
            scope: PresentationScope::default(),
        }),
        unavailable_schema_hash: presentation_stable_hash(&OfficeError {
            code: "unavailable".into(),
            message: "office presentation provider is not installed".into(),
            retryable: false,
            trace_safe_detail: Some("office_presentation_provider_not_installed".into()),
        }),
    }
}

pub fn presentation_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    office_stable_hash(value)
}
