use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::media_common::{
    define_media_command_wrappers, media_pack_definition, media_stable_hash, MediaCommandEnvelope,
    MediaError, MediaPackDescriptor, MediaPage, MediaProviderClass,
};
use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};

pub const MEDIA_RENDERING_PACK_ID: &str = "pack.media.rendering.v1";
pub const MEDIA_RENDERING_SERVICE_ID: &str = "service.media.rendering";

pub const MEDIA_RENDERING_COMMANDS: &[&str] = &[
    "rendering.inspect_provider",
    "rendering.import_source_request",
    "rendering.open_source",
    "rendering.inspect_template",
    "rendering.inspect_scene_graph",
    "rendering.validate_assets",
    "rendering.plan_render",
    "rendering.render_request",
    "rendering.plan_frame",
    "rendering.frame_request",
    "rendering.plan_animation",
    "rendering.animation_request",
    "rendering.plan_preview",
    "rendering.preview_request",
    "rendering.plan_export",
    "rendering.export_request",
    "rendering.inspect_job",
    "rendering.cancel_job",
    "rendering.get_artifact_handle",
];

const RENDERING_PERMISSION_SCOPES: &[&str] = &[
    "rendering.provider.inspect",
    "rendering.source.import",
    "rendering.source.open",
    "rendering.template.read",
    "rendering.scene.read",
    "rendering.asset.validate",
    "rendering.render",
    "rendering.frame",
    "rendering.animation",
    "rendering.preview",
    "rendering.export",
    "rendering.job.read",
    "rendering.job.cancel",
    "rendering.artifact.read",
];

const RENDER_CPU_METADATA: &[(&str, &str)] = &[
    ("raster", "true"),
    ("vector", "true"),
    ("preview", "true"),
    ("deterministic", "true"),
];
const RENDER_GPU_METADATA: &[(&str, &str)] = &[
    ("gpu", "true"),
    ("shader_policy", "true"),
    ("animation", "true"),
    ("resource_limits", "true"),
];
const RENDER_BROWSER_METADATA: &[(&str, &str)] = &[
    ("browser_surface", "true"),
    ("script_policy", "true"),
    ("remote_asset_policy", "true"),
    ("snapshot", "true"),
];
const RENDER_MOCK_METADATA: &[(&str, &str)] = &[
    ("raster", "true"),
    ("preview", "true"),
    ("gpu", "false"),
    ("export", "true"),
];
const RENDER_UNAVAILABLE_METADATA: &[(&str, &str)] = &[
    ("raster", "false"),
    ("preview", "false"),
    ("gpu", "false"),
    ("export", "false"),
];

const RENDER_PROVIDER_CLASSES: &[MediaProviderClass<'_>] = &[
    MediaProviderClass {
        provider_class: "render-cpu",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: RENDER_CPU_METADATA,
    },
    MediaProviderClass {
        provider_class: "render-gpu",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: RENDER_GPU_METADATA,
    },
    MediaProviderClass {
        provider_class: "render-browser",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: RENDER_BROWSER_METADATA,
    },
    MediaProviderClass {
        provider_class: "mock",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: RENDER_MOCK_METADATA,
    },
    MediaProviderClass {
        provider_class: "unavailable",
        availability: DomainPackProviderCapabilityState::Unavailable,
        metadata: RENDER_UNAVAILABLE_METADATA,
    },
];

/// Build the rendering descriptor without binding CPU engines, GPU APIs, browsers, fonts, or stores.
pub fn media_rendering_pack_definition() -> DomainPackDefinition {
    media_pack_definition(MediaPackDescriptor {
        pack_id: MEDIA_RENDERING_PACK_ID,
        child_change_id: "openspec:add-pack-media-rendering",
        docs_slug: "rendering",
        service_id: MEDIA_RENDERING_SERVICE_ID,
        commands: MEDIA_RENDERING_COMMANDS,
        permission_scopes: RENDERING_PERMISSION_SCOPES,
        provider_classes: RENDER_PROVIDER_CLASSES,
        health_probe: "rendering.inspect_provider",
        unavailable_reason: "media_rendering_provider_not_installed",
        replay_schema: "media.rendering.replay.v1",
        data_classification: "media_rendering_metadata",
        retention_policy: "render_sources_scene_summaries_plans_jobs_and_artifacts_by_reference",
        redaction_policy: "credentials_raw_templates_scripts_assets_fonts_scene_graphs_pixels_vectors_exports_and_provider_payloads_redacted",
        examples: &[
            "Declare `pack.media.rendering.v1` as optional until a rendering provider is installed.",
            "Use source handles, scene summaries, viewport profiles, job state, and artifact handles instead of raw pixels.",
        ],
        migration_notes: &[
            "Rendering commands become callable only after an approved rendering provider registers matching schemas.",
            "Provider-native render trees, scripts, shaders, fonts, assets, pixels, vectors, and browser state stay behind adapters.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderingScope {
    pub tenant_scope: String,
    pub workspace_ref: String,
    pub permission_profile: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderingProviderCapability {
    pub provider_class: String,
    pub engine_classes: BTreeSet<String>,
    pub surface_formats: BTreeSet<String>,
    pub features: BTreeSet<String>,
    pub state: DomainPackProviderCapabilityState,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderSourceHandle {
    pub source_id: String,
    pub version_hash: String,
    pub source_kind: String,
    pub scope: RenderingScope,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderTemplateMetadata {
    pub template_id: String,
    pub version_hash: String,
    pub template_kind: String,
    pub metadata_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneGraphSummary {
    pub scene_id: String,
    pub node_count: u32,
    pub layer_count: u32,
    pub asset_refs: Vec<String>,
    pub redaction_profile: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderViewport {
    pub width_px: u32,
    pub height_px: u32,
    pub scale_millis: u32,
}

impl RenderViewport {
    pub fn pixel_budget(&self) -> u64 {
        u64::from(self.width_px).saturating_mul(u64::from(self.height_px))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderSurfaceProfile {
    pub surface_format: String,
    pub color_profile: String,
    pub alpha: bool,
    pub gpu_required: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderAssetHandle {
    pub asset_id: String,
    pub media_type: String,
    pub version_hash: String,
    pub license_profile: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderFontReference {
    pub font_ref: String,
    pub family_hash: String,
    pub license_profile: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderPlan {
    pub plan_id: String,
    pub source_version_hash: String,
    pub viewport: RenderViewport,
    pub surface: RenderSurfaceProfile,
    pub script_policy: String,
    pub network_policy: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderFramePlan {
    pub plan_id: String,
    pub frame_index: u32,
    pub render_plan_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderAnimationPlan {
    pub plan_id: String,
    pub frame_count: u32,
    pub duration_ms: u64,
    pub render_plan_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderPreviewPlan {
    pub plan_id: String,
    pub viewport: RenderViewport,
    pub deterministic: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderExportPlan {
    pub export_id: String,
    pub target_format: String,
    pub redaction_profile: String,
    pub publishable: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderJobStatus {
    pub job_id: String,
    pub state: String,
    pub rendered_frames: u32,
    pub retryable: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderArtifactHandle {
    pub artifact_id: String,
    pub artifact_kind: String,
    pub expires_at_epoch_ms: u64,
}

define_media_command_wrappers!(
    RenderingInspectProviderCommand,
    RenderingImportSourceRequestCommand,
    RenderingOpenSourceCommand,
    RenderingInspectTemplateCommand,
    RenderingInspectSceneGraphCommand,
    RenderingValidateAssetsCommand,
    RenderingPlanRenderCommand,
    RenderingRenderRequestCommand,
    RenderingPlanFrameCommand,
    RenderingFrameRequestCommand,
    RenderingPlanAnimationCommand,
    RenderingAnimationRequestCommand,
    RenderingPlanPreviewCommand,
    RenderingPreviewRequestCommand,
    RenderingPlanExportCommand,
    RenderingExportRequestCommand,
    RenderingInspectJobCommand,
    RenderingCancelJobCommand,
    RenderingGetArtifactHandleCommand,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RenderingResultStatus {
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
    AssetDenied,
    FontDenied,
    ScriptDenied,
    NetworkDenied,
    ShaderDenied,
    GpuUnavailable,
    RenderDenied,
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
pub struct RenderingResultEnvelope<T> {
    pub status: RenderingResultStatus,
    pub data: Option<T>,
    pub page: Option<MediaPage<T>>,
    pub error: Option<MediaError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderingDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub descriptor_hash: String,
    pub provider_capability_hash: String,
    pub source_version_hash: String,
    pub template_metadata_hash: String,
    pub scene_graph_summary_hash: String,
    pub viewport_hash: String,
    pub surface_profile_hash: String,
    pub asset_font_validation_hash: String,
    pub render_plan_hash: String,
    pub frame_plan_hash: String,
    pub animation_plan_hash: String,
    pub preview_plan_hash: String,
    pub export_plan_hash: String,
    pub job_status_hash: String,
    pub artifact_handle_hash: String,
    pub event_cursor_hash: String,
    pub redaction_metadata_hash: String,
}

pub fn media_rendering_descriptor_hashes() -> RenderingDescriptorHashes {
    let viewport = RenderViewport {
        width_px: 800,
        height_px: 600,
        scale_millis: 1_000,
    };
    let surface = RenderSurfaceProfile {
        surface_format: "rgba8".into(),
        color_profile: "srgb".into(),
        alpha: true,
        gpu_required: false,
    };
    let render_plan = RenderPlan {
        plan_id: "render".into(),
        source_version_hash: "v1".into(),
        viewport: viewport.clone(),
        surface: surface.clone(),
        script_policy: "disabled".into(),
        network_policy: "disabled".into(),
    };
    RenderingDescriptorHashes {
        command_schema_hash: rendering_stable_hash(&MEDIA_RENDERING_COMMANDS),
        result_schema_hash: rendering_stable_hash(&RenderingResultStatus::Success),
        descriptor_hash: rendering_stable_hash(&media_rendering_pack_definition()),
        provider_capability_hash: rendering_stable_hash(&RenderingProviderCapability {
            provider_class: "mock".into(),
            engine_classes: BTreeSet::from(["cpu".into()]),
            surface_formats: BTreeSet::from(["rgba8".into(), "png".into()]),
            features: BTreeSet::from(["render".into(), "preview".into(), "export".into()]),
            state: DomainPackProviderCapabilityState::Preview,
        }),
        source_version_hash: rendering_stable_hash(&RenderSourceHandle {
            source_id: "source".into(),
            version_hash: "v1".into(),
            source_kind: "template".into(),
            scope: RenderingScope::default(),
        }),
        template_metadata_hash: rendering_stable_hash(&RenderTemplateMetadata {
            template_id: "template".into(),
            version_hash: "v1".into(),
            template_kind: "scene".into(),
            metadata_ref: "metadata:ref".into(),
        }),
        scene_graph_summary_hash: rendering_stable_hash(&SceneGraphSummary {
            scene_id: "scene".into(),
            node_count: 12,
            layer_count: 3,
            asset_refs: vec!["asset:1".into()],
            redaction_profile: "default".into(),
        }),
        viewport_hash: rendering_stable_hash(&viewport),
        surface_profile_hash: rendering_stable_hash(&surface),
        asset_font_validation_hash: rendering_stable_hash(&BTreeMap::from([
            ("asset", "asset:1"),
            ("font", "font:1"),
        ])),
        render_plan_hash: rendering_stable_hash(&render_plan),
        frame_plan_hash: rendering_stable_hash(&RenderFramePlan {
            plan_id: "frame".into(),
            frame_index: 0,
            render_plan_ref: "render:plan".into(),
        }),
        animation_plan_hash: rendering_stable_hash(&RenderAnimationPlan {
            plan_id: "animation".into(),
            frame_count: 24,
            duration_ms: 1_000,
            render_plan_ref: "render:plan".into(),
        }),
        preview_plan_hash: rendering_stable_hash(&RenderPreviewPlan {
            plan_id: "preview".into(),
            viewport,
            deterministic: true,
        }),
        export_plan_hash: rendering_stable_hash(&RenderExportPlan {
            export_id: "export".into(),
            target_format: "png".into(),
            redaction_profile: "default".into(),
            publishable: false,
        }),
        job_status_hash: rendering_stable_hash(&RenderJobStatus {
            job_id: "job".into(),
            state: "planned".into(),
            rendered_frames: 0,
            retryable: false,
        }),
        artifact_handle_hash: rendering_stable_hash(&RenderArtifactHandle {
            artifact_id: "artifact".into(),
            artifact_kind: "preview".into(),
            expires_at_epoch_ms: 1,
        }),
        event_cursor_hash: rendering_stable_hash(&"cursor:rendering"),
        redaction_metadata_hash: rendering_stable_hash(&MediaError {
            code: "unavailable".into(),
            message: "media rendering provider is not installed".into(),
            retryable: false,
            trace_safe_detail: Some("media_rendering_provider_not_installed".into()),
        }),
    }
}

pub fn rendering_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    media_stable_hash(value)
}
