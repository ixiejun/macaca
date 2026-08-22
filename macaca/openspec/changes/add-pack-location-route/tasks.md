## 1. Research, Governance, And Scope

- [x] 1.1 Re-read architecture governance, microkernel boundaries, serviceization allowlist, design-pattern guidance, and the industrial catalog umbrella proposal before implementation.
- [x] 1.2 Record supplier/API findings for Google Routes, Mapbox Directions/Matrix/Optimization/Map Matching, HERE Routing/Matrix/Tour Planning, TomTom Routing/Matrix/EV Routing, Azure Maps Route, Esri Network Analysis, OSRM, Valhalla, and GraphHopper.
- [x] 1.3 Confirm boundary decisions with adjacent packs: geocode owns address/coordinate conversion, place-search owns POI search, maps owns rendering, timezone owns timezone lookup, device owns location capture, workflow owns dispatch approvals, and applications own fleet/delivery business policy.
- [x] 1.4 Inventory existing descriptors, SDK clients, location services, artifact services, service-runtime decorators, mock providers, and unavailable providers that can back route service implementation.
- [x] 1.5 Record GitNexus CRITICAL/HIGH findings as memo only before implementation commits, without letting advisory severity block this proposal track.

## 2. Contract, Descriptor, And Schema

- [x] 2.1 Define `pack.location.route.v1` descriptor metadata for pack id, family, lifecycle, stability, command schemas, permissions, policy template, resource budget, approval rules, retention rules, attribution rules, data governance, SDK metadata, docs link, compatibility, health, and diagnostics.
- [x] 2.2 Define provider-neutral DTOs for `RouteScope`, `RouteWaypoint`, `RouteTravelProfile`, `RouteConstraintSet`, `RoutePlan`, `RouteLeg`, `RouteStep`, `RouteManeuver`, `RouteGeometry`, `RouteMetricSet`, `RouteMatrixJob`, `WaypointOptimizationJob`, and `RouteArtifactHandle`.
- [x] 2.3 Define command DTOs for `route.inspect_provider`, `route.discover_profiles`, `route.validate_request`, `route.plan`, `route.estimate_eta`, `route.inspect`, `route.plan_matrix`, `route.request_matrix`, `route.inspect_matrix`, `route.cancel_matrix`, `route.plan_optimization`, `route.request_optimization`, `route.inspect_optimization`, `route.cancel_optimization`, `route.inspect_retention`, `route.inspect_attribution`, and `route.get_artifact`.
- [x] 2.4 Define typed success, partial, approval-required, denied, unavailable, unsupported, conflict, no-route, ambiguous, stale-version, quota, rate-limited, timeout, cancelled, retention-denied, attribution-missing, and failure result DTOs.
- [x] 2.5 Add descriptor hashing, schema-version compatibility, command-availability hashing, profile/constraint hashing, geometry-format hashing, retention-policy hashing, attribution-bundle hashing, and redaction-profile hashing.
- [x] 2.6 Add unit tests for valid descriptors, rejected descriptors, missing command schemas, invalid permission scopes, unsupported profiles, unsupported constraints, invalid waypoint counts, missing attribution metadata, retention mismatch, unstable hashes, incompatible versions, and redaction metadata.

## 3. Permission, Policy, Resource, Entitlement, And Approval

- [x] 3.1 Implement declaration validation for scopes: `location.route.plan`, `location.route.eta`, `location.route.matrix`, `location.route.optimize`, `location.route.inspect`, `location.route.retention.read`, `location.route.attribution.read`, and `location.route.artifact.read`.
- [x] 3.2 Implement policy checks for caller subject, application id, tenant id, command, waypoint sensitivity, coordinate precision, travel profile, constraint set, departure/arrival time, traffic model, batch size, retention intent, result field mask, resource budget, approval state, and entitlement state before provider calls.
- [x] 3.3 Implement resource reservation for waypoint count, origin/destination matrix cells, geometry size, alternatives count, maneuver count, optimization waypoint count, provider quota, network budget, timeout, artifact size, retained snapshots, retained artifacts, and event volume.
- [x] 3.4 Implement entitlement checks and structured unavailable/denied diagnostics for missing provider, disabled pack, missing permission, missing entitlement, traffic unavailable, matrix unavailable, optimization unavailable, EV/freight unavailable, premium profile unavailable, attribution unavailable, and disabled host/network capability.
- [x] 3.5 Implement approval behavior for precise private routes, retained route artifacts, high-volume matrices, regulated region/data-boundary crossings, freight/hazmat constraints, externally shared route artifacts, and long-running optimization jobs.
- [x] 3.6 Add tests proving denied, unavailable, unsupported, quota, approval-required, no-route, ambiguous, retention-denied, attribution-missing, conflict, stale-version, missing-entitlement, and disabled-network paths do not call concrete providers or emit side effects.

## 4. Service Runtime Provider Implementation

- [x] 4.1 Implement or bind route service provider behind the service runtime; do not construct providers from SDK, shell, kernel, or application code.
- [x] 4.2 Add a deterministic unavailable provider that returns complete descriptor metadata, health state, command availability, retention diagnostics, attribution diagnostics, and typed unavailable/unsupported diagnostics.
- [x] 4.3 Add mock provider support for provider inspection, profile discovery, request validation, route planning, ETA estimation, route inspection, matrix planning/request/status/cancel, optimization planning/request/status/cancel, retention inspection, attribution inspection, and artifact handle metadata.
- [x] 4.4 Add lifecycle, health, snapshot, shutdown, timeout, cancellation, pagination where applicable, async matrix/optimization behavior, idempotency, stale-version diagnostics, retention diagnostics, attribution diagnostics, quota diagnostics, and rate-limit diagnostics.
- [x] 4.5 Add Strategy implementations for provider adapters, profile mapping, constraint mapping, geometry mapping, metric mapping, maneuver mapping, matrix behavior, optimization behavior, retention handling, attribution resolution, artifact behavior, redaction, and unavailable behavior.
- [x] 4.6 Add explicit state machines for matrix jobs, optimization jobs, route artifacts, provider lifecycle, retention modes, and route freshness.
- [x] 4.7 Add side-effect safety support for idempotency keys, coordinate precision enforcement, waypoint/matrix size bounds, async cancellation, artifact retention, retention-policy validation, attribution validation, and non-mutating plan/validate commands.
- [x] 4.8 Add provider capability reporting for available, degraded, preview, unavailable, unsupported, retired, profile-limited, traffic-limited, matrix-limited, optimization-limited, EV-limited, freight-limited, retention-limited, attribution-limited, quota-limited, and rate-limited states.

## 5. SDK, Admission, ABI, And Examples

- [x] 5.1 Extend SDK discovery for `pack.location.route.v1` with command schemas, permission scopes, profile support, constraint support, traffic support, EV/freight support, matrix limits, optimization limits, geometry formats, retention modes, attribution requirements, examples, availability, diagnostics, documentation link, provider class, compatibility hash, and redaction profile.
- [x] 5.2 Extend application admission so required declarations block readiness when unavailable and optional declarations degrade explicitly with effective capability mementos.
- [x] 5.3 Add SDK command helper builders for all `route.*` commands; helpers must only build canonical traced service calls and must never construct providers, hold credentials, call provider APIs directly, geocode addresses, search places, render maps, capture device location, run fleet dispatch logic, settle tolls, or bypass retention/policy.
- [x] 5.4 Extend WASM/app ABI descriptors so applications can discover route commands, declare permissions, receive unavailable diagnostics, and submit typed service calls through the canonical execution path.
- [x] 5.5 Add generic app-facing examples for profile discovery, route validation, route planning, ETA estimation, matrix request/status, optimization request/status, route inspection, retention inspection, attribution inspection, artifact inspection, and unavailable diagnostics.
- [x] 5.6 Add provider-unavailable, missing-permission, missing-entitlement, no-route, ambiguous-route, retention-denied, attribution-missing, profile-unsupported, constraint-unsupported, matrix-quota-exceeded, optimization-timeout, network-denied, and artifact-denied examples that avoid provider names, credentials, raw provider payloads, private routes, unbounded geometries, route batches, and application business workflows.

## 6. Trace, Audit, Replay, And Boundary Gates

- [x] 6.1 Emit sanitized declaration, admission, discovery, request validation, policy, resource, entitlement, approval, service-call, route plan, ETA estimate, matrix lifecycle, optimization lifecycle, route inspection, retention inspection, attribution inspection, artifact, health, snapshot, unavailable, conflict, and failure events.
- [x] 6.2 Ensure traces, audits, snapshots, SDK diagnostics, and examples exclude raw credentials, API keys, access tokens, raw provider responses, private route batches, unbounded geometry dumps, private manifests, package bytes, private keys, signatures, and unsanitized location/route data.
- [x] 6.3 Add replay tests proving every `route.*` command is trace-addressable through the canonical service path and snapshots contain enough bounded metadata for recovery diagnostics.
- [x] 6.4 Add dependency gates proving kernel, SDK, shells, and generic application framework do not import concrete Google Routes, Mapbox, HERE, TomTom, Azure Maps, Esri, OSRM, Valhalla, GraphHopper, offline route, credential, or route provider adapters.
- [x] 6.5 Add no-direct-provider-call gates proving SDK helpers, WASM ABI handlers, app admission, web, CLI, and frontend paths route through descriptor-owned service commands.
- [x] 6.6 Add boundary tests proving optional provider absence returns structured unavailable diagnostics and never crashes, hangs, silently falls back, calculates routes, computes matrices, optimizes waypoints, uses credentials, contacts providers, or fakes success.
- [x] 6.7 Run `openspec validate add-pack-location-route --strict`, targeted cargo tests, boundary gates, file-size gates, retention/attribution conformance tests, and audit replay checks before marking implementation complete.

## 7. Developer Documentation

- [x] 7.1 Create `docs/developer-packs/location/route.md` with purpose, capability model, manifest declaration, required versus optional behavior, permissions, waypoints, travel profiles, constraints, route plans, legs, steps, maneuvers, geometry, metrics, matrices, optimization jobs, retention policy, attribution, artifacts, unavailable diagnostics, provider replacement, and operational limits.
- [x] 7.2 Document all command DTOs and result DTOs with field-level explanations, idempotency semantics, async job behavior, profile handling, constraint handling, traffic semantics, EV/freight limitations, matrix limits, optimization objective semantics, geometry formats, retention/storage behavior, attribution requirements, redaction behavior, approval behavior, artifact retention behavior, and structured error codes.
- [x] 7.3 Document supplier/API mapping: Google Routes, Mapbox Navigation APIs, HERE Routing/Matrix/Tour Planning, TomTom Routing, Azure Maps Route, Esri Network Analysis, OSRM, Valhalla, and GraphHopper concepts mapped to Macaca abstractions, including what is intentionally not exposed as OS semantics.
- [x] 7.4 Add generic examples for required declaration, optional declaration, route planning, ETA estimation, matrix calculation, waypoint optimization, route inspection, retention inspection, attribution inspection, artifact inspection, unavailable provider, denied permission, no-route, ambiguous-route, retention-denied, and quota-exceeded handling.
- [x] 7.5 Add conformance checklist and test guidance for provider authors: descriptor completeness, request scope validation, idempotency, profile mapping, constraint mapping, geometry mapping, metric mapping, matrix state machine, optimization state machine, retention enforcement, attribution completeness, resource bounds, policy hooks, trace/audit events, unavailable behavior, snapshot/replay, and no raw payload leakage.
- [x] 7.6 Cross-link the guide from SDK discovery metadata and the industrial pack catalog index before marking `add-pack-location-route` complete.
