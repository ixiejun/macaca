## ADDED Requirements

### Requirement: Macaca SHALL provide Location Geocode Pack as a serviceized capability

Macaca SHALL provide `pack.location.geocode.v1` as a provider-neutral
industrial pack for forward geocoding, reverse geocoding, structured address
parsing, address normalization, candidate ranking, confidence diagnostics,
batch geocoding, retention policy evidence, attribution, and artifact handles.
The pack SHALL be declared by applications, resolved by admission/catalog
services, and invoked only through typed service commands.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.location.geocode.v1` as required and a geocode service provider is registered, healthy, entitled, permission-compatible, policy-admissible, and retention/attribution-capable
- **THEN** admission SHALL expose the pack in the effective capability set with command schemas, permission scopes, policy template hash, resource limits, approval rules, retention metadata, attribution metadata, provider health metadata, compatibility metadata, documentation links, and replay metadata
- **AND** SDK discovery SHALL mark callable commands as available without exposing provider secrets, API keys, access tokens, raw provider payloads, private address lists, unbounded batch data, or unsanitized location/address data

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.location.geocode.v1` as required but provider, permission, entitlement, resource, host support, network support, retention support, attribution support, forward support, reverse support, structured support, or batch support is absent
- **THEN** admission SHALL block readiness with structured unavailable or denied diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back, contact another undeclared provider, geocode addresses, or fake success

#### Scenario: Optional declaration is unavailable
- **WHEN** an application declares `pack.location.geocode.v1` as optional and the pack is unavailable
- **THEN** admission SHALL produce an explicit degraded effective capability report with unavailable reason codes and command-level availability
- **AND** SDK command helpers SHALL refuse to build callable service calls for unavailable commands

### Requirement: Location Geocode Pack SHALL expose supplier-grade geocode contracts

`pack.location.geocode.v1` SHALL expose provider-neutral DTOs for geocode
scopes, forward queries, reverse queries, structured address components,
candidate geometry, precision classes, confidence diagnostics, retention
policies, attribution bundles, batch jobs, artifacts, version metadata,
freshness metadata, redaction metadata, and provider capability metadata.

#### Scenario: Provider schema is discovered
- **WHEN** SDK discovery or `geocode.discover_schema` inspects the pack
- **THEN** Macaca SHALL return command schemas, permission scopes, forward/reverse support, structured address support, batch support, supported countries/languages, precision classes, confidence fields, filters, retention modes, attribution requirements, lifecycle state, provider health, redaction profile, and compatibility hash
- **AND** the schema SHALL be provider-neutral even when backed by Google Maps, Mapbox, HERE, TomTom, Esri, Azure Maps, Apple CLGeocoder, Nominatim/Pelias, offline, built-in, plugin, remote, mock, or unavailable providers

#### Scenario: Address components are normalized
- **WHEN** `geocode.normalize_address` receives free-form or structured address input
- **THEN** Macaca SHALL return `AddressComponentSet` with bounded fields for house number, street, unit, neighborhood, locality, district, region, postal code, country, country code, formatted labels, administrative levels, and missing/ambiguous component metadata
- **AND** normalization SHALL NOT assert legal address validity, KYC status, deliverability, or application-specific business eligibility

#### Scenario: Confidence is inspected
- **WHEN** `geocode.inspect_confidence` is invoked for a candidate
- **THEN** Macaca SHALL return normalized score, provider score reference, match type, partial match flag, ambiguity class, result rank, component match summary, precision class, and bounded explanation codes
- **AND** Macaca SHALL NOT pretend all providers share identical score semantics or expose raw provider payloads

### Requirement: Location Geocode Pack commands SHALL use canonical typed service calls

Every `geocode.*` operation SHALL be represented as a typed command/result DTO
and SHALL traverse the canonical service runtime path with trace, policy,
resource, entitlement, approval, health, snapshot, timeout, cancellation,
retention, attribution, idempotency, redaction, and structured error behavior.

#### Scenario: Query is validated
- **WHEN** `geocode.validate_query` validates a forward or reverse geocode request
- **THEN** Macaca SHALL check address/coordinate shape, language, region, country filters, bbox/proximity filters, result type filters, precision, retention intent, attribution requirement, policy, entitlement, resource budget, and provider support
- **AND** no provider side effect SHALL occur during the validation command

#### Scenario: Forward geocode succeeds
- **WHEN** `geocode.forward` is invoked for a declared, validated, and policy-allowed address query
- **THEN** Macaca SHALL route the command through SDK/facade helpers into the service runtime and geocode service provider
- **AND** the result SHALL include bounded candidates with component sets, geometry, precision, confidence, provider reference, source class, attribution bundle, retention policy, freshness, and redaction metadata

#### Scenario: Reverse geocode succeeds
- **WHEN** `geocode.reverse` is invoked for a declared, validated, and policy-allowed coordinate query
- **THEN** Macaca SHALL enforce coordinate precision, region policy, radius, result type filters, retention intent, attribution, resource limits, and provider support before returning candidates
- **AND** candidate results SHALL include only approved coordinate precision and sanitized address components

#### Scenario: Geocode request is denied before provider call
- **WHEN** policy, permission, entitlement, approval, resource, network, coordinate precision, retention, attribution, or provider capability checks reject a request
- **THEN** Macaca SHALL return a typed denied, approval-required, unavailable, unsupported, quota, retention-denied, or attribution-missing result before invoking the concrete provider
- **AND** the audit trail SHALL include bounded reason codes, hashes, counters, and sanitized references only

### Requirement: Location Geocode Pack SHALL support batch geocoding safely

`pack.location.geocode.v1` SHALL support bounded batch geocoding with
idempotency, partial-result handling, cancellation, artifact handles, retention
policy enforcement, and replayable evidence.

#### Scenario: Batch job is planned
- **WHEN** `geocode.plan_batch` validates a batch geocode request
- **THEN** Macaca SHALL check input count, address sensitivity, coordinate precision, retention intent, provider batch support, quota, timeout, artifact policy, approval requirements, and entitlement
- **AND** no provider side effect SHALL occur during the plan command

#### Scenario: Batch job is requested
- **WHEN** `geocode.request_batch` is invoked with a valid idempotency key and policy-allowed inputs
- **THEN** Macaca SHALL route through the canonical service path and return `GeocodeBatchJob` metadata with job handle, input count, completed count, failed count, partial-result state, artifact handles, retention policy, cancellation state, and replay cursor
- **AND** raw input address lists and raw provider payloads SHALL remain excluded from traces, snapshots, and SDK diagnostics

#### Scenario: Batch job is cancelled
- **WHEN** `geocode.cancel_batch` is invoked for an active or completed batch job
- **THEN** Macaca SHALL return typed cancelled, conflict, unsupported, unavailable, or success diagnostics according to provider state
- **AND** cancellation evidence SHALL remain bounded and replayable

### Requirement: Location Geocode Pack SHALL enforce retention and attribution semantics

`pack.location.geocode.v1` SHALL expose and enforce provider-neutral retention
policies and attribution requirements for candidates, batches, and artifacts.

#### Scenario: Retention policy is inspected
- **WHEN** `geocode.inspect_retention` is invoked for a provider, query, candidate, batch, or artifact reference
- **THEN** Macaca SHALL return temporary/permanent mode, storage allowed flag, cache TTL class, derived-data restrictions, attribution requirement, provider terms reference, and redaction class
- **AND** raw provider terms payloads and credentials SHALL remain excluded

#### Scenario: Permanent storage is denied
- **WHEN** a command requests permanent retention but provider capability, entitlement, policy, or attribution requirements do not allow it
- **THEN** Macaca SHALL return a typed retention-denied, denied, unavailable, or unsupported result before storing candidate or batch artifacts
- **AND** temporary geocode results SHALL NOT be silently promoted to permanent stored data

#### Scenario: Attribution is missing
- **WHEN** a provider cannot produce required attribution metadata for candidates, batches, or artifacts
- **THEN** Macaca SHALL return a typed attribution-missing, unavailable, or unsupported result
- **AND** Macaca SHALL NOT return unattributed geocode results as successful results

### Requirement: Location Geocode Pack SHALL expose health, snapshots, and replayable evidence

`pack.location.geocode.v1` SHALL expose descriptor metadata, service health,
command availability, provider capability hashes, schema hashes,
retention-policy hashes, attribution hashes, snapshots, replay pointers, and
sanitized audit events for all operations.

#### Scenario: Snapshot is recorded
- **WHEN** the service runtime records a `pack.location.geocode.v1` snapshot
- **THEN** the snapshot SHALL include descriptor version, provider capability hash, command availability, provider health, schema hash, supported precision classes, retention-policy hash, attribution hash, batch summary, resource counters, artifact summaries, event cursors, and sanitized replay pointers
- **AND** it SHALL exclude raw credentials, API keys, access tokens, raw provider responses, private address lists, unbounded batch dumps, private manifests, package bytes, private keys, signatures, and unsanitized location/address data

#### Scenario: Trace replay inspects a command
- **WHEN** trace replay inspects any `geocode.*` command
- **THEN** replay SHALL prove declaration, admission, policy, resource, entitlement, approval when required, retention validation, attribution validation, service runtime routing, provider class, result variant, and sanitized audit evidence
- **AND** replay SHALL NOT require provider-specific logs, raw provider responses, application-specific address workflow state, or raw address lists

#### Scenario: Provider is unavailable
- **WHEN** the active provider is unavailable, disabled, retired, degraded, command-limited, forward-limited, reverse-limited, structured-limited, batch-limited, retention-limited, attribution-limited, quota-limited, or rate-limited
- **THEN** SDK discovery, health, snapshots, and command results SHALL expose structured diagnostics with stable reason codes
- **AND** Macaca SHALL NOT crash, hang, silently fall back, contact undeclared providers, geocode addresses, reverse geocode coordinates, process batches, or fake success

### Requirement: Location Geocode Pack implementation SHALL preserve Macaca boundaries

The `pack.location.geocode.v1` implementation SHALL remain owned by geocode
service providers and service-runtime contracts. The microkernel, SDK, shells,
and generic application framework SHALL remain provider-neutral and free of
application-specific, supplier-specific, place-search-specific,
routing-specific, map-rendering-specific, timezone-specific,
device-location-specific, verification-specific, or workflow-specific routing
branches.

#### Scenario: Boundary gates scan the implementation
- **WHEN** dependency, no-direct-provider-call, and canonical execution-path gates scan the implementation
- **THEN** they SHALL find no concrete Google, Mapbox, HERE, TomTom, Esri, Azure Maps, Apple CLGeocoder, Nominatim, Pelias, offline geocoder, credential, or geocode provider imports in the microkernel, SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned service registrations and typed service commands

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, mock, or unavailable provider is selected for `pack.location.geocode.v1`
- **THEN** callers SHALL observe the same provider-neutral command/result contract
- **AND** trace/audit evidence SHALL identify only sanitized provider class, descriptor metadata, capability hashes, retention hashes, attribution hashes, and bounded result codes rather than provider-specific business branches

#### Scenario: Adjacent pack boundary is tested
- **WHEN** boundary tests exercise place search, route calculation, map rendering, timezone lookup, device location capture, address verification/KYC, delivery optimization, emergency workflows, and application address business rules
- **THEN** `pack.location.geocode.v1` SHALL expose only address/coordinate candidates, confidence, retention, attribution, artifacts, and policy decisions for those concerns
- **AND** it SHALL NOT implement those adjacent pack behaviors internally

### Requirement: Location Geocode Pack SHALL include detailed developer documentation

The implementation of `pack.location.geocode.v1` SHALL include detailed
developer documentation under `docs/developer-packs/location/geocode.md` and
SHALL link that documentation from SDK discovery metadata and the industrial
pack catalog index.

#### Scenario: Developer reads the pack guide
- **WHEN** a developer opens `docs/developer-packs/location/geocode.md`
- **THEN** the guide SHALL explain purpose, non-goals, manifest declaration, required versus optional behavior, permission scopes, approval behavior, command DTOs, result DTOs, forward/reverse queries, structured addresses, candidates, geometry, precision classes, confidence, retention policy, attribution, batch jobs, artifacts, unavailable diagnostics, provider replacement, and operational limits
- **AND** examples SHALL use synthetic data and generic handles rather than provider names, credentials, API keys, private addresses, private coordinates, raw provider payloads, unbounded batches, application names, or business workflows

#### Scenario: Provider author reads conformance guidance
- **WHEN** a provider author reads the geocode pack documentation
- **THEN** the guide SHALL include a supplier/API mapping for Google Maps Geocoding, Mapbox Geocoding, HERE Geocoding and Search, TomTom Geocoding, Esri World Geocoding, Azure Maps Search, Apple CLGeocoder, Nominatim, and Pelias concepts
- **AND** it SHALL include conformance checks for descriptor completeness, query scope validation, idempotency, precision mapping, confidence mapping, retention enforcement, attribution completeness, batch state machine, resource bounds, policy hooks, trace/audit events, unavailable behavior, snapshot/replay, and no raw payload leakage
