# Location Maps Pack Design

## Context

`pack.location.maps.v1` is a child proposal of the developer-pack industrial
capability catalog. It provides map rendering resources as a serviceized
capability: map tiles, style references, viewports, camera state, annotations,
overlays, static map rendering, attribution bundles, cache/offline metadata, and
map artifact handles.

Map providers expose rendering capabilities through very different surfaces:
Google Map Tiles and Static Maps, Mapbox styles and tilesets, HERE raster/vector
tiles, Azure Maps render endpoints, Esri basemaps and map image services, Apple
MapKit snapshots and overlays, OpenStreetMap-compatible tiles, and
OpenLayers/Leaflet-style layer abstractions. Macaca needs a provider-neutral
contract so applications can request map resources without learning provider
credentials, billing rules, attribution quirks, or client SDK internals.

## Supplier Capability Matrix

| Supplier or ecosystem | Relevant capability | Macaca interpretation |
| --- | --- | --- |
| Google Maps Platform | Map Tiles API, Static Maps API, Map IDs/styles, imagery, attribution | Tile references, static render requests, style references, attribution bundles |
| Mapbox | Styles, tilesets, vector/raster tiles, Static Images, sprites/glyphs, offline packs | Style references, tile source references, static artifacts, cache/offline metadata |
| HERE Maps | Raster/vector tile APIs, map image rendering, styles, traffic overlays, copyright metadata | Tile references, overlay references, render requests, attribution bundles |
| Azure Maps | Render tiles, static images, map styles, copyright endpoints, traffic/weather overlays | Tile/render commands, attribution inspection, overlay capability metadata |
| Esri ArcGIS | Basemaps, vector tile services, map image/export services, feature/graphics layers, spatial references | Layer/overlay references, tile matrix/spatial reference metadata, static render artifacts |
| Apple MapKit / MapKit JS | Map views, camera, annotations, overlays, snapshots, tile overlays | Viewport/camera, annotations, overlays, snapshot artifact model; UI widgets stay app-side |
| OpenStreetMap tile ecosystem | Slippy-map tiles, usage policy, attribution, tile server replacement | Open tile coordinate scheme, attribution enforcement, provider replacement pattern |
| OpenLayers / Leaflet | Layers, sources, projections, features, overlays, view state | API-shape reference for DTOs; library-specific rendering/UI is not OS semantics |

## Goals

- Provide stable pack id `pack.location.maps.v1` and command namespace
  `maps.*`.
- Normalize style references, tile matrix sets, tile references, viewport/camera
  state, annotations, overlays, static render requests, attribution bundles,
  cache/offline status, and map artifact handles.
- Support provider inspection, style catalog discovery, tile matrix discovery,
  tile retrieval, viewport validation, annotation/overlay planning, static
  render planning/request, attribution inspection, cache status, and artifact
  retrieval through typed command/result DTOs.
- Preserve a single canonical execution path through SDK/facade clients,
  service runtime decorators, and replaceable maps service providers.
- Return structured `success`, `partial`, `approval_required`, `denied`,
  `unavailable`, `unsupported`, `conflict`, `stale_version`,
  `quota_exceeded`, `rate_limited`, `timeout`, `cancelled`, and `failure`
  results.
- Emit sanitized trace, audit, health, snapshot, and replay evidence for every
  declaration, admission, policy decision, service call, provider decision, and
  unavailable state.
- Require detailed developer documentation at
  `docs/developer-packs/location/maps.md`.

## Non-Goals

- No geocoding, reverse geocoding, route calculation, place search, timezone
  lookup, device sensor/location capture, navigation workflow, trip planning,
  fleet optimization, emergency workflow, or application-specific map UI.
- No embedded client map widget, DOM/canvas renderer, or native UI component.
  Applications own presentation and may use returned artifacts/references.
- No provider-specific billing policy, cartography workflow, provider style
  branching, or map SDK initialization in kernel, SDK, shells, or generic app
  framework.
- No raw API keys, tokens, credentials, private style payloads, raw provider
  responses, unbounded tile dumps, private manifests, package bytes, private
  keys, signatures, or unsanitized location-derived data in observability
  surfaces.

## Ownership And Boundaries

- Pack id: `pack.location.maps.v1`.
- Family: `location`.
- Backing service owner: replaceable maps service provider.
- SDK surface: `sdk.packs.location.maps`.
- Command namespace: `maps.*`.
- Microkernel ownership: service-call evidence, policy facade, resource facade,
  trace/audit primitives, and scheduling primitives only.
- Application framework ownership: manifest declaration, app-scoped permission
  declarations, lifecycle projection, and effective capability mementos.
- Runtime-host ownership: provider registration, service runtime decorators,
  transport adapters, health/snapshot bridge, and unavailable/mock provider
  composition through approved composition roots.

## Command Surface

All commands carry trace context, application/session/task/tenant identifiers
when available, policy context, idempotency key for rendered artifacts, redaction
profile, resource budget, and replay metadata.

| Command | Purpose | Notes |
| --- | --- | --- |
| `maps.inspect_provider` | Return provider capability metadata | Reports tile/style/render/overlay/attribution/cache support, rate limits, health, and unavailable reasons |
| `maps.discover_styles` | List style references and constraints | Returns style ids/handles, theme class, language support, imagery support, and attribution requirements |
| `maps.discover_tile_matrix` | Inspect tile coordinate systems and formats | Returns scheme, zoom bounds, tile size, spatial reference, formats, retina scale, and cache policy |
| `maps.plan_tile_request` | Validate tile retrieval without side effects | Checks coordinate, zoom, style, layer, format, region policy, quota, cache, and attribution |
| `maps.get_tile` | Retrieve or reference one bounded map tile | Returns tile artifact handle, cache status, freshness, attribution, and redaction metadata |
| `maps.validate_viewport` | Validate viewport/camera state | Checks center, bounds, zoom, pitch, bearing, projection, precision, and provider support |
| `maps.plan_annotation` | Validate markers/annotations | Checks coordinate precision, payload redaction, label size, icon reference, and policy |
| `maps.plan_overlay` | Validate overlays/layers | Checks geometry bounds, layer type, feature count, style reference, policy, and provider support |
| `maps.plan_static_render` | Validate static map rendering | Checks viewport, style, overlays, dimensions, pixel ratio, attribution, resource budget, and output format |
| `maps.render_static_map` | Render or request a static map artifact | Requires idempotency, timeout/cancellation, artifact metadata, and attribution bundle |
| `maps.inspect_attribution` | Return attribution/copyright requirements | Provides data-source notices, logos, copyright text references, and usage-policy metadata |
| `maps.inspect_cache` | Inspect cache/offline status | Returns tile/style/artifact cache metadata, staleness, retention, and offline support |
| `maps.get_artifact` | Retrieve map artifact metadata | Returns handles for tile/static render artifacts without raw provider payload leakage |

## Provider-Neutral DTO Model

- `MapScope`: application id, tenant id, session/task identifiers, region policy
  reference, provider reference, and trace context.
- `MapStyleReference`: style handle, theme class, layer set, imagery class,
  language support, provider capability hash, attribution requirement, version,
  and freshness.
- `TileMatrixDescriptor`: coordinate scheme, tile size, zoom range, spatial
  reference, axis order, bounds, scale set, supported formats, and retina scale.
- `MapTileCoordinate`: z/x/y, quadkey, tile matrix position, bounding box, or
  provider-neutral coordinate with spatial reference metadata.
- `MapTileReference`: tile handle, coordinate, style, layer, format, cache
  state, checksum/hash, attribution bundle, freshness, and artifact handle.
- `MapViewport`: center coordinate, bounds, zoom, pitch, bearing, projection,
  width, height, device pixel ratio, precision class, and language/locale hint.
- `MapAnnotation`: coordinate, icon reference, label reference, payload
  reference, z-order, interaction metadata, redaction class, and policy hints.
- `MapOverlay`: geometry reference, layer type, style reference, feature count,
  bounds, source reference, opacity, z-order, data sensitivity, and attribution.
- `StaticMapRenderRequest`: viewport, style, annotations, overlays, dimensions,
  pixel ratio, output format, attribution mode, idempotency key, and artifact
  retention.
- `MapAttributionBundle`: copyright text reference, data source notices, logo
  requirement, link requirement, provider terms reference, and display rules.
- `MapCacheStatus`: cache key hash, availability, freshness, stale reason,
  offline support, retention, eviction hint, and quota state.
- `MapArtifactHandle`: artifact id, content class, format, size class,
  checksum/hash, redaction state, attribution bundle, retention deadline, and
  retrieval permissions.

## Permission, Policy, Resource, Entitlement, And Approval Model

Initial permission scopes:

- `location.maps.read`
- `location.maps.tile.read`
- `location.maps.style.read`
- `location.maps.viewport.validate`
- `location.maps.annotation.plan`
- `location.maps.overlay.plan`
- `location.maps.render`
- `location.maps.attribution.read`
- `location.maps.cache.read`
- `location.maps.artifact.read`

Policy checks run before side effects and before provider calls that could
reveal sensitive map or location-derived data. Policy inputs include caller
subject, application id, tenant id, command, requested fields, coordinate
precision class, region/residency policy, style/layer type, overlay sensitivity,
render dimensions, network/cache mode, attribution requirement, resource budget,
approval state, and entitlement state.

Approval is required when policy marks a map operation as sensitive, such as
rendering precise private coordinates, exporting retained artifacts, retrieving
high-volume tiles, using externally hosted private overlays, or crossing
regulated region/data-boundary policies.

Resource checks cover tile count, tile size, render dimensions, overlay feature
count, geometry complexity, artifact size, cache retention, provider quota,
network budget, timeout, retained snapshots, and event volume.

Entitlement checks determine whether the calling application/tenant may use the
pack, requested commands, premium styles, imagery layers, traffic/weather
overlays, offline/cache features, static rendering, and retained artifacts.

## Service Runtime And Provider Strategy

The maps service provider is a Strategy behind the service runtime. The runtime
composes provider adapters, unavailable providers, mock providers, policy
decorators, resource decorators, entitlement decorators, metering, redaction,
attribution enforcement, trace, audit, timeout/cancellation, and
health/snapshot behavior.

Provider adapters may target Google Maps, Mapbox, HERE, Azure Maps, Esri,
Apple MapKit, OpenStreetMap-compatible tile sources, OpenLayers/Leaflet-style
tile source bridges, offline tile bundles, built-in local providers, remote
providers, plugin providers, or mock providers. Provider-specific capabilities
are descriptor data, not OS routing branches.

The unavailable provider is first-class. It exposes descriptor metadata, health
state, unsupported command diagnostics, and stable error DTOs without crashing,
hanging, silently falling back, contacting undeclared providers, or faking
success.

## State, Consistency, And Idempotency

Tile references, style references, static render artifacts, cache entries, and
attribution bundles have explicit freshness and version metadata. Static render
requests require idempotency keys and must support timeout/cancellation. Tile
and artifact results must distinguish live provider data, cached data, stale
cached data, unavailable data, and unsupported formats.

Attribution is not optional. Commands that return tiles, static renders,
overlays, or artifacts must carry `MapAttributionBundle` metadata or return a
typed unsupported/unavailable result when attribution cannot be produced.

## SDK Discovery And Developer Documentation

SDK discovery must return pack metadata, command schemas, permission scopes,
style catalog metadata, tile matrix support, supported formats, projection
support, static render limits, overlay support, attribution requirements, cache
support, examples, availability, diagnostics, provider class, compatibility
hash, redaction profile, and documentation link.

SDK helper builders only build canonical traced service calls. They must never
construct providers, hold credentials, call provider APIs directly, geocode
addresses, calculate routes, search places, capture device location, render UI
widgets, or bypass attribution/policy.

Developer documentation at `docs/developer-packs/location/maps.md` must cover:

- Capability purpose and non-goals.
- Manifest declaration examples for required and optional usage.
- Permission scopes and approval behavior.
- Command DTOs and result DTOs with field-level explanations.
- Style, tile matrix, tile, viewport, annotation, overlay, static render,
  attribution, cache, artifact, version, and freshness models.
- Supplier/API mapping and provider replacement guidance.
- Unavailable/denied/conflict/stale-version/quota diagnostics.
- Trace/audit events, redaction rules, snapshot/replay behavior, attribution
  conformance, and provider-author checklist.

## Trace, Audit, Health, Snapshot, And Replay

Required event families:

- `maps_pack_declared`
- `maps_pack_admission_validated`
- `maps_pack_discovery_requested`
- `maps_pack_policy_decision`
- `maps_pack_resource_reserved`
- `maps_pack_approval_required`
- `maps_pack_service_call_requested`
- `maps_pack_service_call_succeeded`
- `maps_pack_service_call_failed`
- `maps_pack_unavailable`
- `maps_pack_attribution_resolved`
- `maps_pack_snapshot_recorded`
- `maps_pack_artifact_created`

Events include pack id, descriptor version, command name, trace id,
application/session/task/tenant identifiers when available, style handle hash,
tile coordinate hash or viewport hash, policy decision, approval state, provider
class, latency, bounded resource counters, capability hash, attribution bundle
hash, and bounded error code.

Events, snapshots, SDK diagnostics, and examples must exclude raw credentials,
API keys, access tokens, private style payloads, raw provider responses,
unbounded tile dumps, private manifests, package bytes, private keys,
signatures, and unsanitized location-derived data.

Snapshots include descriptor version, provider capability hash, command
availability, provider health, style catalog hash, tile matrix hash, attribution
hash, cache summary, resource counters, artifact summaries, event cursors, and
sanitized replay pointers.

## Design Patterns

- **Facade**: `SystemFacade` and focused SDK clients expose discovery and typed
  command builders while hiding service runtime and provider composition.
- **Command**: every operation is represented as a typed command/result DTO
  with explicit success, partial, denied, unavailable, unsupported, conflict,
  stale-version, quota, approval-required, timeout, cancelled, and failure
  variants.
- **Adapter/Bridge**: Google, Mapbox, HERE, Azure, Esri, Apple, OSM-compatible,
  OpenLayers/Leaflet-style, offline, built-in, plugin, remote, mock, and
  unavailable providers adapt into the same contract.
- **Strategy**: provider selection, style mapping, tile matrix mapping, render
  behavior, attribution behavior, cache behavior, and unavailable behavior are
  replaceable.
- **Decorator**: trace, audit, policy, resource, entitlement, approval,
  metering, timeout, cancellation, attribution, and redaction wrap every call.
- **State**: tile freshness, style version, cache state, static render artifact
  lifecycle, and provider lifecycle states are explicit and replayable.
- **Observer**: trace, audit, health, and service events are subscribable by
  shells without giving shells semantic ownership.
- **Memento**: effective capability reports, snapshots, provider capability
  hashes, tile matrix hashes, attribution hashes, and audit cursors preserve
  bounded recovery state.
- **Specification**: admission validates pack id, command availability,
  permission scopes, provider health, entitlement, resource budgets,
  attribution requirements, and policy templates.
- **Abstract Factory**: concrete provider adapters are constructed only in
  approved composition roots.

## Risks And Mitigations

- Risk: maps pack becomes geocoding, routing, or place search. Mitigation:
  coordinates, tiles, styles, overlays, and render artifacts only; adjacent
  location packs own geocode, route, place search, and timezone.
- Risk: provider attribution is lost. Mitigation: attribution bundle is required
  for every tile/render/artifact result and covered by conformance tests.
- Risk: SDK helpers become provider SDK wrappers. Mitigation: helpers only build
  canonical service commands and never hold credentials or initialize map SDKs.
- Risk: UI/widget semantics leak into OS. Mitigation: Macaca returns resources
  and artifacts; applications own presentation.
- Risk: tile or render requests leak private location data. Mitigation:
  coordinate precision, region policy, redaction, and approval gates run before
  provider calls.
