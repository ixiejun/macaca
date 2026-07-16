## ADDED Requirements

### Requirement: Macaca SHALL provide Location Timezone as a serviceized industrial pack

Macaca SHALL provide `pack.location.timezone.v1` as a provider-neutral industrial pack for coordinate-to-zone lookup, time-zone identifier normalization, offset calculation, transition listing, instant conversion, local-time gap/fold resolution, localized display names, database inspection, and identifier mapping. The pack SHALL be declared by applications, resolved by admission/catalog services, and invoked only through typed service commands.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.location.timezone.v1` as required and the timezone service is registered, healthy, entitled, policy-admissible, database-fresh, and command-compatible
- **THEN** admission SHALL expose the pack in the effective capability set with command schemas, permission scopes, provider classes, database versions, freshness metadata, policy template, availability, health, and replay metadata
- **AND** SDK discovery SHALL mark callable commands as available without exposing provider secrets, credentials, raw provider payloads, raw polygon geometry, or unbounded diagnostics

#### Scenario: Required declaration is unavailable or stale
- **WHEN** an application declares `pack.location.timezone.v1` as required but provider, command support, permission, entitlement, resource, host support, database freshness, region policy, or identifier support is absent
- **THEN** admission SHALL block readiness with structured unavailable, stale-database, unsupported, or denied diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back to another provider, or fake success

#### Scenario: Optional declaration is degraded
- **WHEN** an application declares `pack.location.timezone.v1` as optional and the pack is unavailable, stale, or command-limited
- **THEN** admission SHALL produce an explicit degraded effective capability report with bounded reason codes and dataset versions when known
- **AND** SDK command helpers SHALL refuse to build callable service calls for unavailable commands

### Requirement: Location Timezone SHALL expose supplier-grade provider-neutral commands

`pack.location.timezone.v1` SHALL expose typed commands for `timezone.lookup_by_coordinates`, `timezone.resolve_zone`, `timezone.get_offset`, `timezone.list_transitions`, `timezone.convert_instant`, `timezone.resolve_local_time`, `timezone.get_display_names`, `timezone.inspect_database`, and `timezone.inspect_mapping`.

#### Scenario: Coordinate lookup returns versioned zone candidates
- **WHEN** a declared and policy-allowed caller invokes `timezone.lookup_by_coordinates` with coordinate, timestamp, precision class, max candidates, and trace context
- **THEN** Macaca SHALL route the command through SDK/facade helpers into service runtime and the active timezone provider
- **AND** the result SHALL include primary zone, candidate zones, confidence, boundary-distance class, ambiguity reason, database version, boundary provenance, and attribution/provenance metadata

#### Scenario: Zone resolution normalizes identifiers
- **WHEN** a caller invokes `timezone.resolve_zone` with an IANA id, alias/link, Windows id, CLDR id, or provider-specific id
- **THEN** Macaca SHALL return a canonical IANA id when known, mapping metadata, alias/deprecation status, version metadata, and confidence
- **AND** unsupported identifiers SHALL return typed unsupported or invalid-zone diagnostics

#### Scenario: Offset calculation is instant-aware
- **WHEN** a caller invokes `timezone.get_offset` with zone id and instant
- **THEN** Macaca SHALL return total offset, raw/standard offset, daylight offset, abbreviation, DST flag, effective interval, rule provenance, and database version
- **AND** stale or unsupported rule data SHALL be represented explicitly

#### Scenario: Transition listing is bounded
- **WHEN** a caller invokes `timezone.list_transitions`
- **THEN** Macaca SHALL require a bounded time range and page size
- **AND** it SHALL return transition instants, local before/after times, offset before/after values, abbreviations, gap/fold classification, and rule provenance

#### Scenario: Instant conversion returns reproducible local time
- **WHEN** a caller invokes `timezone.convert_instant`
- **THEN** Macaca SHALL convert the instant into local date/time for the requested zone using the declared database version or effective provider version
- **AND** the result SHALL include offset, abbreviation, local date/time, zone id, calendar fields, and database version

#### Scenario: Local time resolution handles gaps and folds
- **WHEN** a caller invokes `timezone.resolve_local_time` with a local date/time and zone id
- **THEN** Macaca SHALL require a resolver strategy such as reject, earlier, later, compatible, or explicit offset
- **AND** nonexistent and ambiguous local times SHALL return explicit gap/fold diagnostics and candidate instants rather than silently choosing an instant without evidence

#### Scenario: Display names are localized with provenance
- **WHEN** a caller invokes `timezone.get_display_names`
- **THEN** Macaca SHALL return generic, standard, daylight, exemplar city, GMT format, metazone, locale fallback chain, and display-name database version when available
- **AND** unsupported locale or display-name data SHALL return typed partial or unsupported diagnostics

#### Scenario: Database inspection exposes freshness
- **WHEN** a caller invokes `timezone.inspect_database`
- **THEN** Macaca SHALL return tzdb version, boundary dataset version, display-name dataset version, source class, release dates, freshness status, health, and update recommendation
- **AND** it SHALL not expose raw database paths, raw polygon data, credentials, or host-private paths

#### Scenario: Mapping inspection explains identifier support
- **WHEN** a caller invokes `timezone.inspect_mapping`
- **THEN** Macaca SHALL return supported identifier systems, canonical mappings, alias/link handling, Windows/IANA mapping confidence, CLDR metadata, unsupported ids, and version metadata
- **AND** OS-layer code SHALL NOT encode provider-specific mapping tables outside descriptor/provider data

### Requirement: Location Timezone DTOs SHALL model correctness, ambiguity, and provenance

The pack SHALL define provider-neutral DTOs for command context, coordinate queries, zones, lookup results, offsets, transitions, local-time resolutions, display names, database information, boundary provenance, identifier mappings, and structured errors. Provider adapters SHALL map supplier data into these DTOs and SHALL retain enough version/provenance evidence for audit and replay.

#### Scenario: Lookup result records boundary ambiguity
- **WHEN** coordinate lookup succeeds near a time-zone boundary or with low-precision coordinates
- **THEN** the result SHALL include confidence, candidate zones, boundary-distance class, ambiguity reason, data version, and boundary provenance
- **AND** callers SHALL be able to distinguish precise, approximate, ambiguous, and policy-coarsened results

#### Scenario: Offset records rule provenance
- **WHEN** an offset is calculated
- **THEN** the `TimezoneOffset` DTO SHALL include total offset seconds, raw/standard offset seconds, daylight offset seconds, abbreviation, DST flag, effective interval, rule id/hash, and database version
- **AND** it SHALL not expose raw provider rule tables unless modeled as sanitized hashes or version identifiers

#### Scenario: Local resolution returns candidates
- **WHEN** a local datetime is ambiguous during a fold
- **THEN** `TimezoneLocalResolution` SHALL return all candidate instants, offsets, selected strategy, selected instant if applicable, and diagnostics
- **AND** the provider SHALL NOT silently discard candidate instants

#### Scenario: Nonexistent local time is explicit
- **WHEN** a local datetime falls into a DST gap
- **THEN** Macaca SHALL return nonexistent-local-time diagnostics or a strategy-selected instant with evidence
- **AND** the selected strategy SHALL be recorded in result and audit metadata

#### Scenario: Errors are stable across providers
- **WHEN** providers return invalid zone, stale database, ambiguous boundary, unsupported mapping, quota, remote failure, or host unavailable states
- **THEN** Macaca SHALL map them to stable `TimezoneError` variants
- **AND** provider-specific diagnostics SHALL be sanitized and bounded

### Requirement: Location Timezone SHALL enforce permission, policy, resource, entitlement, and approval gates

Every command in `pack.location.timezone.v1` SHALL run through permission, policy, resource, entitlement, metering, and approval decorators before provider side effects or external calls.

#### Scenario: Missing permission denies before provider dispatch
- **WHEN** an application invokes a command without required scope such as `location.timezone.lookup.read`, `location.timezone.offset.read`, `location.timezone.names.read`, or `location.timezone.database.inspect`
- **THEN** Macaca SHALL return a typed denied result before invoking the concrete provider
- **AND** the audit event SHALL include the bounded missing-scope code

#### Scenario: Coordinate precision policy coarsens evidence
- **WHEN** policy permits lookup but forbids exact-coordinate logging
- **THEN** provider dispatch MAY receive the permitted coordinate precision while trace/audit evidence stores only hashes, precision class, or coarse spatial class
- **AND** replay SHALL use stable evidence identifiers rather than raw coordinates

#### Scenario: Stale database policy denies results
- **WHEN** the effective tzdb, boundary, or display-name dataset is older than tenant or host policy allows
- **THEN** Macaca SHALL return stale-database diagnostics or require explicit approval depending on policy
- **AND** provider dispatch SHALL be skipped when policy requires denial

#### Scenario: Range policy blocks expensive transition listing
- **WHEN** transition-list range, page size, retained snapshot size, or remote quota class exceeds the effective resource budget
- **THEN** Macaca SHALL return a typed quota-exceeded result before provider dispatch
- **AND** resource counters SHALL be emitted in sanitized trace evidence

#### Scenario: Approval is required for sensitive lookup
- **WHEN** host policy marks a command sensitive because it uses exact coordinates, native host capabilities, external network disclosure, stale data override, or high-volume transition queries
- **THEN** Macaca SHALL require explicit approval evidence before dispatch
- **AND** denial or missing approval SHALL be traceable without leaking raw location data

### Requirement: Location Timezone SHALL preserve canonical service runtime execution

All callable operations SHALL traverse the canonical Macaca service path: application declaration, admission/effective capability projection, SDK/facade command construction, service runtime dispatch, decorators, provider adapter, structured result, trace/audit evidence, and replayable snapshot. SDK helpers SHALL NOT construct providers or create alternate execution paths.

#### Scenario: Command succeeds through the canonical path
- **WHEN** a declared and policy-allowed command is invoked
- **THEN** Macaca SHALL route it through SDK/facade helpers into service runtime dispatch and the active timezone provider adapter
- **AND** trace evidence SHALL show declaration, admission, policy, entitlement, resource, provider selection, command result, database version, and replay pointer events

#### Scenario: Provider is absent
- **WHEN** no provider is registered for `pack.location.timezone.v1`
- **THEN** the unavailable provider SHALL return structured unavailable diagnostics
- **AND** SDK discovery SHALL report unavailable state while preserving the same provider-neutral command/result contract

#### Scenario: Provider supports only a subset
- **WHEN** the active provider supports offset conversion but not coordinate boundary lookup or localized display names
- **THEN** SDK discovery SHALL mark unsupported commands as non-callable
- **AND** direct invocation SHALL return typed unsupported diagnostics without falling through to application-specific logic

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, host-native, offline-data, mock, or unavailable provider is selected
- **THEN** callers SHALL observe the same provider-neutral DTO contract
- **AND** OS-layer code SHALL identify only provider class, descriptor version, dataset version, and capability metadata in traces rather than branching on provider names

### Requirement: Location Timezone SHALL expose industrial SDK discovery and developer documentation

SDK discovery for `pack.location.timezone.v1` SHALL expose pack metadata, lifecycle, command schemas, DTO schemas, permission scopes, effective availability, provider classes, supported identifier systems, database versions, freshness, command capability matrix, policy templates, examples, diagnostics, compatibility, and documentation links. The implementation SHALL provide detailed developer documentation under `docs/developer-packs/location/timezone.md`.

#### Scenario: Developer inspects the pack
- **WHEN** SDK discovery inspects `pack.location.timezone.v1`
- **THEN** it SHALL return command namespace `timezone.*`, supported commands, required scopes, provider classes, database versions, freshness, supported identifier systems, policy templates, examples, lifecycle, health, diagnostics, compatibility metadata, and documentation URL
- **AND** examples SHALL use generic synthetic data rather than application-specific workflows or provider-name routing

#### Scenario: Documentation covers app developer usage
- **WHEN** a developer opens `docs/developer-packs/location/timezone.md`
- **THEN** the guide SHALL explain manifest declarations, required versus optional behavior, scopes, command DTOs, result DTOs, IANA/Windows mappings, instant/local semantics, DST gaps/folds, database freshness, boundary ambiguity, unavailable diagnostics, trace/audit behavior, and replay workflow
- **AND** it SHALL include minimal app-facing examples that use synthetic timezone data and canonical SDK calls

#### Scenario: Documentation covers provider authors
- **WHEN** a provider author reads the guide
- **THEN** it SHALL document descriptor fields, adapter responsibilities, tzdb/boundary/display-name versioning, conformance tests, unsupported behavior, redaction rules, health/snapshot behavior, and replacement strategy
- **AND** it SHALL forbid application-specific business routing in provider-neutral layers

### Requirement: Location Timezone observability SHALL be sanitized, replayable, and auditable

The pack SHALL emit sanitized trace, audit, health, snapshot, and replay evidence for declaration, admission, policy, entitlement, resource reservation, command request, provider selection, command result, unavailable state, stale database state, and snapshot recording.

#### Scenario: Successful command emits bounded evidence
- **WHEN** a timezone command succeeds
- **THEN** Macaca SHALL emit sanitized events containing pack id, command name, service id, descriptor version, trace id, application/session/task/tenant ids when available, provider class, database versions, policy decision, latency, coordinate precision class, zone id hash or canonical id when permitted, time range class, result class, and resource counters
- **AND** it SHALL exclude raw provider payloads, secrets, credentials, raw polygon geometry, exact coordinates when policy forbids them, and unbounded diagnostics

#### Scenario: Stale database emits explicit evidence
- **WHEN** a provider's tzdb, boundary, or display-name dataset is stale
- **THEN** Macaca SHALL emit `timezone.database_stale` evidence with dataset class, version, freshness status, policy decision, and update recommendation
- **AND** it SHALL not expose host-private file paths or raw database contents

#### Scenario: Snapshot records provider versions
- **WHEN** the service runtime records a timezone snapshot
- **THEN** the snapshot SHALL include provider health, command matrix, tzdb version, boundary dataset version, display-name dataset version, freshness status, policy template hash, unavailable diagnostics, and sanitized replay pointers
- **AND** it SHALL exclude raw secrets, credentials, raw provider payloads, raw polygon geometry, exact disallowed coordinates, and unbounded output

#### Scenario: Replay verifies database-version evidence
- **WHEN** a session or task is replayed after refresh or restart
- **THEN** Macaca SHALL reconstruct the timezone command chain from bounded trace/audit evidence
- **AND** replay diagnostics SHALL prove the command used the canonical service runtime path and identify the database versions used

### Requirement: Location Timezone implementation SHALL preserve Macaca architecture boundaries

The `pack.location.timezone.v1` implementation SHALL keep concrete providers behind service/runtime provider adapters. The microkernel, SDK, shells, and generic application framework SHALL remain provider-neutral and free of application-specific or provider-specific routing branches.

#### Scenario: Boundary gates scan imports
- **WHEN** dependency-boundary gates scan the implementation
- **THEN** they SHALL find no concrete timezone provider, remote API client, embedded tzdb loader, CLDR loader, or boundary dataset importer in the microkernel, SDK, shells, or generic application framework
- **AND** provider construction SHALL appear only in approved runtime composition roots or plugin/remote provider registration paths

#### Scenario: No-direct-provider-call gate scans commands
- **WHEN** no-direct-provider-call gates scan timezone commands
- **THEN** every callable operation SHALL be reachable only through descriptor-owned service registrations and typed service runtime dispatch
- **AND** SDK helpers SHALL only build canonical service commands

#### Scenario: Pack remains separate from neighboring capabilities
- **WHEN** architecture review compares related packs and services
- **THEN** timezone SHALL own zone lookup, identifier mapping, offsets, transitions, conversions, display names, database inspection, and boundary provenance
- **AND** foundation time, location geocode, location maps, location place-search, location route, workflow schedule, and communication calendar SHALL remain owned by their respective packs or services
