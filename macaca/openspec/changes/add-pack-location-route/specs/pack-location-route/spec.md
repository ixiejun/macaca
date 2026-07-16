## ADDED Requirements

### Requirement: Macaca SHALL provide Location Route Pack as a serviceized capability

Macaca SHALL provide `pack.location.route.v1` as a provider-neutral industrial
pack for route planning, ETA and distance estimates, route alternatives,
distance/time matrices, waypoint optimization, maneuver metadata, route
geometry, route constraints, retention policy evidence, attribution, and
artifact handles. The pack SHALL be declared by applications, resolved by
admission/catalog services, and invoked only through typed service commands.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.location.route.v1` as required and a route service provider is registered, healthy, entitled, permission-compatible, policy-admissible, and retention/attribution-capable
- **THEN** admission SHALL expose the pack in the effective capability set with command schemas, permission scopes, policy template hash, resource limits, approval rules, profile metadata, constraint metadata, retention metadata, attribution metadata, provider health metadata, compatibility metadata, documentation links, and replay metadata
- **AND** SDK discovery SHALL mark callable commands as available without exposing provider secrets, API keys, access tokens, raw provider payloads, private route batches, unbounded geometry, or unsanitized location/route data

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.location.route.v1` as required but provider, permission, entitlement, resource, host support, network support, profile support, matrix support, optimization support, retention support, or attribution support is absent
- **THEN** admission SHALL block readiness with structured unavailable or denied diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back, contact another undeclared provider, calculate routes, or fake success

#### Scenario: Optional declaration is unavailable
- **WHEN** an application declares `pack.location.route.v1` as optional and the pack is unavailable
- **THEN** admission SHALL produce an explicit degraded effective capability report with unavailable reason codes and command-level availability
- **AND** SDK command helpers SHALL refuse to build callable service calls for unavailable commands

### Requirement: Location Route Pack SHALL expose supplier-grade route contracts

`pack.location.route.v1` SHALL expose provider-neutral DTOs for route scopes,
waypoints, travel profiles, constraint sets, route plans, legs, steps,
maneuvers, geometry, metrics, route matrices, optimization jobs, retention
policies, attribution bundles, artifacts, version metadata, freshness metadata,
redaction metadata, and provider capability metadata.

#### Scenario: Provider capabilities are discovered
- **WHEN** SDK discovery or `route.inspect_provider` inspects the pack
- **THEN** Macaca SHALL return command schemas, permission scopes, supported travel profiles, route constraints, traffic support, EV/freight support, matrix limits, optimization limits, geometry formats, retention modes, attribution requirements, lifecycle state, provider health, redaction profile, and compatibility hash
- **AND** the schema SHALL be provider-neutral even when backed by Google Routes, Mapbox, HERE, TomTom, Azure Maps, Esri, OSRM, Valhalla, GraphHopper, offline, built-in, plugin, remote, mock, or unavailable providers

#### Scenario: Route profiles are discovered
- **WHEN** `route.discover_profiles` is invoked
- **THEN** Macaca SHALL return supported modes, vehicle classes, avoid rules, EV/freight constraints, traffic models, geometry formats, matrix limits, optimization limits, attribution requirements, and provider capability hashes
- **AND** unsupported profiles or constraints SHALL be represented as typed unsupported diagnostics

#### Scenario: Route metrics are represented
- **WHEN** a provider returns distance, duration, traffic delay, toll estimate, energy estimate, ETA window, confidence, or freshness metadata
- **THEN** Macaca SHALL map those values into `RouteMetricSet`
- **AND** unavailable metric classes SHALL be explicit rather than silently defaulted

### Requirement: Location Route Pack commands SHALL use canonical typed service calls

Every `route.*` operation SHALL be represented as a typed command/result DTO and
SHALL traverse the canonical service runtime path with trace, policy, resource,
entitlement, approval, health, snapshot, timeout, cancellation, retention,
attribution, idempotency, redaction, and structured error behavior.

#### Scenario: Route request is validated
- **WHEN** `route.validate_request` validates route, matrix, or optimization input
- **THEN** Macaca SHALL check waypoint shape, coordinate precision, travel profile, constraint set, departure/arrival time, traffic model, retention intent, attribution requirement, policy, entitlement, resource budget, and provider support
- **AND** no provider side effect SHALL occur during the validation command

#### Scenario: Route planning succeeds
- **WHEN** `route.plan` is invoked for a declared, validated, and policy-allowed route request
- **THEN** Macaca SHALL route the command through SDK/facade helpers into the service runtime and route service provider
- **AND** the result SHALL include bounded route plans with summary, alternatives, legs, steps, maneuvers, geometry, metrics, notices, attribution bundle, retention policy, freshness, and redaction metadata

#### Scenario: ETA estimate succeeds
- **WHEN** `route.estimate_eta` is invoked for a declared and policy-allowed request
- **THEN** Macaca SHALL return `RouteMetricSet` values such as distance, duration, static duration, traffic delay, ETA window, toll estimate reference, energy estimate reference, confidence, and freshness according to provider capability
- **AND** the command SHALL NOT require full maneuver geometry when provider capability exposes metric-only estimation

#### Scenario: Route request is denied before provider call
- **WHEN** policy, permission, entitlement, approval, resource, network, coordinate precision, retention, attribution, profile, constraint, or provider capability checks reject a request
- **THEN** Macaca SHALL return a typed denied, approval-required, unavailable, unsupported, quota, retention-denied, or attribution-missing result before invoking the concrete provider
- **AND** the audit trail SHALL include bounded reason codes, hashes, counters, and sanitized references only

### Requirement: Location Route Pack SHALL support matrix calculations safely

`pack.location.route.v1` SHALL support bounded distance/time matrix jobs with
idempotency, partial-result handling, cancellation, artifact handles, retention
policy enforcement, and replayable evidence.

#### Scenario: Matrix job is planned
- **WHEN** `route.plan_matrix` validates a route matrix request
- **THEN** Macaca SHALL check origin count, destination count, matrix cell count, waypoint sensitivity, profile, traffic model, coordinate precision, retention intent, provider matrix support, quota, timeout, artifact policy, approval requirements, and entitlement
- **AND** no provider side effect SHALL occur during the plan command

#### Scenario: Matrix job is requested
- **WHEN** `route.request_matrix` is invoked with a valid idempotency key and policy-allowed inputs
- **THEN** Macaca SHALL route through the canonical service path and return `RouteMatrixJob` metadata with job handle, origin count, destination count, completed cell count, failed cell count, partial-result state, artifact handles, retention policy, cancellation state, and replay cursor
- **AND** raw origin/destination lists and raw provider payloads SHALL remain excluded from traces, snapshots, and SDK diagnostics

#### Scenario: Matrix job is cancelled
- **WHEN** `route.cancel_matrix` is invoked for an active or completed matrix job
- **THEN** Macaca SHALL return typed cancelled, conflict, unsupported, unavailable, or success diagnostics according to provider state
- **AND** cancellation evidence SHALL remain bounded and replayable

### Requirement: Location Route Pack SHALL support waypoint optimization without owning logistics workflows

`pack.location.route.v1` SHALL support generic waypoint optimization jobs while
leaving dispatch, assignment, delivery, fleet, emergency, and product-specific
business decisions outside this pack.

#### Scenario: Optimization job is planned
- **WHEN** `route.plan_optimization` validates an optimization request
- **THEN** Macaca SHALL check waypoint count, fixed stops, objective class, profile, constraints, time windows, resource limits, policy, entitlement, provider optimization support, and approval requirements
- **AND** no provider side effect SHALL occur during the plan command

#### Scenario: Optimization job is requested
- **WHEN** `route.request_optimization` is invoked with a valid idempotency key and policy-allowed inputs
- **THEN** Macaca SHALL return `WaypointOptimizationJob` metadata with objective class, waypoint count, ordered waypoints, unassigned waypoints, violations, metrics, artifact handles, freshness, and replay cursor
- **AND** Macaca SHALL NOT assign drivers, dispatch vehicles, settle tolls, book charging stations, or apply application-specific logistics rules

#### Scenario: Optimization is unsupported
- **WHEN** the active provider does not support requested waypoint count, profile, objective, EV/freight constraint, time window, or optimization mode
- **THEN** Macaca SHALL return typed unsupported or unavailable diagnostics before provider side effects
- **AND** SDK discovery SHALL report the unsupported command or constraint as non-callable for the current effective capability set

### Requirement: Location Route Pack SHALL enforce retention and attribution semantics

`pack.location.route.v1` SHALL expose and enforce provider-neutral retention
policies and attribution requirements for route plans, matrices, optimization
jobs, and artifacts.

#### Scenario: Retention policy is inspected
- **WHEN** `route.inspect_retention` is invoked for a provider, route, matrix, optimization, or artifact reference
- **THEN** Macaca SHALL return temporary/permanent mode, storage allowed flag, cache TTL class, derived-data restrictions, geometry retention restrictions, attribution requirement, provider terms reference, and redaction class
- **AND** raw provider terms payloads and credentials SHALL remain excluded

#### Scenario: Permanent route storage is denied
- **WHEN** a command requests permanent retention but provider capability, entitlement, policy, or attribution requirements do not allow it
- **THEN** Macaca SHALL return a typed retention-denied, denied, unavailable, or unsupported result before storing route, matrix, optimization, or geometry artifacts
- **AND** temporary route results SHALL NOT be silently promoted to permanent stored data

#### Scenario: Attribution is missing
- **WHEN** a provider cannot produce required attribution metadata for routes, matrices, optimization jobs, or artifacts
- **THEN** Macaca SHALL return a typed attribution-missing, unavailable, or unsupported result
- **AND** Macaca SHALL NOT return unattributed route results as successful results

### Requirement: Location Route Pack SHALL expose health, snapshots, and replayable evidence

`pack.location.route.v1` SHALL expose descriptor metadata, service health,
command availability, provider capability hashes, profile/constraint hashes,
retention-policy hashes, attribution hashes, snapshots, replay pointers, and
sanitized audit events for all operations.

#### Scenario: Snapshot is recorded
- **WHEN** the service runtime records a `pack.location.route.v1` snapshot
- **THEN** the snapshot SHALL include descriptor version, provider capability hash, command availability, provider health, profile/constraint hash, supported geometry formats, retention-policy hash, attribution hash, matrix/optimization summary, resource counters, artifact summaries, event cursors, and sanitized replay pointers
- **AND** it SHALL exclude raw credentials, API keys, access tokens, raw provider responses, private route batches, unbounded geometry dumps, private manifests, package bytes, private keys, signatures, and unsanitized location/route data

#### Scenario: Trace replay inspects a command
- **WHEN** trace replay inspects any `route.*` command
- **THEN** replay SHALL prove declaration, admission, policy, resource, entitlement, approval when required, retention validation, attribution validation, service runtime routing, provider class, result variant, and sanitized audit evidence
- **AND** replay SHALL NOT require provider-specific logs, raw provider responses, application-specific dispatch workflow state, or raw route batches

#### Scenario: Provider is unavailable
- **WHEN** the active provider is unavailable, disabled, retired, degraded, command-limited, profile-limited, traffic-limited, matrix-limited, optimization-limited, EV-limited, freight-limited, retention-limited, attribution-limited, quota-limited, or rate-limited
- **THEN** SDK discovery, health, snapshots, and command results SHALL expose structured diagnostics with stable reason codes
- **AND** Macaca SHALL NOT crash, hang, silently fall back, contact undeclared providers, calculate routes, compute matrices, optimize waypoints, or fake success

### Requirement: Location Route Pack implementation SHALL preserve Macaca boundaries

The `pack.location.route.v1` implementation SHALL remain owned by route service
providers and service-runtime contracts. The microkernel, SDK, shells, and
generic application framework SHALL remain provider-neutral and free of
application-specific, supplier-specific, geocoding-specific,
place-search-specific, map-rendering-specific, device-location-specific,
dispatch-specific, billing-specific, or workflow-specific routing branches.

#### Scenario: Boundary gates scan the implementation
- **WHEN** dependency, no-direct-provider-call, and canonical execution-path gates scan the implementation
- **THEN** they SHALL find no concrete Google Routes, Mapbox, HERE, TomTom, Azure Maps, Esri, OSRM, Valhalla, GraphHopper, offline route, credential, or route provider imports in the microkernel, SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned service registrations and typed service commands

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, mock, or unavailable provider is selected for `pack.location.route.v1`
- **THEN** callers SHALL observe the same provider-neutral command/result contract
- **AND** trace/audit evidence SHALL identify only sanitized provider class, descriptor metadata, capability hashes, retention hashes, attribution hashes, and bounded result codes rather than provider-specific business branches

#### Scenario: Adjacent pack boundary is tested
- **WHEN** boundary tests exercise geocoding, place search, map rendering, timezone lookup, device location capture, fleet dispatch, delivery optimization, toll settlement, charging booking, emergency workflows, and application logistics rules
- **THEN** `pack.location.route.v1` SHALL expose only route plans, metrics, matrices, optimization metadata, retention, attribution, artifacts, and policy decisions for those concerns
- **AND** it SHALL NOT implement those adjacent pack behaviors internally

### Requirement: Location Route Pack SHALL include detailed developer documentation

The implementation of `pack.location.route.v1` SHALL include detailed developer
documentation under `docs/developer-packs/location/route.md` and SHALL link that
documentation from SDK discovery metadata and the industrial pack catalog index.

#### Scenario: Developer reads the pack guide
- **WHEN** a developer opens `docs/developer-packs/location/route.md`
- **THEN** the guide SHALL explain purpose, non-goals, manifest declaration, required versus optional behavior, permission scopes, approval behavior, command DTOs, result DTOs, waypoints, travel profiles, constraints, route plans, legs, steps, maneuvers, geometry, metrics, matrices, optimization jobs, retention policy, attribution, artifacts, unavailable diagnostics, provider replacement, and operational limits
- **AND** examples SHALL use synthetic data and generic handles rather than provider names, credentials, API keys, private routes, raw provider payloads, unbounded route geometry, application names, or business workflows

#### Scenario: Provider author reads conformance guidance
- **WHEN** a provider author reads the route pack documentation
- **THEN** the guide SHALL include a supplier/API mapping for Google Routes, Mapbox Navigation APIs, HERE Routing/Matrix/Tour Planning, TomTom Routing, Azure Maps Route, Esri Network Analysis, OSRM, Valhalla, and GraphHopper concepts
- **AND** it SHALL include conformance checks for descriptor completeness, route/matrix/optimization scope validation, idempotency, profile mapping, constraint mapping, geometry mapping, metric mapping, matrix state machine, optimization state machine, retention enforcement, attribution completeness, resource bounds, policy hooks, trace/audit events, unavailable behavior, snapshot/replay, and no raw payload leakage
