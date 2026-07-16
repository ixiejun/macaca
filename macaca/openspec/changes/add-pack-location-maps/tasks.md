## 1. Research, Governance, And Scope

- [x] 1.1 Re-read architecture governance, microkernel boundaries, serviceization allowlist, design-pattern guidance, and the industrial catalog umbrella proposal before implementation.
- [x] 1.2 Record supplier/API findings for Google Maps Platform, Mapbox, HERE Maps, Azure Maps, Esri ArcGIS, Apple MapKit/MapKit JS, OpenStreetMap tile policies, and OpenLayers/Leaflet-style layer abstractions.
- [x] 1.3 Confirm boundary decisions with adjacent packs: geocode owns address/coordinate conversion, route owns path calculation, place-search owns POI search, timezone owns time zone lookup, device owns sensor/location capture, media owns image processing beyond map render artifacts, and application UI owns map widgets.
- [x] 1.4 Inventory existing descriptors, SDK clients, location services, artifact services, cache services, service-runtime decorators, mock providers, and unavailable providers that can back maps service implementation.
- [x] 1.5 Record GitNexus CRITICAL/HIGH findings as memo only before implementation commits, without letting advisory severity block this proposal track.

## 2. Contract, Descriptor, And Schema

- [x] 2.1 Define `pack.location.maps.v1` descriptor metadata for pack id, family, lifecycle, stability, command schemas, permissions, policy template, resource budget, approval rules, attribution rules, data governance, SDK metadata, docs link, compatibility, health, and diagnostics.
- [x] 2.2 Define provider-neutral DTOs for `MapScope`, `MapStyleReference`, `TileMatrixDescriptor`, `MapTileCoordinate`, `MapTileReference`, `MapViewport`, `MapAnnotation`, `MapOverlay`, `StaticMapRenderRequest`, `MapAttributionBundle`, `MapCacheStatus`, and `MapArtifactHandle`.
- [x] 2.3 Define command DTOs for `maps.inspect_provider`, `maps.discover_styles`, `maps.discover_tile_matrix`, `maps.plan_tile_request`, `maps.get_tile`, `maps.validate_viewport`, `maps.plan_annotation`, `maps.plan_overlay`, `maps.plan_static_render`, `maps.render_static_map`, `maps.inspect_attribution`, `maps.inspect_cache`, and `maps.get_artifact`.
- [x] 2.4 Define typed success, partial, approval-required, denied, unavailable, unsupported, conflict, stale-version, quota, rate-limited, timeout, cancelled, attribution-missing, cache-stale, and failure result DTOs.
- [x] 2.5 Add descriptor hashing, schema-version compatibility, command-availability hashing, style-catalog hashing, tile-matrix hashing, attribution-bundle hashing, cache-policy hashing, and redaction-profile hashing.
- [x] 2.6 Add unit tests for valid descriptors, rejected descriptors, missing command schemas, invalid permission scopes, unsupported tile formats, invalid zoom bounds, missing attribution metadata, unstable hashes, incompatible versions, and redaction metadata.

## 3. Permission, Policy, Resource, Entitlement, And Approval

- [x] 3.1 Implement declaration validation for scopes: `location.maps.read`, `location.maps.tile.read`, `location.maps.style.read`, `location.maps.viewport.validate`, `location.maps.annotation.plan`, `location.maps.overlay.plan`, `location.maps.render`, `location.maps.attribution.read`, `location.maps.cache.read`, and `location.maps.artifact.read`.
- [ ] 3.2 Implement policy checks for caller subject, application id, tenant id, command, requested fields, coordinate precision class, region/residency policy, style/layer type, overlay sensitivity, render dimensions, network/cache mode, attribution requirement, resource budget, approval state, and entitlement state before provider calls.
- [ ] 3.3 Implement resource reservation for tile count, tile size, tile zoom range, render dimensions, pixel ratio, overlay feature count, geometry complexity, artifact size, cache retention, provider quota, network budget, timeout, retained snapshots, and event volume.
- [ ] 3.4 Implement entitlement checks and structured unavailable/denied diagnostics for missing provider, disabled pack, missing permission, missing entitlement, premium style unavailable, imagery unavailable, overlay unavailable, static render unavailable, offline/cache unavailable, attribution unavailable, and disabled host/network capability.
- [ ] 3.5 Implement approval behavior for precise private-coordinate renders, retained map artifacts, high-volume tile retrieval, externally hosted private overlays, regulated region/data-boundary crossings, and long-running render jobs.
- [ ] 3.6 Add tests proving denied, unavailable, unsupported, quota, approval-required, attribution-missing, cache-stale, conflict, stale-version, missing-entitlement, and disabled-network paths do not call concrete providers or emit side effects.

## 4. Service Runtime Provider Implementation

- [ ] 4.1 Implement or bind maps service provider behind the service runtime; do not construct providers from SDK, shell, kernel, or application code.
- [x] 4.2 Add a deterministic unavailable provider that returns complete descriptor metadata, health state, command availability, attribution diagnostics, and typed unavailable/unsupported diagnostics.
- [ ] 4.3 Add mock provider support for provider inspection, style discovery, tile matrix discovery, tile planning/retrieval, viewport validation, annotation planning, overlay planning, static render planning/request, attribution inspection, cache inspection, and artifact handle metadata.
- [ ] 4.4 Add lifecycle, health, snapshot, shutdown, timeout, cancellation, pagination where applicable, cache freshness, async static render, idempotency, stale-version diagnostics, attribution diagnostics, cache diagnostics, quota diagnostics, and rate-limit diagnostics.
- [ ] 4.5 Add Strategy implementations for provider adapters, style mapping, tile matrix mapping, tile retrieval, viewport validation, annotation validation, overlay validation, static rendering, attribution resolution, cache behavior, artifact behavior, redaction, and unavailable behavior.
- [ ] 4.6 Add explicit state machines for tile freshness, style version, cache entries, static render artifacts, attribution availability, and provider lifecycle states.
- [ ] 4.7 Add side-effect safety support for idempotency keys, cache validation, attribution validation, coordinate precision enforcement, geometry size bounds, render cancellation, artifact retention, and non-mutating plan commands.
- [ ] 4.8 Add provider capability reporting for available, degraded, preview, unavailable, unsupported, retired, tile-limited, style-limited, overlay-limited, render-limited, attribution-limited, cache-limited, quota-limited, and rate-limited states.

## 5. SDK, Admission, ABI, And Examples

- [x] 5.1 Extend SDK discovery for `pack.location.maps.v1` with command schemas, permission scopes, style catalog metadata, tile matrix support, supported formats, projection support, static render limits, overlay support, attribution requirements, cache support, examples, availability, diagnostics, documentation link, provider class, compatibility hash, and redaction profile.
- [x] 5.2 Extend application admission so required declarations block readiness when unavailable and optional declarations degrade explicitly with effective capability mementos.
- [x] 5.3 Add SDK command helper builders for all `maps.*` commands; helpers must only build canonical traced service calls and must never construct providers, hold credentials, call provider APIs directly, geocode addresses, calculate routes, search places, capture device location, render UI widgets, or bypass attribution/policy.
- [ ] 5.4 Extend WASM/app ABI descriptors so applications can discover maps commands, declare permissions, receive unavailable diagnostics, and submit typed service calls through the canonical execution path.
- [x] 5.5 Add generic app-facing examples for style discovery, tile matrix discovery, tile retrieval, viewport validation, marker/annotation planning, overlay planning, static rendering, attribution inspection, cache inspection, artifact inspection, and unavailable diagnostics.
- [x] 5.6 Add provider-unavailable, missing-permission, missing-entitlement, attribution-missing, style-unsupported, tile-format-unsupported, render-size-denied, precise-location-approval-required, cache-stale, quota-exceeded, network-denied, and artifact-denied examples that avoid provider names, credentials, raw provider payloads, private coordinates, private overlays, unbounded tiles, and application business workflows.

## 6. Trace, Audit, Replay, And Boundary Gates

- [ ] 6.1 Emit sanitized declaration, admission, discovery, policy, resource, entitlement, approval, service-call, style discovery, tile matrix discovery, tile retrieval, viewport validation, annotation planning, overlay planning, static render, attribution resolution, cache inspection, artifact, health, snapshot, unavailable, conflict, and failure events.
- [ ] 6.2 Ensure traces, audits, snapshots, SDK diagnostics, and examples exclude raw credentials, API keys, access tokens, private style payloads, raw provider responses, unbounded tile dumps, private overlays, raw manifests, package bytes, private keys, signatures, and unsanitized location-derived data.
- [ ] 6.3 Add replay tests proving every `maps.*` command is trace-addressable through the canonical service path and snapshots contain enough bounded metadata for recovery diagnostics.
- [ ] 6.4 Add dependency gates proving kernel, SDK, shells, and generic application framework do not import concrete Google, Mapbox, HERE, Azure Maps, Esri, Apple MapKit, OSM tile, OpenLayers, Leaflet, offline tile, credential, or map provider adapters.
- [ ] 6.5 Add no-direct-provider-call gates proving SDK helpers, WASM ABI handlers, app admission, web, CLI, and frontend paths route through descriptor-owned service commands.
- [ ] 6.6 Add boundary tests proving optional provider absence returns structured unavailable diagnostics and never crashes, hangs, silently falls back, retrieves tiles, renders maps, uses credentials, contacts providers, or fakes success.
- [ ] 6.7 Run `openspec validate add-pack-location-maps --strict`, targeted cargo tests, boundary gates, file-size gates, attribution conformance tests, and audit replay checks before marking implementation complete.

## 7. Developer Documentation

- [x] 7.1 Create `docs/developer-packs/location/maps.md` with purpose, capability model, manifest declaration, required versus optional behavior, permissions, style references, tile matrix descriptors, tile references, viewports, annotations, overlays, static render requests, attribution bundles, cache status, artifacts, unavailable diagnostics, provider replacement, and operational limits.
- [x] 7.2 Document all command DTOs and result DTOs with field-level explanations, idempotency semantics, tile coordinate schemes, zoom bounds, projection/spatial reference behavior, render dimensions, artifact retention, cache freshness, attribution requirements, redaction behavior, approval behavior, and structured error codes.
- [x] 7.3 Document supplier/API mapping: Google Maps Platform, Mapbox, HERE Maps, Azure Maps, Esri ArcGIS, Apple MapKit/MapKit JS, OpenStreetMap tile policies, and OpenLayers/Leaflet concepts mapped to Macaca abstractions, including what is intentionally not exposed as OS semantics.
- [x] 7.4 Add generic examples for required declaration, optional declaration, tile retrieval, static rendering, overlay planning, attribution inspection, cache inspection, artifact inspection, unavailable provider, denied permission, attribution-missing, quota-exceeded, and stale-cache handling.
- [x] 7.5 Add conformance checklist and test guidance for provider authors: descriptor completeness, tile/style/render scope validation, idempotency, tile matrix compatibility, attribution completeness, coordinate precision enforcement, cache freshness, resource bounds, policy hooks, trace/audit events, unavailable behavior, snapshot/replay, and no raw payload leakage.
- [x] 7.6 Cross-link the guide from SDK discovery metadata and the industrial pack catalog index before marking `add-pack-location-maps` complete.
