# Location Maps Pack Research

## Purpose

This note records supplier/API research, Macaca provider-neutral mapping,
boundary decisions, existing platform inventory, and GitNexus memo evidence for
`pack.location.maps.v1`. The maps pack owns map style discovery, tile matrix
metadata, tile references, viewport validation, annotation and overlay planning,
static map rendering, attribution bundles, cache inspection, map artifacts,
freshness, and redaction through typed service commands. It must not own
geocoding, route calculation, place search, timezone lookup, device location
capture, image processing beyond map render artifacts, or application UI map
widgets.

## Source Baseline

- Google Maps Platform tiles, static maps, map styles, and attribution:
  <https://developers.google.com/maps/documentation/tile>,
  <https://developers.google.com/maps/documentation/maps-static>, and
  <https://developers.google.com/maps/documentation/>
- Mapbox Maps, Static Images, styles, tiles, and attribution:
  <https://docs.mapbox.com/api/maps/>
- HERE Maps and Map Tile APIs:
  <https://www.here.com/docs/bundle/map-tile-api-developer-guide/page/README.html>
- Azure Maps Render service:
  <https://learn.microsoft.com/en-us/rest/api/maps/render>
- Esri ArcGIS maps, layers, and tile services:
  <https://developers.arcgis.com/rest/services-reference/enterprise/map-service/>
- Apple MapKit and MapKit JS:
  <https://developer.apple.com/documentation/mapkit> and
  <https://developer.apple.com/documentation/mapkitjs>
- OpenStreetMap tile usage policy:
  <https://operations.osmfoundation.org/policies/tiles/>
- OpenLayers and Leaflet layer abstractions:
  <https://openlayers.org/en/latest/apidoc/> and
  <https://leafletjs.com/reference.html>

## Supplier API Notes

- Google, Mapbox, HERE, Azure Maps, and Esri expose style catalogs, tile
  formats, static rendering, zoom bounds, imagery/vector support, attribution,
  quotas, and caching constraints. Macaca should normalize these as descriptor
  capabilities and attribution bundles, not as provider-native URL templates.
- Apple MapKit and MapKit JS contribute host/native and web map rendering
  semantics. Macaca should keep UI widgets and native rendering outside the OS
  contract while allowing bounded static render artifacts.
- OpenStreetMap tile policies emphasize attribution, usage limits, cache
  behavior, and operational fairness. Macaca should make attribution and
  resource-budget checks first-class policy/resource gates.
- OpenLayers and Leaflet provide useful layer/source/viewport vocabulary, but
  they are UI libraries. Macaca should borrow layer abstraction concepts without
  embedding UI rendering semantics into the OS pack.

## Macaca-Owned Abstractions

`pack.location.maps.v1` should define `MapScope`, `MapStyleReference`,
`TileMatrixDescriptor`, `MapTileCoordinate`, `MapTileReference`,
`MapViewport`, `MapAnnotation`, `MapOverlay`, `StaticMapRenderRequest`,
`MapAttributionBundle`, `MapCacheStatus`, `MapArtifactHandle`,
`MapFreshness`, and `MapRedactionPolicy`.

The DTOs must carry style/version references, tile matrix/projection metadata,
zoom bounds, coordinate precision class, viewport bounds, overlay geometry
limits, render dimensions, pixel ratio, cache freshness, attribution text/link
requirements, provider attribution, artifact checksum, retention policy,
redaction class, and replay pointers. Raw API keys, raw provider tile payloads,
private overlays, unbounded tile dumps, provider-specific URL templates, and UI
widget state are rejected.

## Boundary Decisions And Non-Goals

- Geocode owns address/coordinate conversion.
- Route owns path, ETA, matrix, and optimization calculation.
- Place-search owns POI discovery and details.
- Timezone owns zone lookup and offset/transition semantics.
- Device owns location sensor capture.
- Media owns image processing beyond map-render artifacts.
- Applications own interactive map widgets, gestures, business overlays, and
  product-specific map workflows.

## Existing Macaca Platform Inventory

- `crates/foundation/macaca-proto/src/domain_pack_contract/` provides reusable
  descriptor metadata, lifecycle/availability, policy templates, diagnostics,
  SDK metadata, provider snapshots, unavailable diagnostics, and effective
  capability expansion.
- `crates/facade/macaca-sdk/src/system_facade.rs` provides the Facade pattern;
  maps SDK helpers should only build canonical traced service calls.
- Generic policy, approval, resource, entitlement, trace, audit, artifact,
  cache, mock-provider, and unavailable-provider concepts are reusable, but
  current evidence does not prove maps-specific DTOs, descriptors, providers,
  SDK helpers, ABI metadata, tests, or developer docs.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
