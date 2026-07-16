## 1. Research, Governance, And Scope

- [x] 1.1 Re-read architecture governance, microkernel boundaries, serviceization allowlist, design-pattern guidance, and the industrial catalog umbrella proposal before implementation.
- [x] 1.2 Record supplier/API findings for Google Maps Geocoding, Mapbox Geocoding, HERE Geocoding and Search, TomTom Geocoding, Esri World Geocoding, Azure Maps Search, Apple CLGeocoder, Nominatim, and Pelias-style providers.
- [x] 1.3 Confirm boundary decisions with adjacent packs: maps owns tiles/rendering, route owns path calculation, place-search owns POI/search semantics, timezone owns timezone lookup, device owns location capture, identity owns KYC/identity flows, and applications own address business workflows.
- [x] 1.4 Inventory existing descriptors, SDK clients, location services, artifact services, service-runtime decorators, mock providers, and unavailable providers that can back geocode service implementation.
- [x] 1.5 Record GitNexus CRITICAL/HIGH findings as memo only before implementation commits, without letting advisory severity block this proposal track.

## 2. Contract, Descriptor, And Schema

- [x] 2.1 Define `pack.location.geocode.v1` descriptor metadata for pack id, family, lifecycle, stability, command schemas, permissions, policy template, resource budget, approval rules, retention rules, attribution rules, data governance, SDK metadata, docs link, compatibility, health, and diagnostics.
- [x] 2.2 Define provider-neutral DTOs for `GeocodeScope`, `GeocodeQuery`, `ReverseGeocodeQuery`, `AddressComponentSet`, `GeocodeGeometry`, `LocationPrecisionClass`, `GeocodeConfidence`, `GeocodeCandidate`, `GeocodeRetentionPolicy`, `GeocodeBatchJob`, and `GeocodeArtifactHandle`.
- [x] 2.3 Define command DTOs for `geocode.inspect_provider`, `geocode.discover_schema`, `geocode.validate_query`, `geocode.forward`, `geocode.reverse`, `geocode.normalize_address`, `geocode.inspect_confidence`, `geocode.plan_batch`, `geocode.request_batch`, `geocode.inspect_batch`, `geocode.cancel_batch`, `geocode.inspect_retention`, `geocode.inspect_attribution`, and `geocode.get_artifact`.
- [x] 2.4 Define typed success, partial, approval-required, denied, unavailable, unsupported, conflict, ambiguous, no-match, stale-version, quota, rate-limited, timeout, cancelled, retention-denied, attribution-missing, and failure result DTOs.
- [x] 2.5 Add descriptor hashing, schema-version compatibility, command-availability hashing, supported-country/language hashing, precision-class hashing, retention-policy hashing, attribution-bundle hashing, and redaction-profile hashing.
- [x] 2.6 Add unit tests for valid descriptors, rejected descriptors, missing command schemas, invalid permission scopes, unsupported precision, unsupported country/language, retention mismatch, missing attribution metadata, unstable hashes, incompatible versions, and redaction metadata.

## 3. Permission, Policy, Resource, Entitlement, And Approval

- [x] 3.1 Implement declaration validation for scopes: `location.geocode.forward`, `location.geocode.reverse`, `location.geocode.normalize`, `location.geocode.confidence.read`, `location.geocode.batch`, `location.geocode.retention.read`, `location.geocode.attribution.read`, and `location.geocode.artifact.read`.
- [ ] 3.2 Implement policy checks for caller subject, application id, tenant id, command, address sensitivity, coordinate precision, country/region policy, retention intent, batch size, result field mask, provider storage mode, attribution requirement, resource budget, approval state, and entitlement state before provider calls.
- [ ] 3.3 Implement resource reservation for query count, batch size, candidate count, address length, component count, provider quota, network budget, timeout, artifact size, retained snapshots, retained artifacts, and event volume.
- [ ] 3.4 Implement entitlement checks and structured unavailable/denied diagnostics for missing provider, disabled pack, missing permission, missing entitlement, forward unsupported, reverse unsupported, structured unsupported, batch unsupported, permanent storage unsupported, high-precision unavailable, attribution unavailable, and disabled host/network capability.
- [ ] 3.5 Implement approval behavior for private residential addresses, precise reverse geocoding, retained batch artifacts, permanent storage modes, regulated region/data-boundary crossings, and high-volume address lists.
- [ ] 3.6 Add tests proving denied, unavailable, unsupported, quota, approval-required, ambiguous, no-match, retention-denied, attribution-missing, conflict, stale-version, missing-entitlement, and disabled-network paths do not call concrete providers or emit side effects.

## 4. Service Runtime Provider Implementation

- [ ] 4.1 Implement or bind geocode service provider behind the service runtime; do not construct providers from SDK, shell, kernel, or application code.
- [x] 4.2 Add a deterministic unavailable provider that returns complete descriptor metadata, health state, command availability, retention diagnostics, attribution diagnostics, and typed unavailable/unsupported diagnostics.
- [ ] 4.3 Add mock provider support for provider inspection, schema discovery, query validation, forward geocode, reverse geocode, normalization, confidence inspection, batch planning/request/status/cancel, retention inspection, attribution inspection, and artifact handle metadata.
- [ ] 4.4 Add lifecycle, health, snapshot, shutdown, timeout, cancellation, pagination where applicable, async batch behavior, idempotency, stale-version diagnostics, retention diagnostics, attribution diagnostics, quota diagnostics, and rate-limit diagnostics.
- [ ] 4.5 Add Strategy implementations for provider adapters, address component mapping, confidence mapping, precision mapping, retention handling, attribution resolution, batch behavior, artifact behavior, redaction, and unavailable behavior.
- [ ] 4.6 Add explicit state machines for batch jobs, batch artifacts, provider lifecycle, retention modes, and candidate freshness.
- [ ] 4.7 Add side-effect safety support for idempotency keys, coordinate precision enforcement, address-list size bounds, batch cancellation, artifact retention, retention-policy validation, attribution validation, and non-mutating plan/validate commands.
- [ ] 4.8 Add provider capability reporting for available, degraded, preview, unavailable, unsupported, retired, forward-limited, reverse-limited, structured-limited, batch-limited, retention-limited, attribution-limited, quota-limited, and rate-limited states.

## 5. SDK, Admission, ABI, And Examples

- [x] 5.1 Extend SDK discovery for `pack.location.geocode.v1` with command schemas, permission scopes, forward/reverse support, structured address support, batch support, supported countries/languages, precision classes, confidence fields, retention modes, attribution requirements, examples, availability, diagnostics, documentation link, provider class, compatibility hash, and redaction profile.
- [x] 5.2 Extend application admission so required declarations block readiness when unavailable and optional declarations degrade explicitly with effective capability mementos.
- [x] 5.3 Add SDK command helper builders for all `geocode.*` commands; helpers must only build canonical traced service calls and must never construct providers, hold credentials, call provider APIs directly, search places, calculate routes, render maps, capture device location, verify identity documents, or bypass retention/policy.
- [ ] 5.4 Extend WASM/app ABI descriptors so applications can discover geocode commands, declare permissions, receive unavailable diagnostics, and submit typed service calls through the canonical execution path.
- [x] 5.5 Add generic app-facing examples for schema discovery, query validation, forward geocode, reverse geocode, address normalization, confidence inspection, batch geocode, retention inspection, attribution inspection, artifact inspection, and unavailable diagnostics.
- [x] 5.6 Add provider-unavailable, missing-permission, missing-entitlement, no-match, ambiguous, retention-denied, attribution-missing, country-unsupported, high-precision-denied, batch-quota-exceeded, network-denied, and artifact-denied examples that avoid provider names, credentials, raw provider payloads, private addresses, private coordinates, unbounded address lists, and application business workflows.

## 6. Trace, Audit, Replay, And Boundary Gates

- [ ] 6.1 Emit sanitized declaration, admission, discovery, query validation, policy, resource, entitlement, approval, service-call, forward result, reverse result, normalization, confidence inspection, batch lifecycle, retention inspection, attribution inspection, artifact, health, snapshot, unavailable, conflict, and failure events.
- [ ] 6.2 Ensure traces, audits, snapshots, SDK diagnostics, and examples exclude raw credentials, API keys, access tokens, raw provider responses, private address lists, unbounded batch dumps, private manifests, package bytes, private keys, signatures, and unsanitized location/address data.
- [ ] 6.3 Add replay tests proving every `geocode.*` command is trace-addressable through the canonical service path and snapshots contain enough bounded metadata for recovery diagnostics.
- [ ] 6.4 Add dependency gates proving kernel, SDK, shells, and generic application framework do not import concrete Google, Mapbox, HERE, TomTom, Esri, Azure Maps, Apple CLGeocoder, Nominatim, Pelias, offline geocoder, credential, or geocode provider adapters.
- [ ] 6.5 Add no-direct-provider-call gates proving SDK helpers, WASM ABI handlers, app admission, web, CLI, and frontend paths route through descriptor-owned service commands.
- [ ] 6.6 Add boundary tests proving optional provider absence returns structured unavailable diagnostics and never crashes, hangs, silently falls back, geocodes addresses, reverse geocodes coordinates, processes batches, uses credentials, contacts providers, or fakes success.
- [ ] 6.7 Run `openspec validate add-pack-location-geocode --strict`, targeted cargo tests, boundary gates, file-size gates, retention/attribution conformance tests, and audit replay checks before marking implementation complete.

## 7. Developer Documentation

- [x] 7.1 Create `docs/developer-packs/location/geocode.md` with purpose, capability model, manifest declaration, required versus optional behavior, permissions, forward/reverse queries, structured addresses, candidates, geometry, precision classes, confidence, retention policy, attribution, batch jobs, artifacts, unavailable diagnostics, provider replacement, and operational limits.
- [x] 7.2 Document all command DTOs and result DTOs with field-level explanations, idempotency semantics, batch behavior, supported filters, country/language handling, precision behavior, confidence semantics, retention/storage behavior, attribution requirements, redaction behavior, approval behavior, artifact retention behavior, and structured error codes.
- [x] 7.3 Document supplier/API mapping: Google Maps Geocoding, Mapbox Geocoding, HERE Geocoding and Search, TomTom Geocoding, Esri World Geocoding, Azure Maps Search, Apple CLGeocoder, Nominatim, and Pelias concepts mapped to Macaca abstractions, including what is intentionally not exposed as OS semantics.
- [x] 7.4 Add generic examples for required declaration, optional declaration, forward geocode, reverse geocode, normalization, confidence inspection, batch geocode, retention inspection, attribution inspection, artifact inspection, unavailable provider, denied permission, no-match, ambiguous, retention-denied, and quota-exceeded handling.
- [x] 7.5 Add conformance checklist and test guidance for provider authors: descriptor completeness, query scope validation, idempotency, precision mapping, confidence mapping, retention enforcement, attribution completeness, batch state machine, resource bounds, policy hooks, trace/audit events, unavailable behavior, snapshot/replay, and no raw payload leakage.
- [x] 7.6 Cross-link the guide from SDK discovery metadata and the industrial pack catalog index before marking `add-pack-location-geocode` complete.
