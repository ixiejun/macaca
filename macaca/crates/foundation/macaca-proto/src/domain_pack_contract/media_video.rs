use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::media_common::{
    define_media_command_wrappers, media_pack_definition, media_stable_hash, MediaCommandEnvelope,
    MediaError, MediaPackDescriptor, MediaPage, MediaProviderClass,
};
use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};

pub const MEDIA_VIDEO_PACK_ID: &str = "pack.media.video.v1";
pub const MEDIA_VIDEO_SERVICE_ID: &str = "service.media.video";

pub const MEDIA_VIDEO_COMMANDS: &[&str] = &[
    "video.inspect_provider",
    "video.import_video_request",
    "video.open_video",
    "video.inspect_metadata",
    "video.inspect_tracks",
    "video.plan_thumbnail",
    "video.thumbnail_request",
    "video.plan_transcode",
    "video.transcode_request",
    "video.plan_segment",
    "video.segment_request",
    "video.plan_render",
    "video.render_request",
    "video.plan_subtitles",
    "video.subtitles_request",
    "video.plan_package",
    "video.package_request",
    "video.plan_export",
    "video.export_request",
    "video.inspect_job",
    "video.get_artifact_handle",
];

const VIDEO_PERMISSION_SCOPES: &[&str] = &[
    "video.provider.inspect",
    "video.import",
    "video.open",
    "video.metadata.read",
    "video.track.read",
    "video.thumbnail",
    "video.transcode",
    "video.segment",
    "video.render",
    "video.subtitle",
    "video.package",
    "video.export",
    "video.job.read",
    "video.artifact.read",
];

const VIDEO_TRANSCODE_METADATA: &[(&str, &str)] = &[
    ("metadata", "true"),
    ("tracks", "true"),
    ("transcode", "true"),
    ("segment", "true"),
];
const VIDEO_RENDER_METADATA: &[(&str, &str)] = &[
    ("thumbnail", "true"),
    ("render", "true"),
    ("subtitles", "true"),
    ("overlays", "true"),
];
const VIDEO_PACKAGE_METADATA: &[(&str, &str)] = &[
    ("package", "true"),
    ("adaptive", "true"),
    ("job_status", "true"),
    ("export", "true"),
];
const VIDEO_MOCK_METADATA: &[(&str, &str)] = &[
    ("metadata", "true"),
    ("tracks", "true"),
    ("package", "false"),
    ("export", "true"),
];
const VIDEO_UNAVAILABLE_METADATA: &[(&str, &str)] = &[
    ("metadata", "false"),
    ("tracks", "false"),
    ("package", "false"),
    ("export", "false"),
];

const VIDEO_PROVIDER_CLASSES: &[MediaProviderClass<'_>] = &[
    MediaProviderClass {
        provider_class: "video-transcode",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: VIDEO_TRANSCODE_METADATA,
    },
    MediaProviderClass {
        provider_class: "video-render",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: VIDEO_RENDER_METADATA,
    },
    MediaProviderClass {
        provider_class: "video-package",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: VIDEO_PACKAGE_METADATA,
    },
    MediaProviderClass {
        provider_class: "mock",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: VIDEO_MOCK_METADATA,
    },
    MediaProviderClass {
        provider_class: "unavailable",
        availability: DomainPackProviderCapabilityState::Unavailable,
        metadata: VIDEO_UNAVAILABLE_METADATA,
    },
];

/// Build the video pack descriptor without binding codec stacks, queues, browser APIs, or CDNs.
pub fn media_video_pack_definition() -> DomainPackDefinition {
    media_pack_definition(MediaPackDescriptor {
        pack_id: MEDIA_VIDEO_PACK_ID,
        child_change_id: "openspec:add-pack-media-video",
        docs_slug: "video",
        service_id: MEDIA_VIDEO_SERVICE_ID,
        commands: MEDIA_VIDEO_COMMANDS,
        permission_scopes: VIDEO_PERMISSION_SCOPES,
        provider_classes: VIDEO_PROVIDER_CLASSES,
        health_probe: "video.inspect_provider",
        unavailable_reason: "media_video_provider_not_installed",
        replay_schema: "media.video.replay.v1",
        data_classification: "media_video_metadata",
        retention_policy: "video_handles_tracks_plans_jobs_and_artifacts_by_reference",
        redaction_policy: "credentials_private_video_faces_voice_biometrics_frames_subtitles_exports_and_provider_payloads_redacted",
        examples: &[
            "Declare `pack.media.video.v1` as optional until a video provider is installed.",
            "Use video handles, track refs, timeline ranges, job status, and artifact handles instead of raw frames.",
        ],
        migration_notes: &[
            "Video commands become callable only after an approved media video provider registers matching schemas.",
            "Provider-native queues, presets, chunks, subtitles, frames, manifests, and delivery state stay behind adapters.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VideoScope {
    pub tenant_scope: String,
    pub workspace_ref: String,
    pub permission_profile: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VideoProviderCapability {
    pub provider_class: String,
    pub codecs: BTreeSet<String>,
    pub containers: BTreeSet<String>,
    pub features: BTreeSet<String>,
    pub hardware_classes: BTreeSet<String>,
    pub state: DomainPackProviderCapabilityState,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VideoHandle {
    pub video_id: String,
    pub version_hash: String,
    pub container: String,
    pub scope: VideoScope,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VideoMetadata {
    pub video_id: String,
    pub duration_ms: u64,
    pub width_px: u32,
    pub height_px: u32,
    pub frame_rate_millis: u32,
    pub metadata_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VideoTrack {
    pub track_id: String,
    pub track_kind: String,
    pub codec: String,
    pub language_tag: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VideoFrameHandle {
    pub video_id: String,
    pub frame_index: u64,
    pub frame_hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VideoTimelineRange {
    pub start_ms: u64,
    pub end_ms: u64,
}

impl VideoTimelineRange {
    pub fn duration_ms(&self) -> u64 {
        self.end_ms.saturating_sub(self.start_ms)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VideoThumbnailPlan {
    pub plan_id: String,
    pub range: VideoTimelineRange,
    pub width_px: u32,
    pub height_px: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VideoTranscodePlan {
    pub plan_id: String,
    pub target_codec: String,
    pub target_container: String,
    pub rendition_profile: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VideoSegmentPlan {
    pub plan_id: String,
    pub ranges: Vec<VideoTimelineRange>,
    pub segment_profile: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VideoOverlayOperation {
    pub overlay_id: String,
    pub asset_ref: String,
    pub range: VideoTimelineRange,
    pub placement_hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VideoRenderPlan {
    pub plan_id: String,
    pub base_version_hash: String,
    pub overlays: Vec<VideoOverlayOperation>,
    pub approval_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VideoSubtitlePlan {
    pub plan_id: String,
    pub subtitle_ref: String,
    pub language_tag: String,
    pub redaction_profile: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VideoPackagePlan {
    pub plan_id: String,
    pub package_profile: String,
    pub rendition_refs: Vec<String>,
    pub manifest_policy: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VideoExportPlan {
    pub export_id: String,
    pub target_container: String,
    pub quality_profile: String,
    pub strip_metadata: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VideoJobStatus {
    pub job_id: String,
    pub state: String,
    pub progress_millis: u32,
    pub retryable: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VideoArtifactHandle {
    pub artifact_id: String,
    pub artifact_kind: String,
    pub expires_at_epoch_ms: u64,
}

define_media_command_wrappers!(
    VideoInspectProviderCommand,
    VideoImportVideoRequestCommand,
    VideoOpenVideoCommand,
    VideoInspectMetadataCommand,
    VideoInspectTracksCommand,
    VideoPlanThumbnailCommand,
    VideoThumbnailRequestCommand,
    VideoPlanTranscodeCommand,
    VideoTranscodeRequestCommand,
    VideoPlanSegmentCommand,
    VideoSegmentRequestCommand,
    VideoPlanRenderCommand,
    VideoRenderRequestCommand,
    VideoPlanSubtitlesCommand,
    VideoSubtitlesRequestCommand,
    VideoPlanPackageCommand,
    VideoPackageRequestCommand,
    VideoPlanExportCommand,
    VideoExportRequestCommand,
    VideoInspectJobCommand,
    VideoGetArtifactHandleCommand,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VideoResultStatus {
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
    TrackDenied,
    SubtitleDenied,
    RenderDenied,
    PackageDenied,
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
pub struct VideoResultEnvelope<T> {
    pub status: VideoResultStatus,
    pub data: Option<T>,
    pub page: Option<MediaPage<T>>,
    pub error: Option<MediaError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VideoDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub descriptor_hash: String,
    pub provider_capability_hash: String,
    pub codec_container_hash: String,
    pub video_version_hash: String,
    pub track_mapping_hash: String,
    pub timeline_range_hash: String,
    pub thumbnail_plan_hash: String,
    pub transcode_plan_hash: String,
    pub segment_plan_hash: String,
    pub render_graph_hash: String,
    pub subtitle_plan_hash: String,
    pub package_plan_hash: String,
    pub export_plan_hash: String,
    pub job_status_hash: String,
    pub artifact_handle_hash: String,
    pub event_cursor_hash: String,
    pub redaction_metadata_hash: String,
}

pub fn media_video_descriptor_hashes() -> VideoDescriptorHashes {
    let range = VideoTimelineRange {
        start_ms: 0,
        end_ms: 1_000,
    };
    let overlay = VideoOverlayOperation {
        overlay_id: "overlay".into(),
        asset_ref: "asset:overlay".into(),
        range: range.clone(),
        placement_hash: "placement".into(),
    };
    VideoDescriptorHashes {
        command_schema_hash: video_stable_hash(&MEDIA_VIDEO_COMMANDS),
        result_schema_hash: video_stable_hash(&VideoResultStatus::Success),
        descriptor_hash: video_stable_hash(&media_video_pack_definition()),
        provider_capability_hash: video_stable_hash(&VideoProviderCapability {
            provider_class: "mock".into(),
            codecs: BTreeSet::from(["h264".into(), "vp9".into()]),
            containers: BTreeSet::from(["mp4".into(), "webm".into()]),
            features: BTreeSet::from(["metadata".into(), "tracks".into(), "export".into()]),
            hardware_classes: BTreeSet::from(["cpu".into()]),
            state: DomainPackProviderCapabilityState::Preview,
        }),
        codec_container_hash: video_stable_hash(&BTreeMap::from([
            ("codec", "h264"),
            ("container", "mp4"),
        ])),
        video_version_hash: video_stable_hash(&VideoHandle {
            video_id: "video".into(),
            version_hash: "v1".into(),
            container: "mp4".into(),
            scope: VideoScope::default(),
        }),
        track_mapping_hash: video_stable_hash(&VideoTrack {
            track_id: "track".into(),
            track_kind: "video".into(),
            codec: "h264".into(),
            language_tag: None,
        }),
        timeline_range_hash: video_stable_hash(&range),
        thumbnail_plan_hash: video_stable_hash(&VideoThumbnailPlan {
            plan_id: "thumb".into(),
            range: range.clone(),
            width_px: 320,
            height_px: 180,
        }),
        transcode_plan_hash: video_stable_hash(&VideoTranscodePlan {
            plan_id: "transcode".into(),
            target_codec: "h264".into(),
            target_container: "mp4".into(),
            rendition_profile: "mobile".into(),
        }),
        segment_plan_hash: video_stable_hash(&VideoSegmentPlan {
            plan_id: "segment".into(),
            ranges: vec![range.clone()],
            segment_profile: "preview".into(),
        }),
        render_graph_hash: video_stable_hash(&VideoRenderPlan {
            plan_id: "render".into(),
            base_version_hash: "v1".into(),
            overlays: vec![overlay],
            approval_ref: Some("approval".into()),
        }),
        subtitle_plan_hash: video_stable_hash(&VideoSubtitlePlan {
            plan_id: "subtitle".into(),
            subtitle_ref: "subtitle:ref".into(),
            language_tag: "en-US".into(),
            redaction_profile: "pii".into(),
        }),
        package_plan_hash: video_stable_hash(&VideoPackagePlan {
            plan_id: "package".into(),
            package_profile: "adaptive".into(),
            rendition_refs: vec!["rendition:mobile".into()],
            manifest_policy: "private".into(),
        }),
        export_plan_hash: video_stable_hash(&VideoExportPlan {
            export_id: "export".into(),
            target_container: "mp4".into(),
            quality_profile: "preview".into(),
            strip_metadata: true,
        }),
        job_status_hash: video_stable_hash(&VideoJobStatus {
            job_id: "job".into(),
            state: "planned".into(),
            progress_millis: 0,
            retryable: false,
        }),
        artifact_handle_hash: video_stable_hash(&VideoArtifactHandle {
            artifact_id: "artifact".into(),
            artifact_kind: "manifest".into(),
            expires_at_epoch_ms: 1,
        }),
        event_cursor_hash: video_stable_hash(&"cursor:video"),
        redaction_metadata_hash: video_stable_hash(&MediaError {
            code: "unavailable".into(),
            message: "media video provider is not installed".into(),
            retryable: false,
            trace_safe_detail: Some("media_video_provider_not_installed".into()),
        }),
    }
}

pub fn video_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    media_stable_hash(value)
}
