use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::media_common::{
    define_media_command_wrappers, media_pack_definition, media_stable_hash, MediaCommandEnvelope,
    MediaError, MediaPackDescriptor, MediaPage, MediaProviderClass,
};
use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};

pub const MEDIA_AUDIO_PACK_ID: &str = "pack.media.audio.v1";
pub const MEDIA_AUDIO_SERVICE_ID: &str = "service.media.audio";

pub const MEDIA_AUDIO_COMMANDS: &[&str] = &[
    "audio.inspect_provider",
    "audio.import_audio_request",
    "audio.open_audio",
    "audio.inspect_metadata",
    "audio.inspect_waveform",
    "audio.plan_transcode",
    "audio.transcode_request",
    "audio.plan_segment",
    "audio.segment_request",
    "audio.plan_filter",
    "audio.filter_request",
    "audio.plan_mix",
    "audio.mix_request",
    "audio.plan_synthesis",
    "audio.synthesis_request",
    "audio.plan_export",
    "audio.export_request",
    "audio.get_artifact_handle",
];

const AUDIO_PERMISSION_SCOPES: &[&str] = &[
    "audio.provider.inspect",
    "audio.import",
    "audio.open",
    "audio.metadata.read",
    "audio.waveform.read",
    "audio.transcode",
    "audio.segment",
    "audio.filter",
    "audio.mix",
    "audio.synthesize",
    "audio.export",
    "audio.artifact.read",
];

const AUDIO_TRANSCODE_METADATA: &[(&str, &str)] = &[
    ("metadata", "true"),
    ("waveform", "true"),
    ("transcode", "true"),
    ("segment", "true"),
];
const AUDIO_GRAPH_METADATA: &[(&str, &str)] = &[
    ("filter", "true"),
    ("mix", "true"),
    ("loudness", "true"),
    ("export", "true"),
];
const AUDIO_SYNTHESIS_METADATA: &[(&str, &str)] = &[
    ("synthesis", "true"),
    ("voice_catalog", "true"),
    ("streaming", "limited"),
    ("consent_required", "true"),
];
const AUDIO_MOCK_METADATA: &[(&str, &str)] = &[
    ("metadata", "true"),
    ("waveform", "true"),
    ("synthesis", "false"),
    ("export", "true"),
];
const AUDIO_UNAVAILABLE_METADATA: &[(&str, &str)] = &[
    ("metadata", "false"),
    ("waveform", "false"),
    ("synthesis", "false"),
    ("export", "false"),
];

const AUDIO_PROVIDER_CLASSES: &[MediaProviderClass<'_>] = &[
    MediaProviderClass {
        provider_class: "audio-transcode",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: AUDIO_TRANSCODE_METADATA,
    },
    MediaProviderClass {
        provider_class: "audio-graph",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: AUDIO_GRAPH_METADATA,
    },
    MediaProviderClass {
        provider_class: "speech-synthesis",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: AUDIO_SYNTHESIS_METADATA,
    },
    MediaProviderClass {
        provider_class: "mock",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: AUDIO_MOCK_METADATA,
    },
    MediaProviderClass {
        provider_class: "unavailable",
        availability: DomainPackProviderCapabilityState::Unavailable,
        metadata: AUDIO_UNAVAILABLE_METADATA,
    },
];

/// Build the audio pack descriptor without binding FFmpeg, WebAudio, cloud TTS, or device providers.
pub fn media_audio_pack_definition() -> DomainPackDefinition {
    media_pack_definition(MediaPackDescriptor {
        pack_id: MEDIA_AUDIO_PACK_ID,
        child_change_id: "openspec:add-pack-media-audio",
        docs_slug: "audio",
        service_id: MEDIA_AUDIO_SERVICE_ID,
        commands: MEDIA_AUDIO_COMMANDS,
        permission_scopes: AUDIO_PERMISSION_SCOPES,
        provider_classes: AUDIO_PROVIDER_CLASSES,
        health_probe: "audio.inspect_provider",
        unavailable_reason: "media_audio_provider_not_installed",
        replay_schema: "media.audio.replay.v1",
        data_classification: "media_audio_metadata",
        retention_policy: "audio_handles_metadata_waveforms_plans_voice_capabilities_and_artifacts_by_reference",
        redaction_policy: "credentials_raw_prompts_private_recordings_voice_biometrics_pcm_samples_generated_audio_and_provider_payloads_redacted",
        examples: &[
            "Declare `pack.media.audio.v1` as optional until an audio provider is installed.",
            "Use audio handles, segment ranges, graph plans, voice references, and artifact handles instead of raw samples.",
        ],
        migration_notes: &[
            "Audio commands become callable only after an approved media audio provider registers matching schemas.",
            "Provider-native filter graphs, voices, PCM samples, and generated audio stay behind adapters.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioScope {
    pub tenant_scope: String,
    pub workspace_ref: String,
    pub permission_profile: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioProviderCapability {
    pub provider_class: String,
    pub codecs: BTreeSet<String>,
    pub containers: BTreeSet<String>,
    pub features: BTreeSet<String>,
    pub max_duration_ms: u64,
    pub state: DomainPackProviderCapabilityState,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioHandle {
    pub audio_id: String,
    pub version_hash: String,
    pub container: String,
    pub codec: String,
    pub scope: AudioScope,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioMetadata {
    pub audio_id: String,
    pub duration_ms: u64,
    pub sample_rate_hz: u32,
    pub channel_count: u16,
    pub metadata_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioWaveformSummary {
    pub summary_id: String,
    pub sample_count: u64,
    pub peak_db_tenths: i32,
    pub loudness_lufs_tenths: i32,
    pub projection_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioSegment {
    pub segment_id: String,
    pub start_ms: u64,
    pub end_ms: u64,
}

impl AudioSegment {
    pub fn duration_ms(&self) -> u64 {
        self.end_ms.saturating_sub(self.start_ms)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioFilterOperation {
    pub operation_id: String,
    pub filter_kind: String,
    pub parameters: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioMixSource {
    pub source_id: String,
    pub audio_ref: String,
    pub segment: Option<AudioSegment>,
    pub gain_db_tenths: i32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioMixGraph {
    pub graph_id: String,
    pub sources: Vec<AudioMixSource>,
    pub filters: Vec<AudioFilterOperation>,
    pub output_channel_layout: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioVoiceCapability {
    pub voice_ref: String,
    pub language_tags: BTreeSet<String>,
    pub consent_required: bool,
    pub style_classes: BTreeSet<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioSynthesisPlan {
    pub plan_id: String,
    pub text_ref: String,
    pub voice_ref: String,
    pub safety_profile: String,
    pub output_duration_limit_ms: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioExportPlan {
    pub export_id: String,
    pub target_container: String,
    pub target_codec: String,
    pub loudness_profile: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioArtifactHandle {
    pub artifact_id: String,
    pub artifact_kind: String,
    pub expires_at_epoch_ms: u64,
}

define_media_command_wrappers!(
    AudioInspectProviderCommand,
    AudioImportAudioRequestCommand,
    AudioOpenAudioCommand,
    AudioInspectMetadataCommand,
    AudioInspectWaveformCommand,
    AudioPlanTranscodeCommand,
    AudioTranscodeRequestCommand,
    AudioPlanSegmentCommand,
    AudioSegmentRequestCommand,
    AudioPlanFilterCommand,
    AudioFilterRequestCommand,
    AudioPlanMixCommand,
    AudioMixRequestCommand,
    AudioPlanSynthesisCommand,
    AudioSynthesisRequestCommand,
    AudioPlanExportCommand,
    AudioExportRequestCommand,
    AudioGetArtifactHandleCommand,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioResultStatus {
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
    VoiceDenied,
    PromptDenied,
    SynthesisDenied,
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
pub struct AudioResultEnvelope<T> {
    pub status: AudioResultStatus,
    pub data: Option<T>,
    pub page: Option<MediaPage<T>>,
    pub error: Option<MediaError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub descriptor_hash: String,
    pub provider_capability_hash: String,
    pub codec_container_hash: String,
    pub audio_version_hash: String,
    pub waveform_summary_hash: String,
    pub segment_hash: String,
    pub filter_plan_hash: String,
    pub mix_graph_hash: String,
    pub voice_capability_hash: String,
    pub synthesis_plan_hash: String,
    pub export_plan_hash: String,
    pub artifact_handle_hash: String,
    pub event_cursor_hash: String,
    pub redaction_metadata_hash: String,
}

pub fn media_audio_descriptor_hashes() -> AudioDescriptorHashes {
    let segment = AudioSegment {
        segment_id: "segment".into(),
        start_ms: 100,
        end_ms: 1_100,
    };
    let filter = AudioFilterOperation {
        operation_id: "normalize".into(),
        filter_kind: "loudness".into(),
        parameters: BTreeMap::from([("profile".into(), "speech".into())]),
    };
    AudioDescriptorHashes {
        command_schema_hash: audio_stable_hash(&MEDIA_AUDIO_COMMANDS),
        result_schema_hash: audio_stable_hash(&AudioResultStatus::Success),
        descriptor_hash: audio_stable_hash(&media_audio_pack_definition()),
        provider_capability_hash: audio_stable_hash(&AudioProviderCapability {
            provider_class: "mock".into(),
            codecs: BTreeSet::from(["pcm".into(), "aac".into()]),
            containers: BTreeSet::from(["wav".into(), "mp4".into()]),
            features: BTreeSet::from(["metadata".into(), "waveform".into(), "export".into()]),
            max_duration_ms: 300_000,
            state: DomainPackProviderCapabilityState::Preview,
        }),
        codec_container_hash: audio_stable_hash(&BTreeMap::from([
            ("codec", "aac"),
            ("container", "mp4"),
        ])),
        audio_version_hash: audio_stable_hash(&AudioHandle {
            audio_id: "audio".into(),
            version_hash: "v1".into(),
            container: "wav".into(),
            codec: "pcm".into(),
            scope: AudioScope::default(),
        }),
        waveform_summary_hash: audio_stable_hash(&AudioWaveformSummary {
            summary_id: "waveform".into(),
            sample_count: 48_000,
            peak_db_tenths: -10,
            loudness_lufs_tenths: -160,
            projection_ref: "waveform:projection".into(),
        }),
        segment_hash: audio_stable_hash(&segment),
        filter_plan_hash: audio_stable_hash(&filter),
        mix_graph_hash: audio_stable_hash(&AudioMixGraph {
            graph_id: "mix".into(),
            sources: vec![AudioMixSource {
                source_id: "voice".into(),
                audio_ref: "audio:voice".into(),
                segment: Some(segment),
                gain_db_tenths: 0,
            }],
            filters: vec![filter],
            output_channel_layout: "stereo".into(),
        }),
        voice_capability_hash: audio_stable_hash(&AudioVoiceCapability {
            voice_ref: "voice:synthetic".into(),
            language_tags: BTreeSet::from(["en-US".into()]),
            consent_required: true,
            style_classes: BTreeSet::from(["neutral".into()]),
        }),
        synthesis_plan_hash: audio_stable_hash(&AudioSynthesisPlan {
            plan_id: "synthesis".into(),
            text_ref: "text:ref".into(),
            voice_ref: "voice:synthetic".into(),
            safety_profile: "strict".into(),
            output_duration_limit_ms: 30_000,
        }),
        export_plan_hash: audio_stable_hash(&AudioExportPlan {
            export_id: "export".into(),
            target_container: "wav".into(),
            target_codec: "pcm".into(),
            loudness_profile: "speech".into(),
        }),
        artifact_handle_hash: audio_stable_hash(&AudioArtifactHandle {
            artifact_id: "artifact".into(),
            artifact_kind: "preview".into(),
            expires_at_epoch_ms: 1,
        }),
        event_cursor_hash: audio_stable_hash(&"cursor:audio"),
        redaction_metadata_hash: audio_stable_hash(&MediaError {
            code: "unavailable".into(),
            message: "media audio provider is not installed".into(),
            retryable: false,
            trace_safe_detail: Some("media_audio_provider_not_installed".into()),
        }),
    }
}

pub fn audio_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    media_stable_hash(value)
}
