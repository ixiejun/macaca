# Location Route Pack Design

## Context

`pack.location.route.v1` is a child proposal of the developer-pack industrial
capability catalog. It provides routing resources as a serviceized capability:
point-to-point routes, multi-stop routes, route alternatives, ETA and distance
estimates, route matrices, waypoint optimization, route geometry, maneuver
metadata, traffic-aware metrics, route constraints, retention policy, and route
artifact handles.

Routing providers differ across travel profiles, constraints, traffic models,
matrix limits, waypoint optimization behavior, EV/freight support, encoded
geometry formats, and attribution. Macaca needs a provider-neutral contract that
applications can declare and invoke without learning provider credentials,
network-analysis response shapes, or route-specific retention rules.

## Supplier Capability Matrix

| Supplier or engine | Relevant capability | Macaca interpretation |
| --- | --- | --- |
| Google Routes API | Routes, route matrix, travel modes, modifiers, traffic duration, toll info, polylines, waypoint optimization | Route plans, matrix jobs, metrics, constraints, geometry, optimization metadata |
| Mapbox Navigation APIs | Directions, Matrix, Optimization, Map Matching, profiles, steps, maneuvers, annotations, alternatives | Profiles, route legs/steps, matrix jobs, optimization jobs, annotations |
| HERE Routing | Routing, Matrix, Tour Planning, traffic, truck/EV constraints, sections, spans, notices | Route constraints, spans/notices, matrix/optimization jobs, freight/EV capability metadata |
| TomTom Routing | Traffic-aware routes, matrices, route ranges, avoid options, vehicle restrictions, EV routing | ETA metrics, reachable/range references, vehicle/EV constraints, avoid rules |
| Azure Maps Route | Directions, matrices, route ranges, travel modes, traffic, route instructions | Route/matrix DTO mapping and instruction metadata |
| Esri Network Analysis | Route, closest facility, service area, OD cost matrix, VRP, barriers, restrictions | Network-analysis capability metadata, barriers, matrix/optimization artifacts |
| OSRM / Valhalla / GraphHopper | Open routing, profiles, matrices, map matching, isochrones, encoded geometry, self-hosting | Open/offline provider strategy, geometry/profile/matrix abstractions |

## Goals

- Provide stable pack id `pack.location.route.v1` and command namespace
  `route.*`.
- Normalize waypoints, travel profiles, constraint sets, route plans, legs,
  steps, maneuvers, geometry, metrics, notices, matrix jobs, optimization jobs,
  retention policies, attribution bundles, and artifact handles.
- Support provider inspection, profile/constraint discovery, route validation,
  route planning, ETA estimation, route inspection, matrix planning/request,
  matrix status/cancel, optimization planning/request, optimization status/cancel,
  retention inspection, attribution inspection, and artifact retrieval through
  typed command/result DTOs.
- Preserve a single canonical execution path through SDK/facade clients,
  service runtime decorators, and replaceable route service providers.
- Return structured `success`, `partial`, `approval_required`, `denied`,
  `unavailable`, `unsupported`, `conflict`, `no_route`, `ambiguous`,
  `stale_version`, `quota_exceeded`, `rate_limited`, `timeout`, `cancelled`,
  and `failure` results.
- Emit sanitized trace, audit, health, snapshot, and replay evidence for every
  declaration, admission, policy decision, service call, provider decision, and
  unavailable state.
- Require detailed developer documentation at
  `docs/developer-packs/location/route.md`.

## Non-Goals

- No geocoding, place search, map rendering, timezone lookup, device tracking,
  live navigation session control, fleet dispatch workflow, delivery business
  optimization, toll settlement, transit ticketing, charging booking, or
  emergency-routing workflow.
- No client navigation UI, turn-by-turn voice playback, map widget, or
  application-specific route presentation.
- No raw API keys, tokens, credentials, raw provider responses, unbounded route
  geometry dumps, private manifests, package bytes, private keys, signatures, or
  unsanitized location/route data in observability surfaces.

## Ownership And Boundaries

- Pack id: `pack.location.route.v1`.
- Family: `location`.
- Backing service owner: replaceable route service provider.
- SDK surface: `sdk.packs.location.route`.
- Command namespace: `route.*`.
- Microkernel ownership: service-call evidence, policy facade, resource facade,
  trace/audit primitives, and scheduling primitives only.
- Application framework ownership: manifest declaration, app-scoped permission
  declarations, lifecycle projection, and effective capability mementos.
- Runtime-host ownership: provider registration, service runtime decorators,
  transport adapters, health/snapshot bridge, and unavailable/mock provider
  composition through approved composition roots.

## Command Surface

All commands carry trace context, application/session/task/tenant identifiers
when available, policy context, idempotency key for async jobs, redaction
profile, resource budget, retention intent, and replay metadata.

| Command | Purpose | Notes |
| --- | --- | --- |
| `route.inspect_provider` | Return provider capability metadata | Reports modes, constraints, matrix/optimization limits, traffic, EV/freight support, health, and unavailable reasons |
| `route.discover_profiles` | Return travel profiles and constraints | Exposes supported modes, avoid rules, vehicle constraints, traffic models, geometry formats, and attribution |
| `route.validate_request` | Validate route/matrix/optimization request | Checks waypoint shape, precision, profile, constraints, retention, policy, entitlement, and resource budgets |
| `route.plan` | Calculate route alternatives | Returns route plans, legs, steps, maneuvers, geometry, metrics, notices, attribution, and retention policy |
| `route.estimate_eta` | Estimate duration/distance metrics | Returns route metric set without requiring full maneuver geometry where provider supports it |
| `route.inspect` | Inspect stored/returned route metadata | Returns bounded details, freshness, geometry summary, notices, and replay handles |
| `route.plan_matrix` | Validate distance/time matrix job | Checks origin/destination counts, profile, traffic model, precision, quota, and provider support |
| `route.request_matrix` | Start matrix calculation | Requires idempotency, partial-result handling, timeout/cancellation, and artifact metadata |
| `route.inspect_matrix` | Inspect matrix job status/result metadata | Returns counters, partial failures, artifacts, retention state, and replay cursor |
| `route.cancel_matrix` | Cancel matrix job where supported | Returns cancellation status and bounded audit evidence |
| `route.plan_optimization` | Validate waypoint optimization job | Checks waypoint count, fixed stops, constraints, objective class, quota, and provider support |
| `route.request_optimization` | Start optimization job | Requires idempotency, resource policy, partial-result handling, and artifact metadata |
| `route.inspect_optimization` | Inspect optimization result metadata | Returns ordered waypoints, metrics, unassigned stops, violations, artifacts, and freshness |
| `route.cancel_optimization` | Cancel optimization job where supported | Returns typed cancellation diagnostics |
| `route.inspect_retention` | Inspect route storage/cache policy | Returns retention mode, cache TTL class, derived-data restrictions, and provider terms reference |
| `route.inspect_attribution` | Return attribution/source requirements | Provides provider/data-source notices and display requirements |
| `route.get_artifact` | Retrieve route/matrix/optimization artifact metadata | Does not expose raw provider payloads or unbounded geometry |

## Provider-Neutral DTO Model

- `RouteScope`: application id, tenant id, region policy reference, provider
  reference, and trace context.
- `RouteWaypoint`: coordinate, geocode/place reference, label reference, stop
  type, time window reference, service duration, precision class, and redaction
  class.
- `RouteTravelProfile`: profile handle, mode, vehicle class, pedestrian/cycling
  hints, truck/EV support, traffic support, and provider capability hash.
- `RouteConstraintSet`: avoid rules, include rules, vehicle dimensions, weight,
  hazmat class, axle count, EV battery/charging hints, departure/arrival time,
  traffic model, region restrictions, and accessibility hints.
- `RoutePlan`: route handle, summary, alternatives, legs, geometry, metrics,
  notices, attribution bundle, retention policy, freshness, and redaction.
- `RouteLeg`: start/end waypoint references, distance, duration, static
  duration, traffic delay, steps, geometry reference, and notice references.
- `RouteStep`: maneuver, instruction reference, distance, duration, geometry
  reference, travel mode, road/access metadata, and redaction class.
- `RouteManeuver`: action class, bearing, side-of-street hint, exit number,
  junction hint, signpost reference, and localized instruction reference.
- `RouteGeometry`: encoded polyline/reference, bounds, precision, spatial
  reference, simplification level, and artifact handle.
- `RouteMetricSet`: distance, duration, static duration, traffic delay, ETA
  window, toll estimate reference, energy estimate reference, confidence, and
  freshness.
- `RouteMatrixJob`: job handle, origin count, destination count, completed
  cell count, failed cell count, partial-result state, artifact handles, and
  replay cursor.
- `WaypointOptimizationJob`: job handle, objective class, waypoint count,
  ordered waypoints, unassigned waypoints, violations, metrics, artifacts, and
  freshness.
- `RouteArtifactHandle`: artifact id, content class, redaction state, retention
  deadline, size class, checksum/hash, and retrieval permissions.

## Permission, Policy, Resource, Entitlement, And Approval Model

Initial permission scopes:

- `location.route.plan`
- `location.route.eta`
- `location.route.matrix`
- `location.route.optimize`
- `location.route.inspect`
- `location.route.retention.read`
- `location.route.attribution.read`
- `location.route.artifact.read`

Policy checks run before provider calls. Inputs include caller subject,
application id, tenant id, command, waypoint sensitivity, coordinate precision,
travel profile, constraint set, departure/arrival time, traffic model, batch
size, retention intent, result field mask, resource budget, approval state, and
entitlement state.

Approval is required when policy marks routing as sensitive, such as precise
private routes, retained route artifacts, high-volume matrices, regulated
region/data-boundary crossings, freight/hazmat constraints, or externally
shared route artifacts.

Resource checks cover waypoint count, origin/destination matrix cells, geometry
size, alternatives count, maneuver count, optimization waypoint count, provider
quota, network budget, timeout, artifact size, retained snapshots, retained
artifacts, and event volume.

Entitlement checks determine whether the calling application/tenant may use
route planning, ETA estimation, traffic-aware routes, matrix jobs, optimization,
EV/freight constraints, retained artifacts, and premium provider features.

## Service Runtime And Provider Strategy

The route service provider is a Strategy behind the service runtime. The
runtime composes provider adapters, unavailable providers, mock providers,
policy decorators, resource decorators, entitlement decorators, metering,
redaction, retention enforcement, attribution enforcement, trace, audit,
timeout/cancellation, and health/snapshot behavior.

Provider adapters may target Google Routes, Mapbox Navigation APIs, HERE,
TomTom, Azure Maps, Esri, OSRM, Valhalla, GraphHopper, offline routing engines,
built-in local providers, remote providers, plugin providers, or mock providers.
Provider-specific capabilities are descriptor data, not OS routing branches.

The unavailable provider is first-class. It exposes descriptor metadata, health
state, unsupported command diagnostics, and stable error DTOs without crashing,
hanging, silently falling back, contacting undeclared providers, or faking
success.

## SDK Discovery And Developer Documentation

SDK discovery must return pack metadata, command schemas, permission scopes,
profile support, constraint support, traffic support, EV/freight support,
matrix limits, optimization limits, geometry formats, retention modes,
attribution requirements, examples, availability, diagnostics, provider class,
compatibility hash, redaction profile, and documentation link.

SDK helper builders only build canonical traced service calls. They must never
construct providers, hold credentials, call provider APIs directly, geocode
addresses, search places, render maps, capture device location, run fleet
dispatch logic, settle tolls, or bypass retention/policy.

Developer documentation at `docs/developer-packs/location/route.md` must cover
purpose, non-goals, manifest declaration, permission scopes, command DTOs,
result DTOs, provider mapping, route/matrix/optimization examples,
retention/attribution rules, unavailable diagnostics, trace/audit events,
redaction, snapshot/replay, and provider-author conformance checks.

## Trace, Audit, Health, Snapshot, And Replay

Events include pack id, descriptor version, command name, trace id,
application/session/task/tenant identifiers when available, waypoint hash,
route geometry hash, policy decision, approval state, provider class, latency,
bounded resource counters, capability hash, retention policy hash, attribution
hash, and bounded error code.

Events, snapshots, SDK diagnostics, and examples must exclude raw credentials,
API keys, access tokens, raw provider responses, private route batches,
unbounded geometry dumps, private manifests, package bytes, private keys,
signatures, and unsanitized location/route data.

Snapshots include descriptor version, provider capability hash, command
availability, provider health, profile/constraint hash, supported geometry
formats, retention-policy hash, attribution hash, matrix/optimization summary,
resource counters, artifact summaries, event cursors, and sanitized replay
pointers.

## Design Patterns

- **Facade**: `SystemFacade` and focused SDK clients expose discovery and typed
  command builders while hiding service runtime and provider composition.
- **Command**: every operation is represented as a typed command/result DTO.
- **Adapter/Bridge**: Google, Mapbox, HERE, TomTom, Azure, Esri, OSRM,
  Valhalla, GraphHopper, offline, built-in, plugin, remote, mock, and
  unavailable providers adapt into the same contract.
- **Strategy**: provider selection, profile mapping, constraint mapping,
  matrix behavior, optimization behavior, retention handling, attribution
  behavior, and unavailable behavior are replaceable.
- **Decorator**: trace, audit, policy, resource, entitlement, approval,
  metering, timeout, cancellation, retention, attribution, and redaction wrap
  every call.
- **State**: matrix jobs, optimization jobs, artifacts, provider lifecycle, and
  retention states are explicit and replayable.
- **Observer**: trace, audit, health, and service events are subscribable by
  shells without giving shells semantic ownership.
- **Memento**: effective capability reports, snapshots, provider capability
  hashes, profile/constraint hashes, retention hashes, and audit cursors
  preserve bounded recovery state.
- **Specification**: admission validates pack id, command availability,
  permissions, provider health, entitlement, resource budgets, retention,
  attribution, and policy templates.
- **Abstract Factory**: concrete provider adapters are constructed only in
  approved composition roots.

## Risks And Mitigations

- Risk: route becomes fleet dispatch or delivery optimization. Mitigation:
  optimization returns generic waypoint order/violations/metrics; business
  assignment and dispatch remain application/workflow services.
- Risk: route consumes geocoding/place-search semantics. Mitigation: waypoints
  may reference geocode/place handles but this pack does not resolve them.
- Risk: private route geometry leaks. Mitigation: coordinate precision, geometry
  redaction, retention, and approval gates run before provider calls and exports.
- Risk: provider EV/freight details become OS business policy. Mitigation:
  constraints are provider-neutral hints and diagnostics; compliance decisions
  stay policy/application-side.
- Risk: SDK helpers become provider SDK wrappers. Mitigation: helpers only build
  canonical service commands and never hold credentials.
