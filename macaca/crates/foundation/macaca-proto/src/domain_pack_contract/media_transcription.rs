use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::media_common::{
    define_media_command_wrappers, media_pack_definition, media_stable_hash, MediaCommandEnvelope,
    MediaError, MediaPackDescriptor, MediaPage, MediaProviderClass,
};
use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};

pub const MEDIA_TRANSCRIPTION_PACK_ID: &str = "pack.media.transcription.v1";
pub const MEDIA_TRANSCRIPTION_SERVICE_ID: &str = "service.media.transcription";

pub const MEDIA_TRANSCRIPTION_COMMANDS: &[&str] = &[
    "transcription.inspect_provider",
    "transcription.import_source_request",
    "transcription.open_source",
    "transcription.inspect_media",
    "transcription.plan_batch",
    "transcription.batch_request",
    "transcription.plan_stream",
    "transcription.start_stream",
    "transcription.append_stream_chunk",
    "transcription.finish_stream",
    "transcription.cancel_stream",
    "transcription.plan_diarization",
    "transcription.diarization_request",
    "transcription.align_timestamps",
    "transcription.normalize_transcript",
    "transcription.plan_redaction",
    "transcription.redaction_request",
    "transcription.plan_subtitle_export",
    "transcription.subtitle_export_request",
    "transcription.plan_translation_handoff",
    "transcription.translation_handoff_request",
    "transcription.inspect_job",
    "transcription.get_artifact_handle",
];

pub(crate) const TRANSCRIPTION_PERMISSION_SCOPES: &[&str] = &[
    "transcription.provider.inspect",
    "transcription.source.import",
    "transcription.source.open",
    "transcription.media.read",
    "transcription.batch",
    "transcription.stream",
    "transcription.stream.append",
    "transcription.stream.cancel",
    "transcription.diarization",
    "transcription.timestamp.align",
    "transcription.normalize",
    "transcription.redaction",
    "transcription.subtitle.export",
    "transcription.translation.handoff",
    "transcription.job.read",
    "transcription.artifact.read",
];

const TRANSCRIPTION_BATCH_METADATA: &[(&str, &str)] = &[
    ("batch", "true"),
    ("timestamps", "word_and_segment"),
    ("languages", "multi"),
    ("subtitles", "true"),
];
const TRANSCRIPTION_STREAMING_METADATA: &[(&str, &str)] = &[
    ("streaming", "true"),
    ("interim_results", "true"),
    ("chunk_order", "strict"),
    ("endpointing", "true"),
];
const TRANSCRIPTION_REDACTION_METADATA: &[(&str, &str)] = &[
    ("diarization", "true"),
    ("redaction", "true"),
    ("translation_handoff", "true"),
    ("vocabulary", "true"),
];
const TRANSCRIPTION_MOCK_METADATA: &[(&str, &str)] = &[
    ("batch", "true"),
    ("streaming", "false"),
    ("redaction", "true"),
    ("subtitles", "true"),
];
const TRANSCRIPTION_UNAVAILABLE_METADATA: &[(&str, &str)] = &[
    ("batch", "false"),
    ("streaming", "false"),
    ("redaction", "false"),
    ("subtitles", "false"),
];

const TRANSCRIPTION_PROVIDER_CLASSES: &[MediaProviderClass<'_>] = &[
    MediaProviderClass {
        provider_class: "transcription-batch",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: TRANSCRIPTION_BATCH_METADATA,
    },
    MediaProviderClass {
        provider_class: "transcription-streaming",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: TRANSCRIPTION_STREAMING_METADATA,
    },
    MediaProviderClass {
        provider_class: "transcript-redaction",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: TRANSCRIPTION_REDACTION_METADATA,
    },
    MediaProviderClass {
        provider_class: "mock",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: TRANSCRIPTION_MOCK_METADATA,
    },
    MediaProviderClass {
        provider_class: "unavailable",
        availability: DomainPackProviderCapabilityState::Unavailable,
        metadata: TRANSCRIPTION_UNAVAILABLE_METADATA,
    },
];

/// Build the transcription descriptor without binding any speech provider, model, queue, or language engine.
pub fn media_transcription_pack_definition() -> DomainPackDefinition {
    media_pack_definition(MediaPackDescriptor {
        pack_id: MEDIA_TRANSCRIPTION_PACK_ID,
        child_change_id: "openspec:add-pack-media-transcription",
        docs_slug: "transcription",
        service_id: MEDIA_TRANSCRIPTION_SERVICE_ID,
        commands: MEDIA_TRANSCRIPTION_COMMANDS,
        permission_scopes: TRANSCRIPTION_PERMISSION_SCOPES,
        provider_classes: TRANSCRIPTION_PROVIDER_CLASSES,
        health_probe: "transcription.inspect_provider",
        unavailable_reason: "media_transcription_provider_not_installed",
        replay_schema: "media.transcription.replay.v1",
        data_classification: "media_transcription_metadata",
        retention_policy: "source_handles_plans_stream_cursors_transcript_projections_jobs_and_artifacts_by_reference",
        redaction_policy: "credentials_private_audio_video_chunks_voice_biometrics_raw_transcripts_pii_subtitles_and_provider_payloads_redacted",
        examples: &[
            "Declare `pack.media.transcription.v1` as optional until a transcription provider is installed.",
            "Use source handles, chunk handles, transcript refs, token projections, and artifact handles instead of raw audio or text.",
        ],
        migration_notes: &[
            "Transcription commands become callable only after an approved provider registers matching schemas.",
            "Provider-native models, vocabulary payloads, callbacks, transcripts, subtitles, and streaming chunks stay behind adapters.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TranscriptionScope {
    pub tenant_scope: String,
    pub workspace_ref: String,
    pub permission_profile: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TranscriptionProviderCapability {
    pub provider_class: String,
    pub languages: BTreeSet<String>,
    pub model_classes: BTreeSet<String>,
    pub features: BTreeSet<String>,
    pub state: DomainPackProviderCapabilityState,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TranscriptionSourceHandle {
    pub source_id: String,
    pub version_hash: String,
    pub media_kind: String,
    pub scope: TranscriptionScope,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TranscriptionMediaMetadata {
    pub source_id: String,
    pub duration_ms: u64,
    pub channel_count: u16,
    pub language_hint: Option<String>,
    pub metadata_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanguageProfile {
    pub language_tag: String,
    pub confidence_ppm: u32,
    pub detected: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VocabularyReference {
    pub vocabulary_ref: String,
    pub language_tag: String,
    pub version_hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TranscriptionPlan {
    pub plan_id: String,
    pub source_version_hash: String,
    pub language_profiles: Vec<LanguageProfile>,
    pub vocabulary_refs: Vec<VocabularyReference>,
    pub redaction_profile: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TranscriptionStreamingSession {
    pub session_id: String,
    pub source_ref: String,
    pub state: String,
    pub next_sequence: u64,
}

impl TranscriptionStreamingSession {
    pub fn accepts_sequence(&self, sequence: u64) -> bool {
        self.state == "accepting_chunks" && self.next_sequence == sequence
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TranscriptionAudioChunkHandle {
    pub chunk_id: String,
    pub session_id: String,
    pub sequence: u64,
    pub chunk_hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TranscriptDocument {
    pub transcript_id: String,
    pub source_version_hash: String,
    pub text_ref: String,
    pub segment_refs: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TranscriptSegment {
    pub segment_id: String,
    pub start_ms: u64,
    pub end_ms: u64,
    pub speaker_label: Option<String>,
    pub text_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TranscriptToken {
    pub token_id: String,
    pub segment_id: String,
    pub start_ms: u64,
    pub end_ms: u64,
    pub token_ref: String,
    pub confidence_ppm: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpeakerLabel {
    pub speaker_ref: String,
    pub label_hash: String,
    pub confidence_ppm: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelLabel {
    pub channel_index: u16,
    pub label_hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TranscriptionRedactionProfile {
    pub profile_id: String,
    pub entity_classes: BTreeSet<String>,
    pub replacement_policy: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TranscriptionSubtitleExportPlan {
    pub export_id: String,
    pub subtitle_format: String,
    pub max_line_length: u32,
    pub redaction_profile: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TranscriptionTranslationHandoffPlan {
    pub handoff_id: String,
    pub transcript_ref: String,
    pub source_language: String,
    pub target_languages: BTreeSet<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TranscriptionJobStatus {
    pub job_id: String,
    pub state: String,
    pub processed_duration_ms: u64,
    pub retryable: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TranscriptionArtifactHandle {
    pub artifact_id: String,
    pub artifact_kind: String,
    pub expires_at_epoch_ms: u64,
}

define_media_command_wrappers!(
    TranscriptionInspectProviderCommand,
    TranscriptionImportSourceRequestCommand,
    TranscriptionOpenSourceCommand,
    TranscriptionInspectMediaCommand,
    TranscriptionPlanBatchCommand,
    TranscriptionBatchRequestCommand,
    TranscriptionPlanStreamCommand,
    TranscriptionStartStreamCommand,
    TranscriptionAppendStreamChunkCommand,
    TranscriptionFinishStreamCommand,
    TranscriptionCancelStreamCommand,
    TranscriptionPlanDiarizationCommand,
    TranscriptionDiarizationRequestCommand,
    TranscriptionAlignTimestampsCommand,
    TranscriptionNormalizeTranscriptCommand,
    TranscriptionPlanRedactionCommand,
    TranscriptionRedactionRequestCommand,
    TranscriptionPlanSubtitleExportCommand,
    TranscriptionSubtitleExportRequestCommand,
    TranscriptionPlanTranslationHandoffCommand,
    TranscriptionTranslationHandoffRequestCommand,
    TranscriptionInspectJobCommand,
    TranscriptionGetArtifactHandleCommand,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TranscriptionResultStatus {
    Success,
    Paged,
    Partial,
    Streaming,
    Asynchronous,
    Denied,
    Unavailable,
    Unsupported,
    Conflict,
    StaleVersion,
    SchemaMismatch,
    FormatUnsupported,
    LanguageUnsupported,
    ModelUnsupported,
    DiarizationUnsupported,
    TimestampUnsupported,
    RedactionDenied,
    TranslationDenied,
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
pub struct TranscriptionResultEnvelope<T> {
    pub status: TranscriptionResultStatus,
    pub data: Option<T>,
    pub page: Option<MediaPage<T>>,
    pub error: Option<MediaError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TranscriptionDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub descriptor_hash: String,
    pub provider_capability_hash: String,
    pub source_version_hash: String,
    pub language_profile_hash: String,
    pub vocabulary_reference_hash: String,
    pub plan_hash: String,
    pub streaming_session_cursor_hash: String,
    pub transcript_document_hash: String,
    pub segment_token_projection_hash: String,
    pub redaction_profile_hash: String,
    pub subtitle_export_plan_hash: String,
    pub translation_handoff_plan_hash: String,
    pub job_status_hash: String,
    pub artifact_handle_hash: String,
    pub event_cursor_hash: String,
    pub redaction_metadata_hash: String,
}

pub fn media_transcription_descriptor_hashes() -> TranscriptionDescriptorHashes {
    let language = LanguageProfile {
        language_tag: "en-US".into(),
        confidence_ppm: 900_000,
        detected: true,
    };
    let vocabulary = VocabularyReference {
        vocabulary_ref: "vocabulary:ref".into(),
        language_tag: "en-US".into(),
        version_hash: "v1".into(),
    };
    TranscriptionDescriptorHashes {
        command_schema_hash: transcription_stable_hash(&MEDIA_TRANSCRIPTION_COMMANDS),
        result_schema_hash: transcription_stable_hash(&TranscriptionResultStatus::Success),
        descriptor_hash: transcription_stable_hash(&media_transcription_pack_definition()),
        provider_capability_hash: transcription_stable_hash(&TranscriptionProviderCapability {
            provider_class: "mock".into(),
            languages: BTreeSet::from(["en-US".into()]),
            model_classes: BTreeSet::from(["general".into()]),
            features: BTreeSet::from(["batch".into(), "redaction".into(), "subtitles".into()]),
            state: DomainPackProviderCapabilityState::Preview,
        }),
        source_version_hash: transcription_stable_hash(&TranscriptionSourceHandle {
            source_id: "source".into(),
            version_hash: "v1".into(),
            media_kind: "audio".into(),
            scope: TranscriptionScope::default(),
        }),
        language_profile_hash: transcription_stable_hash(&language),
        vocabulary_reference_hash: transcription_stable_hash(&vocabulary),
        plan_hash: transcription_stable_hash(&TranscriptionPlan {
            plan_id: "batch".into(),
            source_version_hash: "v1".into(),
            language_profiles: vec![language],
            vocabulary_refs: vec![vocabulary],
            redaction_profile: Some("pii".into()),
        }),
        streaming_session_cursor_hash: transcription_stable_hash(&TranscriptionStreamingSession {
            session_id: "stream".into(),
            source_ref: "source:ref".into(),
            state: "accepting_chunks".into(),
            next_sequence: 7,
        }),
        transcript_document_hash: transcription_stable_hash(&TranscriptDocument {
            transcript_id: "transcript".into(),
            source_version_hash: "v1".into(),
            text_ref: "text:ref".into(),
            segment_refs: vec!["segment:1".into()],
        }),
        segment_token_projection_hash: transcription_stable_hash(&TranscriptToken {
            token_id: "token".into(),
            segment_id: "segment".into(),
            start_ms: 0,
            end_ms: 500,
            token_ref: "token:ref".into(),
            confidence_ppm: 800_000,
        }),
        redaction_profile_hash: transcription_stable_hash(&TranscriptionRedactionProfile {
            profile_id: "pii".into(),
            entity_classes: BTreeSet::from(["person".into(), "account".into()]),
            replacement_policy: "tokenize".into(),
        }),
        subtitle_export_plan_hash: transcription_stable_hash(&TranscriptionSubtitleExportPlan {
            export_id: "subtitle".into(),
            subtitle_format: "vtt".into(),
            max_line_length: 42,
            redaction_profile: "pii".into(),
        }),
        translation_handoff_plan_hash: transcription_stable_hash(
            &TranscriptionTranslationHandoffPlan {
                handoff_id: "translation".into(),
                transcript_ref: "transcript:ref".into(),
                source_language: "en-US".into(),
                target_languages: BTreeSet::from(["zh-CN".into()]),
            },
        ),
        job_status_hash: transcription_stable_hash(&TranscriptionJobStatus {
            job_id: "job".into(),
            state: "planned".into(),
            processed_duration_ms: 0,
            retryable: false,
        }),
        artifact_handle_hash: transcription_stable_hash(&TranscriptionArtifactHandle {
            artifact_id: "artifact".into(),
            artifact_kind: "subtitle".into(),
            expires_at_epoch_ms: 1,
        }),
        event_cursor_hash: transcription_stable_hash(&"cursor:transcription"),
        redaction_metadata_hash: transcription_stable_hash(&MediaError {
            code: "unavailable".into(),
            message: "media transcription provider is not installed".into(),
            retryable: false,
            trace_safe_detail: Some("media_transcription_provider_not_installed".into()),
        }),
    }
}

pub fn transcription_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    media_stable_hash(value)
}
