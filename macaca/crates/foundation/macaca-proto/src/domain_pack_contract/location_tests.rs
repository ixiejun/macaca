use std::collections::{BTreeMap, BTreeSet};

use super::location_common::*;
use super::location_geocode::*;
use super::location_maps::*;
use super::location_place_search::*;
use super::location_route::*;
use super::location_timezone::*;
use super::*;

// Location tests validate provider-neutral contract shape only. They do not
// contact map, geocode, routing, place, timezone, host-native, or boundary
// providers. Fixtures intentionally use synthetic references and hashes instead
// of raw tiles, raw addresses, exact coordinates, route geometry, place
// payloads, timezone boundary geometry, session tokens, or credentials.

#[test]
fn location_descriptors_are_discoverable_and_not_callable() {
    let cases = [
        (
            location_maps_pack_definition(),
            LOCATION_MAPS_PACK_ID,
            LOCATION_MAPS_SERVICE_ID,
            LOCATION_MAPS_COMMANDS,
            "location_maps_provider_not_installed",
            "tile-matrix",
            "maps.plan_static_render",
        ),
        (
            location_geocode_pack_definition(),
            LOCATION_GEOCODE_PACK_ID,
            LOCATION_GEOCODE_SERVICE_ID,
            LOCATION_GEOCODE_COMMANDS,
            "location_geocode_provider_not_installed",
            "reverse-geocoder",
            "geocode.reverse",
        ),
        (
            location_route_pack_definition(),
            LOCATION_ROUTE_PACK_ID,
            LOCATION_ROUTE_SERVICE_ID,
            LOCATION_ROUTE_COMMANDS,
            "location_route_provider_not_installed",
            "matrix-engine",
            "route.request_matrix",
        ),
        (
            location_place_search_pack_definition(),
            LOCATION_PLACE_SEARCH_PACK_ID,
            LOCATION_PLACE_SEARCH_SERVICE_ID,
            LOCATION_PLACE_SEARCH_COMMANDS,
            "location_place_search_provider_not_installed",
            "autocomplete",
            "place_search.resolve_suggestion",
        ),
        (
            location_timezone_pack_definition(),
            LOCATION_TIMEZONE_PACK_ID,
            LOCATION_TIMEZONE_SERVICE_ID,
            LOCATION_TIMEZONE_COMMANDS,
            "location_timezone_provider_not_installed",
            "timezone-database",
            "timezone.resolve_local_time",
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
            Some("pack.location.v1")
        );
        assert_eq!(
            definition.metadata.diagnostics.unavailable_reason,
            unavailable_reason
        );
        assert!(definition
            .metadata
            .sdk
            .docs_url
            .contains("developer-packs/location"));
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
            .expect("location descriptor exposes command schemas");
        for expected in commands {
            assert!(
                descriptor_commands.contains(*expected),
                "missing command {expected}"
            );
        }
    }
}

#[test]
fn industrial_catalog_uses_specialized_location_descriptors() {
    let definitions = industrial_reference_domain_pack_definitions();
    let maps = find_pack(&definitions, LOCATION_MAPS_PACK_ID);
    let geocode = find_pack(&definitions, LOCATION_GEOCODE_PACK_ID);
    let route = find_pack(&definitions, LOCATION_ROUTE_PACK_ID);
    let place = find_pack(&definitions, LOCATION_PLACE_SEARCH_PACK_ID);
    let timezone = find_pack(&definitions, LOCATION_TIMEZONE_PACK_ID);

    assert_eq!(
        maps.metadata
            .provider_descriptors
            .get("static-render")
            .and_then(|descriptor| descriptor.metadata.get("ui_widgets"))
            .map(String::as_str),
        Some("false")
    );
    assert_eq!(
        geocode
            .metadata
            .provider_descriptors
            .get("reverse-geocoder")
            .and_then(|descriptor| descriptor.metadata.get("private_coordinates"))
            .map(String::as_str),
        Some("redacted")
    );
    assert_eq!(
        route
            .metadata
            .provider_descriptors
            .get("waypoint-optimizer")
            .and_then(|descriptor| descriptor.metadata.get("dispatch_policy"))
            .map(String::as_str),
        Some("application_owned")
    );
    assert_eq!(
        place
            .metadata
            .provider_descriptors
            .get("place-details")
            .and_then(|descriptor| descriptor.metadata.get("raw_media_bytes"))
            .map(String::as_str),
        Some("false")
    );
    assert_eq!(
        timezone
            .metadata
            .provider_descriptors
            .get("boundary-lookup")
            .and_then(|descriptor| descriptor.metadata.get("raw_boundary_geometry"))
            .map(String::as_str),
        Some("false")
    );
}

#[test]
fn location_command_and_result_dtos_are_serde_compatible() {
    let envelope = LocationPackCommandEnvelope {
        subject_ref: "location:subject".into(),
        parameters: BTreeMap::from([("mode".into(), "synthetic".into())]),
        cursor: None,
        page_size: Some(20),
        idempotency_key: Some("idem-location".into()),
    };

    let values = [
        serde_json::to_value(MapsPlanStaticRenderCommand {
            request: envelope.clone(),
        })
        .unwrap(),
        serde_json::to_value(GeocodeReverseCommand {
            request: envelope.clone(),
        })
        .unwrap(),
        serde_json::to_value(RouteRequestMatrixCommand {
            request: envelope.clone(),
        })
        .unwrap(),
        serde_json::to_value(PlaceSearchResolveSuggestionCommand {
            request: envelope.clone(),
        })
        .unwrap(),
        serde_json::to_value(TimezoneResolveLocalTimeCommand { request: envelope }).unwrap(),
        serde_json::to_value(MapsResultEnvelope::<MapTileReference> {
            status: MapsResultStatus::AttributionMissing,
            data: None,
            page: None,
            error: Some(LocationPackError {
                code: "attribution_missing".into(),
                message: "synthetic missing attribution".into(),
                retryable: false,
                trace_safe_detail: Some("attribution_required".into()),
            }),
        })
        .unwrap(),
        serde_json::to_value(GeocodeResultEnvelope::<GeocodeCandidate> {
            status: GeocodeResultStatus::Ambiguous,
            data: None,
            page: None,
            error: None,
        })
        .unwrap(),
        serde_json::to_value(RouteResultEnvelope::<RoutePlan> {
            status: RouteResultStatus::NoRoute,
            data: None,
            page: None,
            error: None,
        })
        .unwrap(),
        serde_json::to_value(PlaceSearchResultEnvelope::<PlaceSummary> {
            status: PlaceSearchResultStatus::AttributionRequired,
            data: None,
            page: None,
            error: None,
        })
        .unwrap(),
        serde_json::to_value(TimezoneResultEnvelope::<TimezoneZone> {
            status: TimezoneResultStatus::AmbiguousLocalTime,
            data: None,
            page: None,
            error: None,
        })
        .unwrap(),
    ];

    assert!(values.iter().all(|value| value.is_object()));
}

#[test]
fn location_descriptor_hashes_are_stable_and_distinct() {
    let hash_groups = [
        hash_values(&location_maps_descriptor_hashes()),
        hash_values(&location_geocode_descriptor_hashes()),
        hash_values(&location_route_descriptor_hashes()),
        hash_values(&location_place_search_descriptor_hashes()),
        hash_values(&location_timezone_descriptor_hashes()),
    ];

    for hashes in hash_groups {
        let unique = hashes.into_iter().collect::<BTreeSet<_>>();
        assert!(unique.len() >= 8);
        assert!(unique.iter().all(|hash| !hash.is_empty()));
    }
}

#[test]
fn location_validation_helpers_are_provider_neutral() {
    let matrix = TileMatrixDescriptor {
        min_zoom: 1,
        max_zoom: 20,
        tile_size_px: 256,
        ..Default::default()
    };
    let viewport = MapViewport {
        width_px: 1200,
        height_px: 800,
        ..Default::default()
    };
    let geocode_query = GeocodeQuery {
        structured_component_refs: vec!["component:1".into(), "component:2".into()],
        ..Default::default()
    };
    let route = RoutePlan {
        waypoints: vec![RouteWaypoint::default(), RouteWaypoint::default()],
        alternatives_requested: 1,
        ..Default::default()
    };
    let place_query = PlaceQuery {
        field_mask: BTreeSet::from(["summary".into(), "category".into()]),
        page_size: 10,
        ..Default::default()
    };
    let local_resolution = TimezoneLocalResolution {
        resolver_strategy: "compatible".into(),
        ..Default::default()
    };

    assert!(matrix.has_valid_zoom_bounds());
    assert!(viewport.is_bounded(1_000_000));
    assert!(geocode_query.is_bounded(4));
    assert!(route.is_bounded(4, 2));
    assert!(place_query.is_bounded(8, 20));
    assert!(local_resolution.has_explicit_strategy());
}

#[test]
fn invalid_location_descriptor_is_rejected() {
    let mut invalid = location_maps_pack_definition();
    invalid.pack_id = "location.maps.v1".into();

    assert!(DomainPackDefinitionSpec.validate(&invalid).is_err());
}

fn hash_values<T: serde::Serialize>(value: &T) -> Vec<String> {
    let json = serde_json::to_value(value).expect("descriptor hash fixture serializes");
    json.as_object()
        .expect("descriptor hashes serialize as object")
        .values()
        .filter_map(|value| value.as_str().map(str::to_string))
        .collect()
}

fn find_pack<'a>(
    definitions: &'a [DomainPackDefinition],
    pack_id: &str,
) -> &'a DomainPackDefinition {
    definitions
        .iter()
        .find(|definition| definition.pack_id == pack_id)
        .expect("industrial catalog includes specialized location descriptor")
}
