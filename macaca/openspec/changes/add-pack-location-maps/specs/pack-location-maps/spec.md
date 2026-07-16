## ADDED Requirements

### Requirement: Macaca SHALL provide Location Maps Pack as a serviceized capability

Macaca SHALL provide `pack.location.maps.v1` as a provider-neutral industrial
pack for map style references, tile matrix descriptors, map tiles, viewports,
annotations, overlays, static map rendering, attribution bundles, cache/offline
metadata, and map artifact handles. The pack SHALL be declared by applications,
resolved by admission/catalog services, and invoked only through typed service
commands.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.location.maps.v1` as required and a maps service provider is registered, healthy, entitled, permission-compatible, policy-admissible, and attribution-capable
- **THEN** admission SHALL expose the pack in the effective capability set with command schemas, permission scopes, policy template hash, resource limits, approval rules, attribution metadata, provider health metadata, compatibility metadata, documentation links, and replay metadata
- **AND** SDK discovery SHALL mark callable commands as available without exposing provider secrets, API keys, access tokens, private style payloads, raw provider payloads, unbounded tile data, or unsanitized location-derived data

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.location.maps.v1` as required but provider, permission, entitlement, resource, host support, network support, attribution support, style support, tile support, or render support is absent
- **THEN** admission SHALL block readiness with structured unavailable or denied diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back, contact another undeclared provider, return unattributed tiles, or fake success

#### Scenario: Optional declaration is unavailable
- **WHEN** an application declares `pack.location.maps.v1` as optional and the pack is unavailable
- **THEN** admission SHALL produce an explicit degraded effective capability report with unavailable reason codes and command-level availability
- **AND** SDK command helpers SHALL refuse to build callable service calls for unavailable commands

### Requirement: Location Maps Pack SHALL expose supplier-grade map contracts

`pack.location.maps.v1` SHALL expose provider-neutral DTOs for map scopes,
style references, tile matrix descriptors, tile coordinates, tile references,
viewports, annotations, overlays, static render requests, attribution bundles,
cache status, artifacts, version metadata, freshness metadata, redaction
metadata, and provider capability metadata.

#### Scenario: Provider schema is discovered
- **WHEN** SDK discovery or `maps.inspect_provider` inspects the pack
- **THEN** Macaca SHALL return command schemas, permission scopes, style support, tile matrix support, tile formats, projection/spatial-reference support, static render limits, overlay support, attribution requirements, cache support, lifecycle state, provider health, redaction profile, and compatibility hash
- **AND** the schema SHALL be provider-neutral even when backed by Google Maps, Mapbox, HERE, Azure Maps, Esri, Apple MapKit, OpenStreetMap-compatible, OpenLayers/Leaflet-style, offline, built-in, plugin, remote, mock, or unavailable providers

#### Scenario: Style catalog is discovered
- **WHEN** `maps.discover_styles` is invoked for a declared and policy-allowed pack
- **THEN** Macaca SHALL return bounded `MapStyleReference` records with style handles, theme class, layer set, imagery class, language support, provider capability hash, attribution requirement, version, and freshness
- **AND** raw provider style payloads SHALL be returned only through bounded artifact handles when explicitly permitted

#### Scenario: Tile matrix is discovered
- **WHEN** `maps.discover_tile_matrix` is invoked
- **THEN** Macaca SHALL return coordinate scheme, tile size, zoom range, spatial reference, axis order, bounds, scale set, supported formats, and retina scale
- **AND** unsupported coordinate schemes, formats, or zoom ranges SHALL be represented as typed unsupported diagnostics

### Requirement: Location Maps Pack commands SHALL use canonical typed service calls

Every `maps.*` operation SHALL be represented as a typed command/result DTO and
SHALL traverse the canonical service runtime path with trace, policy, resource,
entitlement, approval, health, snapshot, timeout, cancellation, attribution,
idempotency, redaction, and structured error behavior.

#### Scenario: Tile request is planned
- **WHEN** `maps.plan_tile_request` validates a tile retrieval request
- **THEN** Macaca SHALL check coordinate scheme, zoom bounds, style, layer, format, region policy, precision policy, entitlement, resource budget, cache policy, attribution availability, and provider capability
- **AND** no provider side effect SHALL occur during the plan command

#### Scenario: Tile is retrieved
- **WHEN** `maps.get_tile` is invoked for a declared, planned, and policy-allowed tile request
- **THEN** Macaca SHALL route the command through SDK/facade helpers into the service runtime and maps service provider
- **AND** the result SHALL include tile handle or artifact handle, tile coordinate metadata, cache state, freshness, checksum/hash, attribution bundle, and redaction metadata

#### Scenario: Tile request is denied before provider call
- **WHEN** policy, permission, entitlement, approval, resource, network, coordinate precision, attribution, or provider capability checks reject a tile request
- **THEN** Macaca SHALL return a typed denied, approval-required, unavailable, unsupported, quota, or attribution-missing result before invoking the concrete provider
- **AND** the audit trail SHALL include bounded reason codes, hashes, counters, and sanitized references only

### Requirement: Location Maps Pack SHALL validate viewports, annotations, and overlays without owning UI widgets

`pack.location.maps.v1` SHALL support viewport/camera validation and map
annotation/overlay planning while applications remain responsible for UI
composition and interaction behavior.

#### Scenario: Viewport is validated
- **WHEN** `maps.validate_viewport` is invoked
- **THEN** Macaca SHALL validate center, bounds, zoom, pitch, bearing, projection, dimensions, pixel ratio, precision class, locale hint, region policy, and provider support
- **AND** the result SHALL return normalized viewport metadata or typed diagnostics without rendering a UI widget

#### Scenario: Annotation is planned
- **WHEN** `maps.plan_annotation` validates markers, pins, graphics, labels, or annotation payload references
- **THEN** Macaca SHALL check coordinate precision, payload redaction, icon reference, label size, z-order, policy, resource budget, and provider support
- **AND** Macaca SHALL NOT expose private payload data or implement application-specific marker interaction logic

#### Scenario: Overlay is planned
- **WHEN** `maps.plan_overlay` validates polygons, polylines, circles, heatmaps, traffic/weather overlays, feature layers, or tile overlays
- **THEN** Macaca SHALL check geometry bounds, layer type, feature count, style reference, source reference, data sensitivity, attribution, policy, resource budget, and provider support
- **AND** private overlay data SHALL be represented by redacted source/artifact references rather than raw provider or application payloads

### Requirement: Location Maps Pack SHALL render static maps as bounded artifacts

`pack.location.maps.v1` SHALL support static map rendering and snapshot-like
artifact creation with idempotency, attribution, resource bounds, artifact
retention, and replayable evidence.

#### Scenario: Static render is planned
- **WHEN** `maps.plan_static_render` validates a static map request
- **THEN** Macaca SHALL check viewport, style, annotations, overlays, dimensions, pixel ratio, output format, attribution mode, coordinate precision, policy, entitlement, provider support, timeout, and artifact retention
- **AND** no provider side effect SHALL occur during the plan command

#### Scenario: Static map is rendered
- **WHEN** `maps.render_static_map` is invoked with a valid idempotency key and policy-allowed request
- **THEN** Macaca SHALL route through the canonical service path and return `MapArtifactHandle` with content class, format, size class, checksum/hash, redaction state, attribution bundle, retention deadline, and retrieval permissions
- **AND** raw provider responses and unbounded image/tile data SHALL remain excluded from traces, snapshots, and SDK diagnostics

#### Scenario: Static render is cancelled or times out
- **WHEN** a static render exceeds timeout, is cancelled, or exceeds resource budget
- **THEN** Macaca SHALL return a typed timeout, cancelled, or quota result with replay metadata
- **AND** partial artifacts SHALL be discarded or represented only by sanitized failure metadata according to retention policy

### Requirement: Location Maps Pack SHALL enforce attribution and cache semantics

`pack.location.maps.v1` SHALL require attribution/copyright metadata for every
tile, overlay, static render, or artifact result and SHALL expose cache/offline
status without hiding stale data.

#### Scenario: Attribution is inspected
- **WHEN** `maps.inspect_attribution` is invoked for a style, tile, overlay, or artifact reference
- **THEN** Macaca SHALL return copyright text references, data source notices, logo requirements, link requirements, provider terms references, and display rules
- **AND** the result SHALL exclude raw provider payloads and credentials

#### Scenario: Attribution is missing
- **WHEN** a provider cannot produce required attribution metadata for a tile, overlay, render, or artifact
- **THEN** Macaca SHALL return a typed attribution-missing, unavailable, or unsupported result
- **AND** Macaca SHALL NOT return unattributed map resources as successful results

#### Scenario: Cache status is inspected
- **WHEN** `maps.inspect_cache` is invoked
- **THEN** Macaca SHALL return cache key hash, availability, freshness, stale reason, offline support, retention, eviction hint, quota state, and attribution hash
- **AND** stale cached data SHALL be explicitly marked and SHALL NOT be presented as fresh provider data

### Requirement: Location Maps Pack SHALL expose health, snapshots, and replayable evidence

`pack.location.maps.v1` SHALL expose descriptor metadata, service health,
command availability, provider capability hashes, style catalog hashes, tile
matrix hashes, attribution hashes, snapshots, replay pointers, and sanitized
audit events for all operations.

#### Scenario: Snapshot is recorded
- **WHEN** the service runtime records a `pack.location.maps.v1` snapshot
- **THEN** the snapshot SHALL include descriptor version, provider capability hash, command availability, provider health, style catalog hash, tile matrix hash, attribution hash, cache summary, resource counters, artifact summaries, event cursors, and sanitized replay pointers
- **AND** it SHALL exclude raw credentials, API keys, access tokens, private style payloads, raw provider responses, unbounded tile dumps, private manifests, package bytes, private keys, signatures, and unsanitized location-derived data

#### Scenario: Trace replay inspects a command
- **WHEN** trace replay inspects any `maps.*` command
- **THEN** replay SHALL prove declaration, admission, policy, resource, entitlement, approval when required, attribution validation, service runtime routing, provider class, result variant, and sanitized audit evidence
- **AND** replay SHALL NOT require provider-specific logs, raw provider responses, client map SDK state, or application-specific UI state

#### Scenario: Provider is unavailable
- **WHEN** the active provider is unavailable, disabled, retired, degraded, command-limited, style-limited, tile-limited, overlay-limited, render-limited, attribution-limited, cache-limited, quota-limited, or rate-limited
- **THEN** SDK discovery, health, snapshots, and command results SHALL expose structured diagnostics with stable reason codes
- **AND** Macaca SHALL NOT crash, hang, silently fall back, contact undeclared providers, render maps, retrieve tiles, or fake success

### Requirement: Location Maps Pack implementation SHALL preserve Macaca boundaries

The `pack.location.maps.v1` implementation SHALL remain owned by maps service
providers and service-runtime contracts. The microkernel, SDK, shells, and
generic application framework SHALL remain provider-neutral and free of
application-specific, supplier-specific, geocoding-specific, routing-specific,
place-search-specific, device-location-specific, UI-specific, or
workflow-specific routing branches.

#### Scenario: Boundary gates scan the implementation
- **WHEN** dependency, no-direct-provider-call, and canonical execution-path gates scan the implementation
- **THEN** they SHALL find no concrete Google, Mapbox, HERE, Azure Maps, Esri, Apple MapKit, OpenStreetMap tile, OpenLayers, Leaflet, offline tile, credential, or map provider imports in the microkernel, SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned service registrations and typed service commands

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, mock, or unavailable provider is selected for `pack.location.maps.v1`
- **THEN** callers SHALL observe the same provider-neutral command/result contract
- **AND** trace/audit evidence SHALL identify only sanitized provider class, descriptor metadata, capability hashes, attribution hashes, cache hashes, and bounded result codes rather than provider-specific business branches

#### Scenario: Adjacent pack boundary is tested
- **WHEN** boundary tests exercise geocoding, routing, place search, timezone lookup, device location capture, media image processing, UI rendering, provider billing, and application workflow scenarios
- **THEN** `pack.location.maps.v1` SHALL expose only map resources, references, artifacts, attribution, and policy decisions for those concerns
- **AND** it SHALL NOT implement those adjacent pack behaviors internally

### Requirement: Location Maps Pack SHALL include detailed developer documentation

The implementation of `pack.location.maps.v1` SHALL include detailed developer
documentation under `docs/developer-packs/location/maps.md` and SHALL link that
documentation from SDK discovery metadata and the industrial pack catalog index.

#### Scenario: Developer reads the pack guide
- **WHEN** a developer opens `docs/developer-packs/location/maps.md`
- **THEN** the guide SHALL explain purpose, non-goals, manifest declaration, required versus optional behavior, permission scopes, approval behavior, command DTOs, result DTOs, style references, tile matrix descriptors, tile references, viewports, annotations, overlays, static render requests, attribution bundles, cache status, artifacts, unavailable diagnostics, provider replacement, and operational limits
- **AND** examples SHALL use synthetic data and generic handles rather than provider names, credentials, API keys, private coordinates, private overlays, raw provider payloads, unbounded tiles, application names, or business workflows

#### Scenario: Provider author reads conformance guidance
- **WHEN** a provider author reads the maps pack documentation
- **THEN** the guide SHALL include a supplier/API mapping for Google Maps Platform, Mapbox, HERE Maps, Azure Maps, Esri ArcGIS, Apple MapKit/MapKit JS, OpenStreetMap tile policies, and OpenLayers/Leaflet concepts
- **AND** it SHALL include conformance checks for descriptor completeness, tile/style/render scope validation, idempotency, tile matrix compatibility, attribution completeness, coordinate precision enforcement, cache freshness, resource bounds, policy hooks, trace/audit events, unavailable behavior, snapshot/replay, and no raw payload leakage
