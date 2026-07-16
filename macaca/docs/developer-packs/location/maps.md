# Location Maps Pack

`pack.location.maps.v1` provides provider-neutral map style discovery, tile
matrix discovery, tile-reference planning, bounded tile retrieval, viewport
validation, annotation and overlay planning, static-map render planning,
attribution inspection, cache inspection, and artifact handle discovery.

The pack is descriptor-only until a serviceized maps provider is registered by
the runtime composition root. Applications declare the pack; they do not hold
map provider credentials, call tile services directly, or render application UI
through the OS contract.

## Manifest Declaration

Required declarations block readiness when the pack is unavailable. Optional
declarations degrade with `location_maps_provider_not_installed` and keep the
effective capability memento traceable.

```toml
[service_contract]
optional_packs = ["pack.location.maps.v1"]
```

## Commands

- `maps.inspect_provider`: returns provider-class capability metadata.
- `maps.discover_styles`: returns `MapStyleReference` rows.
- `maps.discover_tile_matrix`: returns `TileMatrixDescriptor` rows.
- `maps.plan_tile_request`: validates tile scope without retrieving tiles.
- `maps.get_tile`: returns bounded `MapTileReference` metadata, not raw tile
  bytes.
- `maps.validate_viewport`: validates `MapViewport` dimensions, zoom, and
  spatial reference.
- `maps.plan_annotation`: plans `MapAnnotation` records with redaction class.
- `maps.plan_overlay`: plans `MapOverlay` records by geometry reference.
- `maps.plan_static_render` and `maps.render_static_map`: plan and request a
  static render artifact through idempotent service calls.
- `maps.inspect_attribution`: returns `MapAttributionBundle` obligations.
- `maps.inspect_cache`: returns `MapCacheStatus`.
- `maps.get_artifact`: returns `MapArtifactHandle`.

## DTOs And Results

Core DTOs include `MapScope`, `MapStyleReference`, `TileMatrixDescriptor`,
`MapTileCoordinate`, `MapTileReference`, `MapViewport`, `MapAnnotation`,
`MapOverlay`, `StaticMapRenderRequest`, `MapAttributionBundle`,
`MapCacheStatus`, and `MapArtifactHandle`. Result envelopes use typed statuses
including success, partial, approval-required, denied, unavailable,
unsupported, conflict, stale-version, quota, rate-limited, timeout, cancelled,
attribution-missing, cache-stale, and failure.

## Field Notes

`MapScope` carries tenant, region, precision, and retention classes. Style and
tile matrix references carry stable ids, provider class, projection, supported
formats, zoom bounds, and compatibility hashes. Tile coordinates include matrix
id, x/y/zoom, scale, and spatial reference; commands reject invalid zoom or
projection combinations before provider calls. Viewports carry center, extent,
dimensions, pixel ratio, and precision class. Render requests carry idempotency
key, style ref, viewport ref, overlay refs, output format, size, artifact
retention, attribution requirement, approval ref, and redaction profile. Cache
and artifact DTOs carry freshness, expiry, checksum, replay pointer, and
bounded retention metadata. Structured errors include denied, unavailable,
unsupported, stale-version, attribution-missing, cache-stale, quota,
rate-limited, timeout, cancelled, and artifact-denied diagnostics.

## Provider Mapping

Google Maps Platform, Mapbox, HERE Maps, Azure Maps, Esri ArcGIS, Apple MapKit
or MapKit JS, OpenStreetMap tile-policy deployments, OpenLayers, and Leaflet
concepts map into styles, tile matrices, tile references, viewports, overlays,
static render plans, attribution bundles, and cache metadata. Supplier-specific
style payloads, raw tiles, JavaScript UI widgets, credentials, and proprietary
provider response bodies are intentionally not OS semantics.

## App-Facing Examples

Applications declare `pack.location.maps.v1` as required when map output is a
readiness dependency, or optional when the UI can degrade with
`location_maps_provider_not_installed`. All calls pass through typed SDK
commands with synthetic scope, style, tile, viewport, overlay, cache, and
artifact refs.

- Discover styles and tile matrices before requesting any tile references.
- Retrieve a bounded tile with `maps.get_tile`, store only
  `MapTileReference`, and never log raw tile bytes.
- Validate a viewport before planning markers, annotations, overlays, or static
  render requests.
- Plan and request a static render through idempotent render refs, then inspect
  attribution, cache freshness, and artifact handles.
- Handle unavailable provider, denied permission, missing entitlement,
  attribution missing, unsupported style, unsupported tile format,
  render-size denied, precise-location approval, stale cache, quota exceeded,
  network denied, and artifact denied with provider-neutral diagnostics.

## Conformance

Provider authors must prove descriptor completeness, tile/style/render scope
validation, idempotency, tile matrix compatibility, attribution completeness,
coordinate precision enforcement, cache freshness, resource bounds, policy
hooks, sanitized trace and audit events, unavailable behavior, snapshot/replay,
and no raw payload leakage.
