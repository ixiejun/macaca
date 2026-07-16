# Change: Add Location Route Pack

## Why

Macaca applications need `pack.location.route.v1` as an industrial routing
capability for point-to-point routes, multi-stop routes, ETA and distance
estimates, route alternatives, distance/time matrices, waypoint optimization,
turn-by-turn maneuver metadata, route constraints, traffic-aware calculations,
EV/freight constraints, and route diagnostics. Mature providers expose these
capabilities through directions, routes, matrices, optimization, map matching,
traffic models, toll/ferry/highway avoidance, vehicle profiles, charging-aware
routes, and encoded geometry formats. Macaca must normalize routing without
becoming a navigation UI, fleet-dispatch product, map renderer, geocoder, place
search engine, device tracking provider, or provider-specific route SDK.

This proposal defines routing as a serviceized, provider-neutral pack. It gives
applications typed route commands while keeping concrete Google Routes, Mapbox,
HERE, TomTom, Azure Maps, Esri, OSRM/Valhalla/GraphHopper-style, offline,
mock, and unavailable providers behind replaceable service providers.

## Supplier And API Baseline

The design is based on mature routing APIs:

- Google Maps Routes API exposes `computeRoutes`, `computeRouteMatrix`, travel
  modes, route modifiers, traffic-aware duration, toll information, polyline
  encodings, localized values, route labels, and waypoint optimization.
- Mapbox Directions, Matrix, Optimization, and Map Matching APIs expose routing
  profiles, steps, maneuvers, alternatives, annotations, traffic, matrices,
  optimized trips, and geometry formats.
- HERE Routing, Matrix Routing, and Tour Planning expose routes, sections,
  spans, notices, transport modes, traffic, truck/EV constraints, matrices, and
  optimization/tour planning workflows.
- TomTom Routing and Matrix Routing expose route calculation, traffic, travel
  modes, route types, avoid options, vehicle restrictions, EV routing, reachable
  ranges, batch/matrix behavior, and guidance instructions.
- Azure Maps Route APIs expose directions, route matrix, route range, travel
  modes, traffic, avoid options, and route instructions.
- Esri routing services expose route, closest facility, service area, origin
  destination cost matrix, vehicle routing problem, barriers, restrictions, and
  network-analysis outputs.
- OSRM, Valhalla, and GraphHopper-style engines provide open routing references
  for profiles, matrices, map matching, isochrones, encoded geometry, and
  offline/self-hosted provider replacement.

Research references:

- Google Maps Routes API:
  https://developers.google.com/maps/documentation/routes
- Mapbox Directions, Matrix, Optimization, and Map Matching:
  https://docs.mapbox.com/api/navigation/
- HERE Routing and Matrix Routing:
  https://www.here.com/docs/category/routing
- TomTom Routing API:
  https://developer.tomtom.com/routing-api/documentation/product-information/introduction
- Azure Maps Route APIs:
  https://learn.microsoft.com/rest/api/maps/route
- Esri routing and network analysis:
  https://developers.arcgis.com/documentation/mapping-apis-and-services/routing/
- OSRM, Valhalla, and GraphHopper:
  https://project-osrm.org/docs/v5.24.0/api/,
  https://valhalla.github.io/valhalla/api/, and
  https://docs.graphhopper.com/

## Macaca Provider-Neutral Mapping

`pack.location.route.v1` maps supplier concepts into stable Macaca contracts:

- Origins, destinations, waypoints, via points, stops, depot points, and
  candidate place/geocode references become `RouteWaypoint` values with
  coordinate precision and source-reference metadata.
- Driving, walking, cycling, transit-adjacent, truck, scooter, EV, and custom
  provider profiles become `RouteTravelProfile`; provider-specific profiles are
  descriptor capabilities, not OS routing branches.
- Avoid tolls, highways, ferries, indoor routes, low-emission zones, hazmat,
  vehicle dimensions, weight, axle count, EV battery, charging, traffic model,
  departure/arrival time, and region constraints become `RouteConstraintSet`.
- Route legs, sections, steps, maneuvers, spans, notices, encoded polylines,
  shapes, and annotations become `RoutePlan`, `RouteLeg`, `RouteStep`,
  `RouteManeuver`, and `RouteGeometry`.
- ETA, duration, static duration, traffic delay, distance, toll estimates,
  confidence, and freshness become `RouteMetricSet`.
- Distance matrices, travel-time matrices, origin-destination cost matrices, and
  batch route matrices become `RouteMatrixJob` and bounded matrix artifacts.
- Optimized trips, waypoint reordering, vehicle routing problem outputs, and
  tour planning outputs become `WaypointOptimizationPlan` with explicit
  provider capability and non-goal boundaries.

## What Changes

- Add provider-neutral `pack.location.route.v1` under the location family.
- Define commands for provider inspection, profile/constraint discovery, route
  validation, route planning, route alternative inspection, ETA estimation,
  matrix planning/request/status/cancel, waypoint optimization
  planning/request/status/cancel, route diagnostics, retention/attribution
  inspection, and artifact retrieval.
- Define DTOs for route scope, provider capability, waypoints, travel profiles,
  constraints, route plans, legs, steps, maneuvers, geometry, metrics, notices,
  matrices, optimization jobs, retention policy, attribution, freshness,
  redaction, and artifact handles.
- Require policy, coordinate precision controls, traffic/retention policy,
  provider attribution, resource/quota checks, entitlement checks, idempotency
  for async jobs, sanitized trace/audit, and deterministic
  unavailable/unsupported behavior.
- Require detailed developer documentation at
  `docs/developer-packs/location/route.md`.

## Impact

- Affected specs: `pack-location-route`,
  `developer-pack-industrial-capability-catalog`, `sdk-system-facade`,
  `service-runtime`, `unified-execution-path`.
- Affected code later: protocol DTOs, pack descriptors, admission validators,
  SDK discovery/command builders, route service providers,
  mock/unavailable providers, trace/audit schemas, replay tests, redaction
  tests, matrix/optimization tests, retention/attribution tests, and boundary
  gates.

## Non-Goals

- No geocoding, place search, map rendering, timezone lookup, device tracking,
  navigation UI, live turn-by-turn session control, fleet dispatch workflow,
  delivery optimization business policy, emergency routing workflow, or
  application-specific logistics rules.
- No provider-specific route scoring policy, vehicle compliance engine, toll
  settlement, transit ticketing, charging-station booking, or provider SDK
  initialization in Macaca OS layers.
- No raw credentials, API keys, access tokens, raw provider responses, private
  route batches, unbounded route geometry dumps, private manifests, package
  bytes, private keys, signatures, or unsanitized location/route data in logs,
  traces, snapshots, or SDK diagnostics.
- No provider construction or provider-name routing in kernel, SDK, shells, or
  generic application framework.
