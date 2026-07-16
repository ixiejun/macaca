# Location Route Pack Research

## Purpose

This note records supplier/API research, Macaca provider-neutral mapping,
boundary decisions, existing platform inventory, and GitNexus memo evidence for
`pack.location.route.v1`. The route pack owns route validation, route planning,
ETA estimation, route inspection, matrix jobs, waypoint optimization jobs,
retention/attribution inspection, artifacts, freshness, and redaction. It must
not own geocoding, place search, map rendering, timezone lookup, device location
capture, workflow dispatch approvals, toll settlement, or application
fleet/delivery business policy.

## Source Baseline

- Google Routes API:
  <https://developers.google.com/maps/documentation/routes>
- Mapbox Directions, Matrix, Optimization, and Map Matching APIs:
  <https://docs.mapbox.com/api/navigation/>
- HERE Routing, Matrix Routing, and Tour Planning:
  <https://www.here.com/docs/category/routing>
- TomTom Routing, Matrix, and EV Routing:
  <https://developer.tomtom.com/routing-api/documentation/product-information/introduction>
- Azure Maps Route:
  <https://learn.microsoft.com/en-us/rest/api/maps/route>
- Esri Network Analysis:
  <https://developers.arcgis.com/rest/network/api-reference/overview-of-network-analysis-services.htm>
- OSRM:
  <https://project-osrm.org/docs/v5.24.0/api/>
- Valhalla:
  <https://valhalla.github.io/valhalla/api/>
- GraphHopper Directions API:
  <https://docs.graphhopper.com/openapi/routing>

## Supplier API Notes

- Google, Mapbox, HERE, TomTom, Azure Maps, and Esri expose profiles,
  waypoints, route alternatives, traffic, avoidance constraints, EV/freight
  variants, matrix routing, optimization, geometry, maneuvers, attribution,
  quotas, and provider-specific errors. Macaca should normalize these as travel
  profiles, constraint sets, jobs, metrics, geometry formats, and typed result
  envelopes.
- OSRM, Valhalla, and GraphHopper contribute open/self-hostable routing engine
  patterns with profile-specific costing, matrix, map matching, and geometry
  semantics. Macaca should treat local/offline engines as replaceable providers,
  not as special OS-layer execution paths.
- Supplier APIs vary on retention, traffic freshness, route geometry precision,
  asynchronous jobs, and attribution. Macaca should model all of these through
  policy/resource gates, descriptor capability hashes, and replayable metadata.

## Macaca-Owned Abstractions

`pack.location.route.v1` should define `RouteScope`, `RouteWaypoint`,
`RouteTravelProfile`, `RouteConstraintSet`, `RoutePlan`, `RouteLeg`,
`RouteStep`, `RouteManeuver`, `RouteGeometry`, `RouteMetricSet`,
`RouteMatrixJob`, `WaypointOptimizationJob`, `RouteArtifactHandle`,
`RouteRetentionPolicy`, `RouteAttribution`, and `RouteRedactionPolicy`.

The DTOs must carry waypoint sensitivity, coordinate precision, travel profile,
constraint hash, departure/arrival time, traffic model, geometry format,
metric units, alternatives count, matrix cell count, optimization objective,
async job state, provider attribution, retention mode, artifact checksum,
redaction class, and replay pointers. Raw provider responses, private route
batches, unbounded geometry dumps, raw credentials, provider-specific route IDs
as stable OS identifiers, and fleet/delivery business workflows are rejected.

## Boundary Decisions And Non-Goals

- Geocode owns address and coordinate conversion.
- Place-search owns POI search and details.
- Maps owns rendering, tiles, and map artifacts.
- Timezone owns time zone lookup.
- Device owns location capture.
- Workflow owns approval/review orchestration.
- Applications own dispatch, fleet, delivery, toll, and optimization business
  policy.

## Existing Macaca Platform Inventory

- `crates/foundation/macaca-proto/src/domain_pack_contract/` provides reusable
  descriptor metadata, lifecycle/availability, policy templates, diagnostics,
  SDK metadata, provider snapshots, unavailable diagnostics, and effective
  capability expansion.
- `crates/facade/macaca-sdk/src/system_facade.rs` provides the Facade pattern;
  route SDK helpers should only build canonical traced service calls.
- Generic policy, approval, resource, entitlement, trace, audit, artifact,
  mock-provider, and unavailable-provider concepts are reusable, but current
  evidence does not prove route-specific DTOs, descriptors, providers, SDK
  helpers, ABI metadata, tests, or developer docs.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
