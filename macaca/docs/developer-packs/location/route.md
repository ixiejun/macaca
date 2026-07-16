# Location Route Pack

`pack.location.route.v1` provides provider-neutral route validation, route
planning, ETA estimation, route inspection, matrix planning/request/status/cancel,
waypoint optimization planning/request/status/cancel, retention inspection,
attribution inspection, and artifact handle discovery.

The pack does not own geocoding, place search, map rendering, device location
capture, toll settlement, fleet dispatch policy, or application-specific
delivery workflows. It becomes callable only when a serviceized route provider
is registered through the runtime composition root.

## Manifest Declaration

```toml
[service_contract]
optional_packs = ["pack.location.route.v1"]
```

Optional declarations degrade with `location_route_provider_not_installed`.
Required declarations block readiness until compatible provider schemas,
permissions, policy, resource budget, entitlement, and attribution requirements
are available.

## Commands

- `route.inspect_provider` and `route.discover_profiles`: inspect capability
  classes and supported travel profiles.
- `route.validate_request`: validates `RoutePlan` without provider side
  effects.
- `route.plan`, `route.estimate_eta`, and `route.inspect`: manage bounded
  route plans, metrics, legs, steps, maneuvers, and geometry references.
- `route.plan_matrix`, `route.request_matrix`, `route.inspect_matrix`, and
  `route.cancel_matrix`: manage `RouteMatrixJob`.
- `route.plan_optimization`, `route.request_optimization`,
  `route.inspect_optimization`, and `route.cancel_optimization`: manage
  `WaypointOptimizationJob`.
- `route.inspect_retention`, `route.inspect_attribution`, and
  `route.get_artifact`: expose retention, attribution, and artifact handles.

## DTOs And Results

Core DTOs include `RouteScope`, `RouteWaypoint`, `RouteTravelProfile`,
`RouteConstraintSet`, `RoutePlan`, `RouteLeg`, `RouteStep`, `RouteManeuver`,
`RouteGeometry`, `RouteMetricSet`, `RouteMatrixJob`,
`WaypointOptimizationJob`, and `RouteArtifactHandle`. Result statuses include
success, partial, approval-required, denied, unavailable, unsupported,
conflict, no-route, ambiguous, stale-version, quota, rate-limited, timeout,
cancelled, retention-denied, attribution-missing, and failure.

## Field Notes

`RouteScope` carries tenant, region, retention, precision, and attribution
classes. Waypoints carry coordinate refs, stop metadata, and redaction class.
Travel profiles describe mode, traffic usage, EV/freight flags, and provider
capability requirements. Constraint sets carry avoid rules, time windows,
vehicle constraints, objective hints, and approval refs. Plans, legs, steps,
maneuvers, geometry, and metrics carry bounded distance/time/cost references,
geometry format, freshness, attribution, and replay pointers rather than raw
provider geometry dumps. Matrix and optimization jobs carry idempotency key,
cell/waypoint limits, async state, cancellation token, cursor, artifact
retention, and expiry. Structured errors include no-route, ambiguous-route,
profile-unsupported, constraint-unsupported, matrix-quota-exceeded,
optimization-timeout, retention-denied, attribution-missing, network-denied,
quota, rate-limited, timeout, cancelled, and artifact-denied diagnostics.

## Provider Mapping

Google Routes, Mapbox Navigation APIs, HERE Routing/Matrix/Tour Planning,
TomTom Routing, Azure Maps Route, Esri Network Analysis, OSRM, Valhalla, and
GraphHopper map into profiles, constraints, metrics, geometry references,
matrix jobs, optimization jobs, retention, and attribution. Raw geometries,
private route batches, credentials, provider responses, and dispatch business
rules are intentionally outside OS semantics.

## App-Facing Examples

Applications declare `pack.location.route.v1` as required when routing is a
readiness dependency, or optional when the UI can degrade with
`location_route_provider_not_installed`. All calls pass through typed SDK
commands with synthetic profile, waypoint, route, matrix, optimization,
retention, attribution, and artifact refs.

- Discover travel profiles, validate a route request, plan a route, estimate
  ETA, and inspect the resulting route refs.
- Plan and request matrix calculations, then inspect or cancel by
  `RouteMatrixJob` ref.
- Plan and request waypoint optimization, then inspect or cancel by
  `WaypointOptimizationJob` ref.
- Inspect retention, attribution, and artifact handles before displaying or
  persisting any route metadata.
- Handle unavailable provider, denied permission, missing entitlement, no
  route, ambiguous route, retention denied, attribution missing, unsupported
  profile, unsupported constraint, matrix quota exceeded, optimization timeout,
  network denied, artifact denied, and generic quota exceeded with
  provider-neutral diagnostics.

## Conformance

Provider authors must prove descriptor completeness, request scope validation,
idempotency, profile mapping, constraint mapping, geometry mapping, metric
mapping, matrix and optimization state machines, retention enforcement,
attribution completeness, resource bounds, policy hooks, sanitized trace/audit
events, unavailable behavior, snapshot/replay, and no raw payload leakage.
