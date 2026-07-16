use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::media_common::{
    define_media_command_wrappers, media_pack_definition, media_stable_hash, MediaCommandEnvelope,
    MediaError, MediaPackDescriptor, MediaPage, MediaProviderClass,
};
use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};

pub const MEDIA_IMAGE_PACK_ID: &str = "pack.media.image.v1";
pub const MEDIA_IMAGE_SERVICE_ID: &str = "service.media.image";

pub const MEDIA_IMAGE_COMMANDS: &[&str] = &[
    "image.inspect_provider",
    "image.import_image_request",
    "image.open_image",
    "image.inspect_metadata",
    "image.plan_thumbnail",
    "image.thumbnail_request",
    "image.plan_transform",
    "image.transform_request",
    "image.plan_composite",
    "image.composite_request",
    "image.plan_redaction",
    "image.redaction_request",
    "image.plan_generation",
    "image.generation_request",
    "image.plan_edit",
    "image.edit_request",
    "image.inspect_safety",
    "image.plan_export",
    "image.export_request",
    "image.get_artifact_handle",
];

const IMAGE_PERMISSION_SCOPES: &[&str] = &[
    "image.provider.inspect",
    "image.import",
    "image.open",
    "image.metadata.read",
    "image.thumbnail",
    "image.transform",
    "image.composite",
    "image.redaction",
    "image.generate",
    "image.edit",
    "image.safety.read",
    "image.export",
    "image.artifact.read",
];

const IMAGE_RASTER_METADATA: &[(&str, &str)] = &[
    ("metadata", "true"),
    ("transform", "true"),
    ("profiles", "true"),
    ("animation", "limited"),
];
const IMAGE_COMPOSITION_METADATA: &[(&str, &str)] = &[
    ("composite", "true"),
    ("annotation", "true"),
    ("redaction", "true"),
    ("export", "true"),
];
const IMAGE_GENERATION_METADATA: &[(&str, &str)] = &[
    ("generation", "true"),
    ("edit", "true"),
    ("safety", "true"),
    ("provenance", "true"),
];
const IMAGE_MOCK_METADATA: &[(&str, &str)] = &[
    ("metadata", "true"),
    ("transform", "true"),
    ("generation", "false"),
    ("export", "true"),
];
const IMAGE_UNAVAILABLE_METADATA: &[(&str, &str)] = &[
    ("metadata", "false"),
    ("transform", "false"),
    ("generation", "false"),
    ("export", "false"),
];

const IMAGE_PROVIDER_CLASSES: &[MediaProviderClass<'_>] = &[
    MediaProviderClass {
        provider_class: "raster-image",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: IMAGE_RASTER_METADATA,
    },
    MediaProviderClass {
        provider_class: "image-composition",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: IMAGE_COMPOSITION_METADATA,
    },
    MediaProviderClass {
        provider_class: "image-generation",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: IMAGE_GENERATION_METADATA,
    },
    MediaProviderClass {
        provider_class: "mock",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: IMAGE_MOCK_METADATA,
    },
    MediaProviderClass {
        provider_class: "unavailable",
        availability: DomainPackProviderCapabilityState::Unavailable,
        metadata: IMAGE_UNAVAILABLE_METADATA,
    },
];

/// Build the image pack descriptor without binding ImageMagick, libvips, cloud, or model providers.
pub fn media_image_pack_definition() -> DomainPackDefinition {
    media_pack_definition(MediaPackDescriptor {
        pack_id: MEDIA_IMAGE_PACK_ID,
        child_change_id: "openspec:add-pack-media-image",
        docs_slug: "image",
        service_id: MEDIA_IMAGE_SERVICE_ID,
        commands: MEDIA_IMAGE_COMMANDS,
        permission_scopes: IMAGE_PERMISSION_SCOPES,
        provider_classes: IMAGE_PROVIDER_CLASSES,
        health_probe: "image.inspect_provider",
        unavailable_reason: "media_image_provider_not_installed",
        replay_schema: "media.image.replay.v1",
        data_classification: "media_image_metadata",
        retention_policy: "image_handles_metadata_plans_safety_reports_and_artifacts_by_reference",
        redaction_policy: "credentials_raw_prompts_private_images_exif_gps_faces_masks_pixels_and_provider_payloads_redacted",
        examples: &[
            "Declare `pack.media.image.v1` as optional until an image provider is installed.",
            "Use image handles, hashes, plans, safety reports, and artifact handles instead of raw image bytes.",
        ],
        migration_notes: &[
            "Image commands become callable only after an approved media image provider registers matching schemas.",
            "Provider-native pipelines, model controls, masks, pixels, EXIF/GPS payloads, and generated images stay behind adapters.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageScope {
    pub tenant_scope: String,
    pub workspace_ref: String,
    pub permission_profile: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageProviderCapability {
    pub provider_class: String,
    pub formats: BTreeSet<String>,
    pub features: BTreeSet<String>,
    pub max_pixel_count: u64,
    pub state: DomainPackProviderCapabilityState,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageHandle {
    pub image_id: String,
    pub version_hash: String,
    pub format: String,
    pub scope: ImageScope,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageMetadata {
    pub image_id: String,
    pub geometry: ImagePixelGeometry,
    pub color_profile: ImageColorProfile,
    pub metadata_ref: String,
    pub gps_present: bool,
    pub exif_present: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImagePixelGeometry {
    pub width_px: u32,
    pub height_px: u32,
    pub frame_count: u32,
    pub orientation: String,
}

impl ImagePixelGeometry {
    pub fn pixel_count(&self) -> u64 {
        u64::from(self.width_px)
            .saturating_mul(u64::from(self.height_px))
            .saturating_mul(u64::from(self.frame_count.max(1)))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageColorProfile {
    pub profile_class: String,
    pub icc_profile_hash: Option<String>,
    pub alpha: bool,
    pub bit_depth: u16,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageFrame {
    pub frame_index: u32,
    pub frame_hash: String,
    pub duration_ms: Option<u32>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageTransformOperation {
    pub operation_id: String,
    pub operation_kind: String,
    pub parameters: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageCompositeLayer {
    pub layer_id: String,
    pub source_ref: String,
    pub blend_mode: String,
    pub placement_hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageAnnotationOperation {
    pub annotation_id: String,
    pub annotation_kind: String,
    pub target_region_hash: String,
    pub payload_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageRedactionOperation {
    pub redaction_id: String,
    pub region_hash: String,
    pub reason_code: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageGenerationPlan {
    pub plan_id: String,
    pub prompt_ref: String,
    pub safety_profile: String,
    pub output_count: u32,
    pub provenance_required: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageEditPlan {
    pub plan_id: String,
    pub base_version_hash: String,
    pub mask_ref: Option<String>,
    pub operations: Vec<ImageTransformOperation>,
    pub approval_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageSafetyReport {
    pub report_id: String,
    pub image_id: String,
    pub categories: BTreeMap<String, String>,
    pub blocked: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageExportPlan {
    pub export_id: String,
    pub target_format: String,
    pub quality_profile: String,
    pub strip_metadata: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageArtifactHandle {
    pub artifact_id: String,
    pub artifact_kind: String,
    pub expires_at_epoch_ms: u64,
}

define_media_command_wrappers!(
    ImageInspectProviderCommand,
    ImageImportImageRequestCommand,
    ImageOpenImageCommand,
    ImageInspectMetadataCommand,
    ImagePlanThumbnailCommand,
    ImageThumbnailRequestCommand,
    ImagePlanTransformCommand,
    ImageTransformRequestCommand,
    ImagePlanCompositeCommand,
    ImageCompositeRequestCommand,
    ImagePlanRedactionCommand,
    ImageRedactionRequestCommand,
    ImagePlanGenerationCommand,
    ImageGenerationRequestCommand,
    ImagePlanEditCommand,
    ImageEditRequestCommand,
    ImageInspectSafetyCommand,
    ImagePlanExportCommand,
    ImageExportRequestCommand,
    ImageGetArtifactHandleCommand,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageResultStatus {
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
    CodecUnsupported,
    MetadataDenied,
    SafetyDenied,
    PromptDenied,
    GenerationDenied,
    RedactionDenied,
    ExportDenied,
    WriteDenied,
    ArtifactDenied,
    Quota,
    Timeout,
    Cancellation,
    ApprovalRequired,
    Failure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageResultEnvelope<T> {
    pub status: ImageResultStatus,
    pub data: Option<T>,
    pub page: Option<MediaPage<T>>,
    pub error: Option<MediaError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub descriptor_hash: String,
    pub provider_capability_hash: String,
    pub image_format_hash: String,
    pub image_version_hash: String,
    pub geometry_hash: String,
    pub color_profile_hash: String,
    pub transform_plan_hash: String,
    pub composite_plan_hash: String,
    pub redaction_plan_hash: String,
    pub generation_plan_hash: String,
    pub edit_plan_hash: String,
    pub safety_report_hash: String,
    pub export_plan_hash: String,
    pub artifact_handle_hash: String,
    pub event_cursor_hash: String,
    pub redaction_metadata_hash: String,
}

pub fn media_image_descriptor_hashes() -> ImageDescriptorHashes {
    let geometry = ImagePixelGeometry {
        width_px: 640,
        height_px: 480,
        frame_count: 1,
        orientation: "normal".into(),
    };
    let color = ImageColorProfile {
        profile_class: "srgb".into(),
        icc_profile_hash: Some("icc".into()),
        alpha: true,
        bit_depth: 8,
    };
    let transform = ImageTransformOperation {
        operation_id: "resize".into(),
        operation_kind: "resize".into(),
        parameters: BTreeMap::from([("width".into(), "320".into())]),
    };
    ImageDescriptorHashes {
        command_schema_hash: image_stable_hash(&MEDIA_IMAGE_COMMANDS),
        result_schema_hash: image_stable_hash(&ImageResultStatus::Success),
        descriptor_hash: image_stable_hash(&media_image_pack_definition()),
        provider_capability_hash: image_stable_hash(&ImageProviderCapability {
            provider_class: "mock".into(),
            formats: BTreeSet::from(["png".into(), "jpeg".into(), "webp".into()]),
            features: BTreeSet::from(["metadata".into(), "transform".into(), "export".into()]),
            max_pixel_count: 20_000_000,
            state: DomainPackProviderCapabilityState::Preview,
        }),
        image_format_hash: image_stable_hash(&BTreeSet::from(["png", "jpeg", "webp"])),
        image_version_hash: image_stable_hash(&ImageHandle {
            image_id: "image".into(),
            version_hash: "v1".into(),
            format: "png".into(),
            scope: ImageScope::default(),
        }),
        geometry_hash: image_stable_hash(&geometry),
        color_profile_hash: image_stable_hash(&color),
        transform_plan_hash: image_stable_hash(&transform),
        composite_plan_hash: image_stable_hash(&ImageCompositeLayer {
            layer_id: "layer".into(),
            source_ref: "image:layer".into(),
            blend_mode: "normal".into(),
            placement_hash: "placement".into(),
        }),
        redaction_plan_hash: image_stable_hash(&ImageRedactionOperation {
            redaction_id: "redaction".into(),
            region_hash: "region".into(),
            reason_code: "private".into(),
        }),
        generation_plan_hash: image_stable_hash(&ImageGenerationPlan {
            plan_id: "generation".into(),
            prompt_ref: "prompt:ref".into(),
            safety_profile: "strict".into(),
            output_count: 1,
            provenance_required: true,
        }),
        edit_plan_hash: image_stable_hash(&ImageEditPlan {
            plan_id: "edit".into(),
            base_version_hash: "v1".into(),
            mask_ref: Some("mask:ref".into()),
            operations: vec![transform],
            approval_ref: Some("approval".into()),
        }),
        safety_report_hash: image_stable_hash(&ImageSafetyReport {
            report_id: "safety".into(),
            image_id: "image".into(),
            categories: BTreeMap::from([("synthetic".into(), "allow".into())]),
            blocked: false,
        }),
        export_plan_hash: image_stable_hash(&ImageExportPlan {
            export_id: "export".into(),
            target_format: "png".into(),
            quality_profile: "lossless".into(),
            strip_metadata: true,
        }),
        artifact_handle_hash: image_stable_hash(&ImageArtifactHandle {
            artifact_id: "artifact".into(),
            artifact_kind: "preview".into(),
            expires_at_epoch_ms: 1,
        }),
        event_cursor_hash: image_stable_hash(&"cursor:image"),
        redaction_metadata_hash: image_stable_hash(&MediaError {
            code: "unavailable".into(),
            message: "media image provider is not installed".into(),
            retryable: false,
            trace_safe_detail: Some("media_image_provider_not_installed".into()),
        }),
    }
}

pub fn image_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    media_stable_hash(value)
}
