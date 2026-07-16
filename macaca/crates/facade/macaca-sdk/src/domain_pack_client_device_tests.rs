use macaca_proto::domain_pack_contract::{
    device_camera::{DEVICE_CAMERA_PACK_ID, DEVICE_CAMERA_SERVICE_ID},
    device_foreground_background_host::{
        DEVICE_FOREGROUND_BACKGROUND_HOST_PACK_ID, DEVICE_FOREGROUND_BACKGROUND_HOST_SERVICE_ID,
    },
    device_local_files::{DEVICE_LOCAL_FILES_PACK_ID, DEVICE_LOCAL_FILES_SERVICE_ID},
    device_notifications::{DEVICE_NOTIFICATIONS_PACK_ID, DEVICE_NOTIFICATIONS_SERVICE_ID},
    device_sensors::{DEVICE_SENSORS_PACK_ID, DEVICE_SENSORS_SERVICE_ID},
};
use macaca_proto::{
    compose_installed_domain_pack_catalog, reference_domain_pack_definitions,
    AppServiceContractConfig,
};

use super::*;

// Device SDK tests validate catalog discovery only. The SDK must not create
// host-native, browser, remote-host, sensor, camera, filesystem, notification,
// or lifecycle providers; it only reports provider-neutral descriptors and
// explicit unavailable diagnostics from the installed catalog.

#[tokio::test]
async fn catalog_client_discovers_device_contract_metadata() {
    let catalog = compose_installed_domain_pack_catalog(reference_domain_pack_definitions());
    let client = CatalogBackedDomainPackClient::new(catalog);

    let cases = [
        (
            DEVICE_SENSORS_PACK_ID,
            DEVICE_SENSORS_SERVICE_ID,
            "sensors.open_stream",
            "device_sensors_provider_not_installed",
            "host-native",
        ),
        (
            DEVICE_CAMERA_PACK_ID,
            DEVICE_CAMERA_SERVICE_ID,
            "camera.open_session",
            "device_camera_provider_not_installed",
            "browser",
        ),
        (
            DEVICE_LOCAL_FILES_PACK_ID,
            DEVICE_LOCAL_FILES_SERVICE_ID,
            "local_files.request_directory_handle",
            "device_local_files_provider_not_installed",
            "remote-host",
        ),
        (
            DEVICE_NOTIFICATIONS_PACK_ID,
            DEVICE_NOTIFICATIONS_SERVICE_ID,
            "notifications.subscribe_interactions",
            "device_notifications_provider_not_installed",
            "host-native",
        ),
        (
            DEVICE_FOREGROUND_BACKGROUND_HOST_PACK_ID,
            DEVICE_FOREGROUND_BACKGROUND_HOST_SERVICE_ID,
            "host_lifecycle.request_background_lease",
            "device_foreground_background_host_provider_not_installed",
            "browser",
        ),
    ];

    for (pack_id, service_id, command, unavailable_reason, provider_class) in cases {
        let inspect = client
            .inspect_pack(&DomainPackInspectCommand::new(pack_id).expect("valid device id"))
            .await
            .unwrap();
        let pack = inspect.pack.expect("device descriptor exists");

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
        assert!(pack
            .metadata
            .sdk
            .docs_url
            .contains("developer-packs/device"));
    }
}

#[tokio::test]
async fn catalog_client_reports_device_unavailable_reasons() {
    let catalog = compose_installed_domain_pack_catalog(reference_domain_pack_definitions());
    let client = CatalogBackedDomainPackClient::new(catalog);
    let command = DomainPackResolveCommand {
        declaration: AppServiceContractConfig {
            optional_packs: vec![
                DEVICE_SENSORS_PACK_ID.into(),
                DEVICE_CAMERA_PACK_ID.into(),
                DEVICE_LOCAL_FILES_PACK_ID.into(),
                DEVICE_NOTIFICATIONS_PACK_ID.into(),
                DEVICE_FOREGROUND_BACKGROUND_HOST_PACK_ID.into(),
            ],
            ..Default::default()
        },
    };

    let result = client.resolve_declaration(&command).await.unwrap();

    for (pack_id, reason) in [
        (
            DEVICE_SENSORS_PACK_ID,
            "device_sensors_provider_not_installed",
        ),
        (
            DEVICE_CAMERA_PACK_ID,
            "device_camera_provider_not_installed",
        ),
        (
            DEVICE_LOCAL_FILES_PACK_ID,
            "device_local_files_provider_not_installed",
        ),
        (
            DEVICE_NOTIFICATIONS_PACK_ID,
            "device_notifications_provider_not_installed",
        ),
        (
            DEVICE_FOREGROUND_BACKGROUND_HOST_PACK_ID,
            "device_foreground_background_host_provider_not_installed",
        ),
    ] {
        assert!(result
            .effective
            .unresolved_optional_packs
            .contains(&pack_id.to_string()));
        assert_eq!(
            result.effective.unavailable_pack_reasons.get(pack_id),
            Some(&reason.to_string())
        );
    }
}
