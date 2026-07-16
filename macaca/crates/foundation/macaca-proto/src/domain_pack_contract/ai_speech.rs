use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::ai_common::{
    ai_bounded_token, ai_pack_definition, ai_stable_hash, define_ai_command_wrappers,
    AiPackCommandEnvelope, AiPackDescriptor, AiPackError, AiPackPage, AiProviderClass,
};
use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};

pub const AI_SPEECH_PACK_ID: &str = "pack.ai.speech.v1";
pub const AI_SPEECH_SERVICE_ID: &str = "service.ai.speech";

/// Canonical command names described by `pack.ai.speech.v1`.
pub const AI_SPEECH_COMMANDS: &[&str] = &[
    "speech.speech_to_text",
    "speech.text_to_speech",
    "speech.list_voices",
    "speech.translate_speech",
    "speech.align_timing",
];

const SPEECH_PERMISSION_SCOPES: &[&str] = &[
    "ai.speech.recognize",
    "ai.speech.synthesize",
    "ai.speech.translate",
];

const SPEECH_RECOGNITION_METADATA: &[(&str, &str)] = &[
    ("streaming", "true"),
    ("diarization", "true"),
    ("raw_audio_in_trace", "false"),
];
const SPEECH_SYNTHESIS_METADATA: &[(&str, &str)] = &[
    ("voice_catalog", "true"),
    ("style", "limited"),
    ("generated_audio_in_trace", "false"),
];
const TRANSLATION_METADATA: &[(&str, &str)] = &[("translation", "true"), ("word_timing", "true")];
const MOCK_METADATA: &[(&str, &str)] =
    &[("deterministic", "true"), ("audio_payloads", "synthetic")];
const UNAVAILABLE_METADATA: &[(&str, &str)] =
    &[("callable", "false"), ("reason", "provider_not_installed")];

const SPEECH_PROVIDER_CLASSES: &[AiProviderClass<'_>] = &[
    AiProviderClass {
        provider_class: "speech-recognition",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: SPEECH_RECOGNITION_METADATA,
    },
    AiProviderClass {
        provider_class: "speech-synthesis",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: SPEECH_SYNTHESIS_METADATA,
    },
    AiProviderClass {
        provider_class: "remote-service",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: TRANSLATION_METADATA,
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

/// Build the speech pack descriptor without binding concrete recognition or synthesis engines.
pub fn ai_speech_pack_definition() -> DomainPackDefinition {
    ai_pack_definition(AiPackDescriptor {
        pack_id: AI_SPEECH_PACK_ID,
        child_change_id: "openspec:add-pack-ai-speech",
        docs_slug: "speech",
        sdk_slug: "speech",
        service_id: AI_SPEECH_SERVICE_ID,
        commands: AI_SPEECH_COMMANDS,
        permission_scopes: SPEECH_PERMISSION_SCOPES,
        provider_classes: SPEECH_PROVIDER_CLASSES,
        health_probe: "speech.list_voices",
        unavailable_reason: "ai_speech_provider_not_installed",
        replay_schema: "ai.speech.replay.v1",
        data_classification: "ai_speech_reference_metadata",
        retention_policy: "audio_refs_frames_transcript_segments_voice_descriptors_alignment_and_artifacts_by_reference",
        redaction_policy: "raw_audio_generated_speech_transcripts_voice_biometrics_credentials_and_provider_payloads_redacted",
        timeout_ms: 180_000,
        budget_units: 10,
        examples: &[
            "Declare `pack.ai.speech.v1` as optional until a speech provider is installed.",
            "Use audio handles, transcript refs, voice descriptors, and artifact refs instead of raw audio bytes.",
        ],
        migration_notes: &[
            "Speech commands become callable only after an approved speech service provider registers matching schemas.",
            "Provider-native audio bytes, generated speech, voice identities, and transcripts stay behind adapters.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpeechAudioInput {
    pub audio_ref: String,
    pub codec: String,
    pub duration_ms: u64,
    pub content_hash: String,
    pub redaction_class: String,
}

impl SpeechAudioInput {
    /// Bound speech operations before provider invocation.
    pub fn is_bounded(&self, max_duration_ms: u64) -> bool {
        ai_bounded_token(&self.audio_ref, 128)
            && ai_bounded_token(&self.codec, 64)
            && self.duration_ms > 0
            && self.duration_ms <= max_duration_ms
            && ai_bounded_token(&self.content_hash, 256)
            && ai_bounded_token(&self.redaction_class, 64)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpeechStreamFrame {
    pub stream_ref: String,
    pub sequence: u64,
    pub frame_kind: String,
    pub audio_chunk_ref: Option<String>,
    pub transcript_delta_ref: Option<String>,
    pub terminal: bool,
}

impl SpeechStreamFrame {
    /// Validate streaming frames so late frames cannot follow terminal finalization.
    pub fn sequence_is_finalized(frames: &[SpeechStreamFrame]) -> bool {
        !frames.is_empty()
            && frames.windows(2).all(|window| {
                let left = &window[0];
                let right = &window[1];
                left.stream_ref == right.stream_ref
                    && !left.terminal
                    && right.sequence == left.sequence + 1
            })
            && frames.iter().all(|frame| {
                ai_bounded_token(&frame.stream_ref, 128)
                    && matches!(
                        frame.frame_kind.as_str(),
                        "audio_delta" | "transcript_delta" | "final" | "cancelled"
                    )
                    && frame
                        .audio_chunk_ref
                        .as_ref()
                        .is_none_or(|reference| ai_bounded_token(reference, 256))
                    && frame
                        .transcript_delta_ref
                        .as_ref()
                        .is_none_or(|reference| ai_bounded_token(reference, 256))
            })
            && frames.last().is_some_and(|frame| frame.terminal)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TranscriptSegment {
    pub segment_ref: String,
    pub speaker_ref: Option<String>,
    pub text_ref: String,
    pub start_ms: u64,
    pub end_ms: u64,
    pub confidence_micros: u32,
}

impl TranscriptSegment {
    /// Validate diarization, channel, language, and word-timing compatible segment metadata.
    pub fn is_aligned(&self) -> bool {
        ai_bounded_token(&self.segment_ref, 128)
            && self
                .speaker_ref
                .as_ref()
                .is_none_or(|speaker| ai_bounded_token(speaker, 128))
            && ai_bounded_token(&self.text_ref, 256)
            && self.start_ms < self.end_ms
            && self.confidence_micros <= 1_000_000
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoiceDescriptor {
    pub voice_ref: String,
    pub locale_tags: BTreeSet<String>,
    pub style_tags: BTreeSet<String>,
    pub consent_required: bool,
}

impl VoiceDescriptor {
    /// Validate voice catalog compatibility without exposing biometric voice data.
    pub fn supports(&self, locale: &str, style: &str) -> bool {
        ai_bounded_token(&self.voice_ref, 128)
            && self.locale_tags.contains(locale)
            && self.style_tags.contains(style)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpeechSynthesisRequest {
    pub request_ref: String,
    pub text_ref: String,
    pub voice_ref: String,
    pub output_format: String,
    pub max_duration_ms: u64,
}

impl SpeechSynthesisRequest {
    /// Validate synthesis output compatibility before generated speech is created.
    pub fn is_compatible_with(&self, voice: &VoiceDescriptor, locale: &str, style: &str) -> bool {
        ai_bounded_token(&self.request_ref, 128)
            && ai_bounded_token(&self.text_ref, 256)
            && self.voice_ref == voice.voice_ref
            && matches!(self.output_format.as_str(), "wav" | "mp3" | "opus")
            && self.max_duration_ms > 0
            && voice.supports(locale, style)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpeechSynthesisResult {
    pub result_ref: String,
    pub audio_artifact_ref: String,
    pub duration_ms: u64,
    pub usage_ref: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpeechAlignment {
    pub alignment_ref: String,
    pub segment_ref: String,
    pub word_timing_ref: String,
    pub language_tag: String,
}

impl SpeechAlignment {
    /// Validate language and word-timing references without exposing transcript text.
    pub fn is_bounded(&self) -> bool {
        ai_bounded_token(&self.alignment_ref, 128)
            && ai_bounded_token(&self.segment_ref, 128)
            && ai_bounded_token(&self.word_timing_ref, 256)
            && ai_bounded_token(&self.language_tag, 32)
    }
}

define_ai_command_wrappers!(
    SpeechSpeechToTextCommand,
    SpeechTextToSpeechCommand,
    SpeechListVoicesCommand,
    SpeechTranslateSpeechCommand,
    SpeechAlignTimingCommand,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpeechResultStatus {
    Success,
    Partial,
    Denied,
    Unavailable,
    Unsupported,
    Conflict,
    QuotaExceeded,
    StreamCancelled,
    VoiceUnsupported,
    AudioTooLong,
    AlignmentUnavailable,
    ProviderFailure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpeechResultEnvelope<T> {
    pub status: SpeechResultStatus,
    pub data: Option<T>,
    pub page: Option<AiPackPage<T>>,
    pub error: Option<AiPackError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpeechDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub descriptor_hash: String,
    pub provider_capability_hash: String,
    pub audio_hash: String,
    pub frame_hash: String,
    pub voice_hash: String,
    pub alignment_hash: String,
}

pub fn ai_speech_descriptor_hashes() -> SpeechDescriptorHashes {
    SpeechDescriptorHashes {
        command_schema_hash: speech_stable_hash(&AI_SPEECH_COMMANDS),
        result_schema_hash: speech_stable_hash(&SpeechResultStatus::Success),
        descriptor_hash: speech_stable_hash(&ai_speech_pack_definition()),
        provider_capability_hash: speech_stable_hash(&BTreeMap::from([(
            "provider_class".to_string(),
            "mock".to_string(),
        )])),
        audio_hash: speech_stable_hash(&SpeechAudioInput {
            audio_ref: "audio".into(),
            codec: "audio-ref".into(),
            duration_ms: 1_000,
            content_hash: "audio-hash".into(),
            redaction_class: "private".into(),
        }),
        frame_hash: speech_stable_hash(&SpeechStreamFrame {
            stream_ref: "stream".into(),
            sequence: 1,
            frame_kind: "audio_delta".into(),
            audio_chunk_ref: Some("chunk-ref".into()),
            transcript_delta_ref: None,
            terminal: false,
        }),
        voice_hash: speech_stable_hash(&VoiceDescriptor {
            voice_ref: "voice".into(),
            locale_tags: BTreeSet::from(["und".into()]),
            style_tags: BTreeSet::from(["neutral".into()]),
            consent_required: true,
        }),
        alignment_hash: speech_stable_hash(&SpeechAlignment {
            alignment_ref: "alignment".into(),
            segment_ref: "segment".into(),
            word_timing_ref: "word-timing".into(),
            language_tag: "und".into(),
        }),
    }
}

pub fn speech_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    ai_stable_hash(value)
}
