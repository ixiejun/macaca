use macaca_proto::domain_pack_contract::{
    location_geocode::{LOCATION_GEOCODE_PACK_ID, LOCATION_GEOCODE_SERVICE_ID},
    location_maps::{LOCATION_MAPS_PACK_ID, LOCATION_MAPS_SERVICE_ID},
    location_place_search::{LOCATION_PLACE_SEARCH_PACK_ID, LOCATION_PLACE_SEARCH_SERVICE_ID},
    location_route::{LOCATION_ROUTE_PACK_ID, LOCATION_ROUTE_SERVICE_ID},
    location_timezone::{LOCATION_TIMEZONE_PACK_ID, LOCATION_TIMEZONE_SERVICE_ID},
};
use macaca_proto::{
    compose_installed_domain_pack_catalog, reference_domain_pack_definitions,
    AppServiceContractConfig,
};

use super::*;

// Location SDK tests validate catalog discovery only. The SDK must not create
// map, geocode, route, place, timezone, host-native, boundary, or network
// providers; it only reports provider-neutral descriptor metadata and explicit
// unavailable diagnostics from the installed catalog.

#[tokio::test]
async fn catalog_client_discovers_location_contract_metadata() {
    let catalog = compose_installed_domain_pack_catalog(reference_domain_pack_definitions());
    let client = CatalogBackedDomainPackClient::new(catalog);

    let cases = [
        (
            LOCATION_MAPS_PACK_ID,
            LOCATION_MAPS_SERVICE_ID,
            "maps.plan_static_render",
            "location_maps_provider_not_installed",
            "tile-matrix",
        ),
        (
            LOCATION_GEOCODE_PACK_ID,
            LOCATION_GEOCODE_SERVICE_ID,
            "geocode.reverse",
            "location_geocode_provider_not_installed",
            "reverse-geocoder",
        ),
        (
            LOCATION_ROUTE_PACK_ID,
            LOCATION_ROUTE_SERVICE_ID,
            "route.request_matrix",
            "location_route_provider_not_installed",
            "matrix-engine",
        ),
        (
            LOCATION_PLACE_SEARCH_PACK_ID,
            LOCATION_PLACE_SEARCH_SERVICE_ID,
            "place_search.resolve_suggestion",
            "location_place_search_provider_not_installed",
            "autocomplete",
        ),
        (
            LOCATION_TIMEZONE_PACK_ID,
            LOCATION_TIMEZONE_SERVICE_ID,
            "timezone.resolve_local_time",
            "location_timezone_provider_not_installed",
            "timezone-database",
        ),
    ];

    for (pack_id, service_id, command, unavailable_reason, provider_class) in cases {
        let inspect = client
            .inspect_pack(&DomainPackInspectCommand::new(pack_id).expect("valid location id"))
            .await
            .unwrap();
        let pack = inspect.pack.expect("location descriptor exists");

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
            .contains("developer-packs/location"));
    }
}

#[tokio::test]
async fn catalog_client_reports_location_unavailable_reasons() {
    let catalog = compose_installed_domain_pack_catalog(reference_domain_pack_definitions());
    let client = CatalogBackedDomainPackClient::new(catalog);
    let command = DomainPackResolveCommand {
        declaration: AppServiceContractConfig {
            optional_packs: vec![
                LOCATION_MAPS_PACK_ID.into(),
                LOCATION_GEOCODE_PACK_ID.into(),
                LOCATION_ROUTE_PACK_ID.into(),
                LOCATION_PLACE_SEARCH_PACK_ID.into(),
                LOCATION_TIMEZONE_PACK_ID.into(),
            ],
            ..Default::default()
        },
    };

    let result = client.resolve_declaration(&command).await.unwrap();

    for (pack_id, reason) in [
        (
            LOCATION_MAPS_PACK_ID,
            "location_maps_provider_not_installed",
        ),
        (
            LOCATION_GEOCODE_PACK_ID,
            "location_geocode_provider_not_installed",
        ),
        (
            LOCATION_ROUTE_PACK_ID,
            "location_route_provider_not_installed",
        ),
        (
            LOCATION_PLACE_SEARCH_PACK_ID,
            "location_place_search_provider_not_installed",
        ),
        (
            LOCATION_TIMEZONE_PACK_ID,
            "location_timezone_provider_not_installed",
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
