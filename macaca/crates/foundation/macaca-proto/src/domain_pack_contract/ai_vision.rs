use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::ai_common::{
    ai_bounded_token, ai_pack_definition, ai_stable_hash, define_ai_command_wrappers,
    AiPackCommandEnvelope, AiPackDescriptor, AiPackError, AiPackPage, AiProviderClass,
};
use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};

pub const AI_VISION_PACK_ID: &str = "pack.ai.vision.v1";
pub const AI_VISION_SERVICE_ID: &str = "service.ai.vision";

/// Canonical command names described by `pack.ai.vision.v1`.
pub const AI_VISION_COMMANDS: &[&str] = &[
    "vision.analyze_image",
    "vision.analyze_video",
    "vision.ocr",
    "vision.detect_objects",
    "vision.moderate_visual",
    "vision.extract_visual_evidence",
];

const VISION_PERMISSION_SCOPES: &[&str] =
    &["ai.vision.invoke", "ai.vision.ocr", "ai.vision.moderate"];

const MULTIMODAL_MODEL_METADATA: &[(&str, &str)] = &[
    ("image", "true"),
    ("video", "true"),
    ("ocr", "true"),
    ("raw_pixels_in_trace", "false"),
];
const OCR_ENGINE_METADATA: &[(&str, &str)] = &[
    ("ocr", "true"),
    ("layout", "true"),
    ("raw_text_in_trace", "false"),
];
const MODERATION_METADATA: &[(&str, &str)] =
    &[("moderation", "true"), ("policy_decorated", "true")];
const MOCK_METADATA: &[(&str, &str)] =
    &[("deterministic", "true"), ("visual_payloads", "synthetic")];
const UNAVAILABLE_METADATA: &[(&str, &str)] =
    &[("callable", "false"), ("reason", "provider_not_installed")];

const VISION_PROVIDER_CLASSES: &[AiProviderClass<'_>] = &[
    AiProviderClass {
        provider_class: "hosted-model",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: MULTIMODAL_MODEL_METADATA,
    },
    AiProviderClass {
        provider_class: "local-runtime",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: OCR_ENGINE_METADATA,
    },
    AiProviderClass {
        provider_class: "moderation-service",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: MODERATION_METADATA,
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

/// Build the vision pack descriptor without binding image/video model providers.
pub fn ai_vision_pack_definition() -> DomainPackDefinition {
    ai_pack_definition(AiPackDescriptor {
        pack_id: AI_VISION_PACK_ID,
        child_change_id: "openspec:add-pack-ai-vision",
        docs_slug: "vision",
        sdk_slug: "vision",
        service_id: AI_VISION_SERVICE_ID,
        commands: AI_VISION_COMMANDS,
        permission_scopes: VISION_PERMISSION_SCOPES,
        provider_classes: VISION_PROVIDER_CLASSES,
        health_probe: "vision.extract_visual_evidence",
        unavailable_reason: "ai_vision_provider_not_installed",
        replay_schema: "ai.vision.replay.v1",
        data_classification: "ai_vision_reference_metadata",
        retention_policy: "visual_inputs_regions_ocr_objects_moderation_evidence_and_jobs_by_reference",
        redaction_policy: "raw_pixels_frames_ocr_text_faces_biometrics_credentials_and_provider_payloads_redacted",
        timeout_ms: 180_000,
        budget_units: 10,
        examples: &[
            "Declare `pack.ai.vision.v1` as optional until a vision provider is installed.",
            "Use visual handles, region references, evidence refs, and job ids instead of raw image or video bytes.",
        ],
        migration_notes: &[
            "Vision commands become callable only after an approved vision service provider registers matching schemas.",
            "Provider-native visual payloads, OCR bodies, frames, and moderation categories stay behind adapters.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualInput {
    pub input_ref: String,
    pub media_ref: String,
    pub media_kind: String,
    pub content_hash: String,
    pub redaction_class: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualRegion {
    pub region_ref: String,
    pub coordinate_space: String,
    pub x_micros: u32,
    pub y_micros: u32,
    pub width_micros: u32,
    pub height_micros: u32,
}

impl VisualRegion {
    /// Validate normalized regions without inspecting the underlying visual media.
    pub fn is_normalized(&self) -> bool {
        matches!(self.coordinate_space.as_str(), "" | "normalized")
            && ai_bounded_token(&self.region_ref, 128)
            && self.width_micros > 0
            && self.height_micros > 0
            && self.x_micros <= 1_000_000
            && self.y_micros <= 1_000_000
            && self.width_micros <= 1_000_000
            && self.height_micros <= 1_000_000
    }
}

impl OcrTextSpan {
    /// Validate OCR layout ordering with references to redacted text only.
    pub fn spans_are_layout_ordered(spans: &[OcrTextSpan]) -> bool {
        !spans.is_empty()
            && spans.iter().all(|span| {
                ai_bounded_token(&span.span_ref, 128)
                    && ai_bounded_token(&span.page_ref, 128)
                    && ai_bounded_token(&span.block_ref, 128)
                    && ai_bounded_token(&span.line_ref, 128)
                    && ai_bounded_token(&span.redacted_text_ref, 256)
                    && span.region.is_normalized()
            })
            && spans.windows(2).all(|window| {
                let left = &window[0];
                let right = &window[1];
                (
                    left.page_ref.as_str(),
                    left.block_ref.as_str(),
                    left.line_ref.as_str(),
                ) <= (
                    right.page_ref.as_str(),
                    right.block_ref.as_str(),
                    right.line_ref.as_str(),
                )
            })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OcrTextSpan {
    pub span_ref: String,
    pub page_ref: String,
    pub block_ref: String,
    pub line_ref: String,
    pub redacted_text_ref: String,
    pub region: VisualRegion,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DetectedObject {
    pub object_ref: String,
    pub class_ref: String,
    pub confidence_micros: u32,
    pub region: VisualRegion,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualModerationResult {
    pub result_ref: String,
    pub category_refs: Vec<String>,
    pub policy_action: String,
    pub evidence_ref: String,
}

impl VisualModerationResult {
    /// Validate moderation policy evidence before any sensitive visual category leaves adapters.
    pub fn is_policy_safe(&self) -> bool {
        ai_bounded_token(&self.result_ref, 128)
            && !self.category_refs.is_empty()
            && self
                .category_refs
                .iter()
                .all(|category| ai_bounded_token(category, 128))
            && matches!(self.policy_action.as_str(), "allow" | "review" | "block")
            && ai_bounded_token(&self.evidence_ref, 256)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualEvidenceRef {
    pub evidence_ref: String,
    pub source_input_ref: String,
    pub artifact_ref: String,
    pub redaction_profile: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisionJob {
    pub job_ref: String,
    pub state: String,
    pub input_ref: String,
    pub progress_basis_points: u16,
    pub result_ref: Option<String>,
}

impl VisionJob {
    /// Validate async video job state, timeout/cancellation replay, and partial-result refs.
    pub fn is_replayable(&self) -> bool {
        ai_bounded_token(&self.job_ref, 128)
            && matches!(
                self.state.as_str(),
                "queued" | "running" | "partial" | "completed" | "cancelled" | "timed_out"
            )
            && ai_bounded_token(&self.input_ref, 128)
            && self.progress_basis_points <= 10_000
            && self
                .result_ref
                .as_ref()
                .is_none_or(|reference| ai_bounded_token(reference, 256))
    }
}

define_ai_command_wrappers!(
    VisionAnalyzeImageCommand,
    VisionAnalyzeVideoCommand,
    VisionOcrCommand,
    VisionDetectObjectsCommand,
    VisionModerateVisualCommand,
    VisionExtractVisualEvidenceCommand,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VisionResultStatus {
    Success,
    Partial,
    Denied,
    Unavailable,
    Unsupported,
    Conflict,
    QuotaExceeded,
    RegionInvalid,
    ModerationBlocked,
    JobPending,
    JobCancelled,
    ProviderFailure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisionResultEnvelope<T> {
    pub status: VisionResultStatus,
    pub data: Option<T>,
    pub page: Option<AiPackPage<T>>,
    pub error: Option<AiPackError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisionDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub descriptor_hash: String,
    pub provider_capability_hash: String,
    pub input_hash: String,
    pub region_hash: String,
    pub object_hash: String,
    pub job_hash: String,
}

pub fn ai_vision_descriptor_hashes() -> VisionDescriptorHashes {
    let region = VisualRegion {
        region_ref: "region".into(),
        coordinate_space: "normalized".into(),
        x_micros: 1,
        y_micros: 1,
        width_micros: 100,
        height_micros: 100,
    };
    VisionDescriptorHashes {
        command_schema_hash: vision_stable_hash(&AI_VISION_COMMANDS),
        result_schema_hash: vision_stable_hash(&VisionResultStatus::Success),
        descriptor_hash: vision_stable_hash(&ai_vision_pack_definition()),
        provider_capability_hash: vision_stable_hash(&BTreeMap::from([(
            "provider_class".to_string(),
            "mock".to_string(),
        )])),
        input_hash: vision_stable_hash(&VisualInput {
            input_ref: "input".into(),
            media_ref: "media-ref".into(),
            media_kind: "image".into(),
            content_hash: "image-hash".into(),
            redaction_class: "private".into(),
        }),
        region_hash: vision_stable_hash(&region),
        object_hash: vision_stable_hash(&DetectedObject {
            object_ref: "object".into(),
            class_ref: "class-ref".into(),
            confidence_micros: 900_000,
            region: region.clone(),
        }),
        job_hash: vision_stable_hash(&VisionJob {
            job_ref: "job".into(),
            state: "pending".into(),
            input_ref: "input".into(),
            progress_basis_points: 100,
            result_ref: None,
        }),
    }
}

pub fn vision_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    ai_stable_hash(value)
}
