# Change: Add Location Maps Pack

## Why

Macaca applications need `pack.location.maps.v1` as an industrial maps
capability for map tiles, map styles, viewports, camera state, overlays,
markers, static map rendering, attribution, copyright notices, offline/cache
metadata, and map artifact handles. Mature providers expose these capabilities
through raster tiles, vector tiles, static image APIs, map style IDs, basemap
layers, annotation/overlay SDKs, copyright endpoints, tile matrix sets, and
usage restrictions. Macaca must normalize the map-rendering boundary without
becoming a geocoder, router, place-search engine, device-location provider, or
provider-specific map SDK.

This proposal defines maps as a serviceized, provider-neutral pack. It gives
applications typed map commands while keeping concrete Google Maps, Mapbox,
HERE, Esri, Apple MapKit, Azure Maps, OpenStreetMap-compatible, OpenLayers-style,
offline tile, and unavailable providers behind replaceable service providers.

## Supplier And API Baseline

The design is based on mature map rendering APIs:

- Google Maps Platform exposes Map Tiles API, Static Maps API, Map IDs/styles,
  Street View imagery, map controls, markers, overlays, and required
  attribution/copyright behavior.
- Mapbox exposes Maps APIs, Static Images API, Tilesets, vector/raster tiles,
  style documents, sprites/glyphs, annotations, offline packs, and attribution
  requirements.
- HERE Maps exposes raster/vector tile APIs, map image/static rendering,
  styles, map view parameters, traffic overlays, and copyright/attribution
  metadata.
- Azure Maps exposes Render APIs for tiles and static map images, map styles,
  copyright endpoints, traffic/weather overlays, and authentication/usage
  constraints.
- Esri ArcGIS exposes basemap layers, vector tile services, map image/export
  services, feature/graphics layers, web maps, spatial references, and
  attribution requirements.
- Apple MapKit and MapKit JS expose map views, annotations, overlays, camera
  state, snapshots, tile overlays, and Apple-specific usage/attribution
  constraints.
- OpenStreetMap tile ecosystems expose slippy-map tile conventions,
  attribution requirements, tile usage policies, and open tile server
  replacement patterns.
- OpenLayers and Leaflet are useful client-library references for layer,
  viewport, feature, overlay, projection, and tile-source abstractions, but
  Macaca must not copy library-specific UI semantics into OS contracts.

Research references:

- Google Maps Platform Map Tiles and Static Maps:
  https://developers.google.com/maps/documentation/tile and
  https://developers.google.com/maps/documentation/maps-static
- Mapbox Maps, Static Images, and Tilesets:
  https://docs.mapbox.com/api/maps/ and
  https://docs.mapbox.com/api/maps/static-images/ and
  https://docs.mapbox.com/api/maps/tilesets/
- HERE Maps APIs:
  https://www.here.com/docs/category/maps and
  https://www.here.com/docs/bundle/raster-tile-api-developer-guide/page/README.html
- Azure Maps Render and copyright APIs:
  https://learn.microsoft.com/azure/azure-maps/how-to-render-custom-data and
  https://learn.microsoft.com/rest/api/maps/render
- Esri ArcGIS maps, vector tiles, and export map services:
  https://developers.arcgis.com/documentation/mapping-apis-and-services/maps/
  and
  https://developers.arcgis.com/rest/services-reference/enterprise/export-map/
- Apple MapKit and MapKit JS:
  https://developer.apple.com/documentation/mapkit and
  https://developer.apple.com/documentation/mapkitjs
- OpenStreetMap tile usage and attribution:
  https://operations.osmfoundation.org/policies/tiles/ and
  https://www.openstreetmap.org/copyright
- OpenLayers map/layer/source model:
  https://openlayers.org/doc/

## Macaca Provider-Neutral Mapping

`pack.location.maps.v1` maps supplier concepts into stable Macaca contracts:

- Raster tiles, vector tiles, slippy-map tiles, tile matrix set tiles, map image
  tiles, and offline tiles become `MapTileReference` with coordinate scheme,
  zoom, layer, format, cache policy, attribution, and freshness metadata.
- Google Map IDs, Mapbox style URLs, HERE/Esri/Azure style names, basemap layer
  IDs, and custom style documents become `MapStyleReference`; raw provider
  style payloads are optional artifacts and not OS routing branches.
- Map camera, center/zoom, pitch/bearing, bounding boxes, spatial references,
  device pixel ratio, and viewport constraints become `MapViewport`.
- Markers, annotations, pins, graphics, feature highlights, popups, and labels
  become `MapAnnotation` with redacted payload references and display metadata.
- Polygons, polylines, circles, heatmaps, traffic/weather overlays, feature
  layers, tile overlays, and data-driven overlays become `MapOverlay`.
- Static maps, snapshots, exported map images, thumbnails, and rendered map
  artifacts become `StaticMapRenderRequest` and `MapArtifactHandle`.
- Provider copyright endpoints, attribution strings, logo rules, data source
  notices, and tile usage policies become `MapAttributionBundle`.
- Cache entries, offline packs, tile availability, stale tile metadata, and
  quota/rate-limit state become `MapCacheStatus` and provider capability
  diagnostics.

## What Changes

- Add provider-neutral `pack.location.maps.v1` under the location family.
- Define commands for provider inspection, style catalog discovery, tile matrix
  discovery, tile retrieval planning/request, viewport validation, annotation
  planning, overlay planning, static render planning/request, attribution
  inspection, cache/offline status inspection, artifact retrieval, and provider
  health/snapshot behavior.
- Define DTOs for map scope, provider capability, style references, tile
  references, tile coordinates, tile matrix sets, viewport/camera, spatial
  references, annotations, overlays, static render requests, attribution
  bundles, cache status, freshness/version metadata, redaction, and artifact
  handles.
- Require policy, attribution enforcement, coordinate precision controls,
  network/cache/resource checks, entitlement checks, idempotency for rendered
  artifacts, sanitized trace/audit, and deterministic unavailable/unsupported
  behavior.
- Require detailed developer documentation at
  `docs/developer-packs/location/maps.md`.

## Impact

- Affected specs: `pack-location-maps`,
  `developer-pack-industrial-capability-catalog`, `sdk-system-facade`,
  `service-runtime`, `unified-execution-path`.
- Affected code later: protocol DTOs, pack descriptors, admission validators,
  SDK discovery/command builders, maps service providers,
  mock/unavailable providers, trace/audit schemas, replay tests, redaction
  tests, attribution tests, cache/quota tests, and boundary gates.

## Non-Goals

- No geocoding, reverse geocoding, route calculation, place search, timezone
  lookup, device sensor/location capture, navigation workflow, trip planning,
  fleet business logic, emergency dispatch, or application-specific map UI.
- No provider-specific map style policy, tile billing policy, cartography
  workflow, client rendering engine, or map SDK widget implementation in Macaca
  OS layers.
- No raw credentials, API keys, access tokens, private style payloads, raw
  provider responses, raw manifests, package bytes, private keys, signatures,
  unbounded tile dumps, or unsanitized location-derived data in logs, traces,
  snapshots, or SDK diagnostics.
- No provider construction or provider-name routing in kernel, SDK, shells, or
  generic application framework.
