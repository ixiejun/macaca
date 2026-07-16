use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use super::media_audio::*;
use super::media_common::MediaCommandEnvelope;
use super::media_image::*;
use super::media_rendering::*;
use super::media_transcription::*;
use super::media_video::*;
use super::*;

// Media pack tests validate provider-neutral contract shape only. They do not
// load images, read audio/video, stream chunks, render pixels, call model APIs,
// fetch remote assets, or expose raw media/provider payloads in fixtures.

#[test]
fn media_descriptors_are_discoverable_and_not_callable() {
    let cases = [
        (
            media_image_pack_definition(),
            MEDIA_IMAGE_PACK_ID,
            MEDIA_IMAGE_SERVICE_ID,
            MEDIA_IMAGE_COMMANDS,
            "media_image_provider_not_installed",
            "raster-image",
            "image.open_image",
        ),
        (
            media_audio_pack_definition(),
            MEDIA_AUDIO_PACK_ID,
            MEDIA_AUDIO_SERVICE_ID,
            MEDIA_AUDIO_COMMANDS,
            "media_audio_provider_not_installed",
            "audio-transcode",
            "audio.open_audio",
        ),
        (
            media_video_pack_definition(),
            MEDIA_VIDEO_PACK_ID,
            MEDIA_VIDEO_SERVICE_ID,
            MEDIA_VIDEO_COMMANDS,
            "media_video_provider_not_installed",
            "video-transcode",
            "video.open_video",
        ),
        (
            media_transcription_pack_definition(),
            MEDIA_TRANSCRIPTION_PACK_ID,
            MEDIA_TRANSCRIPTION_SERVICE_ID,
            MEDIA_TRANSCRIPTION_COMMANDS,
            "media_transcription_provider_not_installed",
            "transcription-batch",
            "transcription.open_source",
        ),
        (
            media_rendering_pack_definition(),
            MEDIA_RENDERING_PACK_ID,
            MEDIA_RENDERING_SERVICE_ID,
            MEDIA_RENDERING_COMMANDS,
            "media_rendering_provider_not_installed",
            "render-cpu",
            "rendering.open_source",
        ),
    ];

    for (definition, pack_id, service_id, commands, unavailable_reason, provider_class, command) in
        cases
    {
        assert_eq!(definition.pack_id, pack_id);
        assert!(!definition.is_callable());
        assert!(DomainPackDefinitionSpec.validate(&definition).is_ok());
        assert_eq!(
            definition.metadata.parent_pack_id.as_deref(),
            Some("pack.media.v1")
        );
        assert_eq!(
            definition.metadata.diagnostics.unavailable_reason,
            unavailable_reason
        );
        assert!(definition
            .metadata
            .sdk
            .docs_url
            .contains("developer-packs/media"));
        assert!(definition
            .metadata
            .provider_descriptors
            .contains_key(provider_class));
        assert!(definition
            .metadata
            .service_command_schemas
            .get(service_id)
            .is_some_and(|schemas| schemas.contains(command)));

        let descriptor_commands = definition
            .metadata
            .service_command_schemas
            .get(service_id)
            .expect("media descriptor exposes command schemas");
        for expected in commands {
            assert!(
                descriptor_commands.contains(*expected),
                "missing command {expected}"
            );
        }
    }
}

#[test]
fn industrial_catalog_uses_specialized_media_descriptors() {
    let definitions = industrial_reference_domain_pack_definitions();
    let image = definitions
        .iter()
        .find(|definition| definition.pack_id == MEDIA_IMAGE_PACK_ID)
        .expect("industrial catalog includes media image");
    let transcription = definitions
        .iter()
        .find(|definition| definition.pack_id == MEDIA_TRANSCRIPTION_PACK_ID)
        .expect("industrial catalog includes media transcription");
    let rendering = definitions
        .iter()
        .find(|definition| definition.pack_id == MEDIA_RENDERING_PACK_ID)
        .expect("industrial catalog includes media rendering");

    assert_eq!(
        image.metadata.diagnostics.unavailable_reason,
        "media_image_provider_not_installed"
    );
    assert!(image
        .metadata
        .service_command_schemas
        .get(MEDIA_IMAGE_SERVICE_ID)
        .is_some_and(|commands| commands.contains("image.plan_transform")));
    assert_eq!(
        transcription
            .metadata
            .provider_descriptors
            .get("transcription-streaming")
            .and_then(|descriptor| descriptor.metadata.get("interim_results"))
            .map(String::as_str),
        Some("true")
    );
    assert_eq!(
        rendering
            .metadata
            .provider_descriptors
            .get("render-browser")
            .and_then(|descriptor| descriptor.metadata.get("script_policy"))
            .map(String::as_str),
        Some("true")
    );
}

#[test]
fn media_command_dtos_are_serde_compatible() {
    let envelope = MediaCommandEnvelope {
        subject_ref: "media:subject".into(),
        parameters: BTreeMap::from([("mode".into(), "preview".into())]),
        cursor: None,
        page_size: Some(25),
        idempotency_key: Some("idem-media".into()),
    };

    let values = [
        serde_json::to_value(ImageOpenImageCommand {
            request: envelope.clone(),
        })
        .unwrap(),
        serde_json::to_value(AudioOpenAudioCommand {
            request: envelope.clone(),
        })
        .unwrap(),
        serde_json::to_value(VideoOpenVideoCommand {
            request: envelope.clone(),
        })
        .unwrap(),
        serde_json::to_value(TranscriptionOpenSourceCommand {
            request: envelope.clone(),
        })
        .unwrap(),
        serde_json::to_value(RenderingOpenSourceCommand { request: envelope }).unwrap(),
    ];

    assert!(values.iter().all(|value| value.is_object()));
}

#[test]
fn media_descriptor_hashes_are_stable_and_distinct() {
    let hash_groups = [
        hash_values(&media_image_descriptor_hashes()),
        hash_values(&media_audio_descriptor_hashes()),
        hash_values(&media_video_descriptor_hashes()),
        hash_values(&media_transcription_descriptor_hashes()),
        hash_values(&media_rendering_descriptor_hashes()),
    ];

    for hashes in hash_groups {
        let unique = hashes.into_iter().collect::<BTreeSet<_>>();
        assert!(unique.len() >= 12);
        assert!(unique.iter().all(|hash| !hash.is_empty()));
    }
}

#[test]
fn media_validation_helpers_are_provider_neutral() {
    let geometry = ImagePixelGeometry {
        width_px: 640,
        height_px: 480,
        frame_count: 2,
        orientation: "normal".into(),
    };
    assert_eq!(geometry.pixel_count(), 614_400);

    let segment = AudioSegment {
        segment_id: "segment".into(),
        start_ms: 1_000,
        end_ms: 2_500,
    };
    assert_eq!(segment.duration_ms(), 1_500);

    let range = VideoTimelineRange {
        start_ms: 500,
        end_ms: 1_500,
    };
    assert_eq!(range.duration_ms(), 1_000);

    let session = TranscriptionStreamingSession {
        session_id: "session".into(),
        source_ref: "source:ref".into(),
        state: "accepting_chunks".into(),
        next_sequence: 3,
    };
    assert!(session.accepts_sequence(3));

    let viewport = RenderViewport {
        width_px: 800,
        height_px: 600,
        scale_millis: 1_000,
    };
    assert_eq!(viewport.pixel_budget(), 480_000);
}

#[test]
fn invalid_media_descriptor_is_rejected() {
    let mut invalid = media_image_pack_definition();
    invalid.pack_id = "pack.media.image.v2".into();
    assert!(DomainPackDefinitionSpec.validate(&invalid).is_err());
}

fn hash_values<T: Serialize>(value: &T) -> Vec<String> {
    serde_json::to_value(value)
        .expect("descriptor hash DTO is serializable")
        .as_object()
        .expect("descriptor hash DTO serializes as an object")
        .values()
        .map(|value| {
            value
                .as_str()
                .expect("descriptor hash fields are strings")
                .to_string()
        })
        .collect()
}
