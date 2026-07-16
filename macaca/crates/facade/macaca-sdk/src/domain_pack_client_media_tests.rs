use macaca_proto::domain_pack_contract::{
    media_audio::{MEDIA_AUDIO_PACK_ID, MEDIA_AUDIO_SERVICE_ID},
    media_image::{MEDIA_IMAGE_PACK_ID, MEDIA_IMAGE_SERVICE_ID},
    media_rendering::{MEDIA_RENDERING_PACK_ID, MEDIA_RENDERING_SERVICE_ID},
    media_transcription::{MEDIA_TRANSCRIPTION_PACK_ID, MEDIA_TRANSCRIPTION_SERVICE_ID},
    media_video::{MEDIA_VIDEO_PACK_ID, MEDIA_VIDEO_SERVICE_ID},
};
use macaca_proto::{compose_installed_domain_pack_catalog, reference_domain_pack_definitions};

use super::*;

// These tests keep the Media SDK path provider-neutral. The SDK reads catalog
// metadata and never constructs codec stacks, renderers, browser automation,
// storage adapters, model APIs, speech providers, credentials, or media clients.

#[tokio::test]
async fn catalog_client_discovers_media_contract_metadata() {
    let catalog = compose_installed_domain_pack_catalog(reference_domain_pack_definitions());
    let client = CatalogBackedDomainPackClient::new(catalog);

    let cases = [
        (
            MEDIA_IMAGE_PACK_ID,
            MEDIA_IMAGE_SERVICE_ID,
            "image.open_image",
            "media_image_provider_not_installed",
            "raster-image",
        ),
        (
            MEDIA_AUDIO_PACK_ID,
            MEDIA_AUDIO_SERVICE_ID,
            "audio.open_audio",
            "media_audio_provider_not_installed",
            "audio-transcode",
        ),
        (
            MEDIA_VIDEO_PACK_ID,
            MEDIA_VIDEO_SERVICE_ID,
            "video.open_video",
            "media_video_provider_not_installed",
            "video-transcode",
        ),
        (
            MEDIA_TRANSCRIPTION_PACK_ID,
            MEDIA_TRANSCRIPTION_SERVICE_ID,
            "transcription.open_source",
            "media_transcription_provider_not_installed",
            "transcription-batch",
        ),
        (
            MEDIA_RENDERING_PACK_ID,
            MEDIA_RENDERING_SERVICE_ID,
            "rendering.open_source",
            "media_rendering_provider_not_installed",
            "render-cpu",
        ),
    ];

    for (pack_id, service_id, command, unavailable_reason, provider_class) in cases {
        let inspect = client
            .inspect_pack(&DomainPackInspectCommand::new(pack_id).expect("valid media id"))
            .await
            .unwrap();
        let pack = inspect.pack.expect("media descriptor exists");

        assert!(!pack.is_callable());
        assert_eq!(
            pack.metadata.diagnostics.unavailable_reason,
            unavailable_reason
        );
        assert!(pack
            .metadata
            .service_command_schemas
            .get(service_id)
            .is_some_and(|commands| commands.contains(command)));
        assert!(pack
            .metadata
            .provider_descriptors
            .contains_key(provider_class));
        assert!(pack.metadata.sdk.docs_url.contains("developer-packs/media"));
    }
}
