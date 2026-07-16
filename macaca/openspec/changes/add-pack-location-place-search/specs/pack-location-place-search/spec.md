## ADDED Requirements

### Requirement: Macaca SHALL provide Location Place Search as a serviceized industrial pack

Macaca SHALL provide `pack.location.place.search.v1` as a provider-neutral industrial pack for POI discovery, place lookup, nearby/category search, autocomplete, suggestion resolution, place details, category taxonomy, field capability inspection, attribution inspection, and retained search-session purge. The pack SHALL be declared by applications, resolved by admission/catalog services, and invoked only through typed service commands.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.location.place.search.v1` as required and the location place search service is registered, healthy, entitled, policy-admissible, and command-compatible
- **THEN** admission SHALL expose the pack in the effective capability set with command schemas, permission scopes, field capability matrix, policy template, availability, health, attribution obligations, and replay metadata
- **AND** SDK discovery SHALL mark callable commands as available without exposing provider secrets, credentials, raw provider payloads, session tokens, raw media, or unbounded review text

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.location.place.search.v1` as required but provider, command support, permission, entitlement, resource, host support, region policy, or attribution compliance is absent
- **THEN** admission SHALL block readiness with structured unavailable or denied diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back to another provider, or fake success

#### Scenario: Optional declaration is unavailable
- **WHEN** an application declares `pack.location.place.search.v1` as optional and the pack is unavailable
- **THEN** admission SHALL produce an explicit degraded effective capability report with bounded reason codes
- **AND** SDK command helpers SHALL refuse to build callable service calls for unavailable commands

### Requirement: Location Place Search SHALL expose supplier-grade provider-neutral commands

`pack.location.place.search.v1` SHALL expose typed commands for `place_search.search`, `place_search.nearby`, `place_search.suggest`, `place_search.resolve_suggestion`, `place_search.get_details`, `place_search.list_categories`, `place_search.inspect_fields`, `place_search.inspect_attribution`, and `place_search.purge_session`.

#### Scenario: Text search returns normalized place summaries
- **WHEN** a declared and policy-allowed caller invokes `place_search.search` with query text, locale, region policy, category filters, field mask, proximity or viewport bias, page size, and trace context
- **THEN** Macaca SHALL route the command through SDK/facade helpers into the service runtime and location place search provider
- **AND** the result SHALL contain ranked `PlaceSummary` DTOs with stable Macaca place references, labels, coordinates or viewports when permitted, category summaries, distance when available, quality metadata, provenance, and attribution ids

#### Scenario: Nearby search enforces spatial constraints
- **WHEN** a caller invokes `place_search.nearby`
- **THEN** Macaca SHALL require a bounded spatial constraint such as coordinate radius, viewport, polygon reference, route-corridor reference, or place anchor
- **AND** it SHALL reject unbounded or policy-disallowed searches before provider dispatch with a typed denied or invalid-constraint result

#### Scenario: Autocomplete uses ephemeral session behavior
- **WHEN** a caller invokes `place_search.suggest` for interactive typeahead
- **THEN** Macaca SHALL represent provider-specific session-token behavior through a provider-neutral `SearchSession` reference
- **AND** the session SHALL have explicit expiry, retention class, purge behavior, and sanitized trace evidence

#### Scenario: Suggestion resolution preserves canonical path
- **WHEN** a caller invokes `place_search.resolve_suggestion` with a suggestion id and search session reference
- **THEN** Macaca SHALL resolve the suggestion through the service runtime into a `PlaceReference` or bounded `PlaceDetails` subset
- **AND** it SHALL emit trace and audit evidence linking the suggestion, session, policy decision, and result without leaking provider session tokens

#### Scenario: Details require field masks
- **WHEN** a caller invokes `place_search.get_details`
- **THEN** Macaca SHALL require a field mask and SHALL check entitlement, provider cost/rate class, policy, and attribution obligations before provider dispatch
- **AND** unsupported or entitlement-gated fields SHALL produce typed unsupported, denied, or entitlement-required diagnostics

#### Scenario: Category taxonomy is discoverable
- **WHEN** a caller invokes `place_search.list_categories`
- **THEN** Macaca SHALL return normalized `PlaceCategory` DTOs with stable ids, labels, locale labels when available, hierarchy, deprecation metadata, and provider support hints
- **AND** it SHALL NOT expose provider-specific category ids as routing rules in OS-layer code

#### Scenario: Field capability inspection explains provider limits
- **WHEN** a caller invokes `place_search.inspect_fields`
- **THEN** Macaca SHALL return the effective provider's normalized field matrix, unsupported fields, entitlement-gated fields, cost/rate classes, and version compatibility metadata
- **AND** SDK helpers SHALL use that metadata to prevent obviously unsupported calls

#### Scenario: Attribution inspection returns display obligations
- **WHEN** a caller invokes `place_search.inspect_attribution`
- **THEN** Macaca SHALL return `PlaceAttribution` rules for covered result ids, retention limits, refresh requirements, display references, and bounded warning codes
- **AND** it SHALL NOT expose raw supplier contracts, secrets, or credential-bearing URLs

#### Scenario: Search session purge removes retained artifacts
- **WHEN** a caller invokes `place_search.purge_session`
- **THEN** Macaca SHALL delete retained autocomplete/search session artifacts within the pack's ownership boundary
- **AND** it SHALL emit sanitized purge audit evidence with session reference, retention class, and trace id

### Requirement: Location Place Search DTOs SHALL normalize provider data without leaking raw payloads

The pack SHALL define provider-neutral DTOs for command context, spatial constraints, queries, summaries, details, suggestions, categories, attributions, media references, external references, quality metadata, and structured errors. Provider adapters SHALL translate supplier payloads into these DTOs and SHALL drop or redact fields that are not permitted by policy, entitlement, attribution, or schema.

#### Scenario: Place summary carries bounded discovery data
- **WHEN** a search or nearby command succeeds
- **THEN** each `PlaceSummary` SHALL include only normalized bounded fields such as place reference, display name, category summaries, coordinate or viewport when permitted, address label, distance, business status, provenance, quality metadata, and attribution id
- **AND** raw provider payloads SHALL NOT be returned unless explicitly modeled as sanitized provider-neutral fields

#### Scenario: Place details carry field-level provenance
- **WHEN** a details command succeeds
- **THEN** `PlaceDetails` SHALL include requested and permitted fields with field-level provenance, freshness, confidence, entitlement status, and attribution coverage
- **AND** absent, unsupported, stale, or redacted fields SHALL be represented explicitly rather than omitted ambiguously

#### Scenario: Media remains a reference
- **WHEN** a provider exposes photos or other media for a place
- **THEN** Macaca SHALL return bounded `PlaceMediaReference` DTOs with type, dimensions when available, attribution id, expiry, and retrieval constraints
- **AND** the generic place search pack SHALL NOT store or trace raw media bytes

#### Scenario: Review-derived data is bounded
- **WHEN** a provider exposes ratings, price, tips, popularity, or review-derived summaries
- **THEN** Macaca SHALL expose only normalized bounded summary fields allowed by entitlement and policy
- **AND** raw full review bodies SHALL NOT enter generic traces, audits, snapshots, or examples

#### Scenario: Structured errors are stable across providers
- **WHEN** a provider returns absent, unsupported, quota, stale, ambiguous, entitlement, attribution, or provider failure conditions
- **THEN** Macaca SHALL map them to stable `PlaceSearchError` variants
- **AND** provider-specific diagnostics SHALL be sanitized and bounded

### Requirement: Location Place Search SHALL enforce permission, policy, resource, entitlement, and approval gates

Every command in `pack.location.place.search.v1` SHALL run through policy, permission, resource, entitlement, metering, and approval decorators before provider side effects or external calls.

#### Scenario: Missing permission denies before provider dispatch
- **WHEN** an application invokes a command without the required scope such as `location.place.search.read`, `location.place.autocomplete.read`, `location.place.details.read`, `location.place.categories.read`, `location.place.media.reference.read`, or `location.place.session.manage`
- **THEN** Macaca SHALL return a typed denied result before invoking the concrete provider
- **AND** the audit event SHALL include the bounded missing-scope code

#### Scenario: Region policy restricts a query
- **WHEN** tenant or host policy restricts region, provider class, category, result count, retention, field set, or localization for a place search command
- **THEN** Macaca SHALL enforce that policy before provider dispatch
- **AND** the returned denied or constrained result SHALL include stable policy reason codes without leaking sensitive query details

#### Scenario: Resource quota blocks expensive search
- **WHEN** page size, search area, field mask, provider rate class, retained session count, or network budget exceeds the effective resource budget
- **THEN** Macaca SHALL return a typed quota-exceeded result before provider dispatch
- **AND** resource counters SHALL be emitted in sanitized trace evidence

#### Scenario: Entitlement blocks plan-gated fields
- **WHEN** a details or media-reference command requests fields gated by entitlement or supplier plan
- **THEN** Macaca SHALL return entitlement-required or partial results with field-level diagnostics
- **AND** it SHALL NOT call a provider path that would bill for disallowed fields

#### Scenario: Approval is required for sensitive context
- **WHEN** host policy marks a command sensitive because it uses precise location, background/native host capabilities, high-spend provider calls, external network disclosure, or retained user-intent sessions
- **THEN** Macaca SHALL require explicit approval evidence before dispatch
- **AND** denial or missing approval SHALL be traceable without exposing raw location intent

### Requirement: Location Place Search SHALL preserve canonical service runtime execution

All callable operations SHALL traverse the canonical Macaca service path: application declaration, admission/effective capability projection, SDK/facade command construction, service runtime dispatch, decorators, provider adapter, structured result, trace/audit evidence, and replayable snapshot. SDK helpers SHALL NOT construct providers or create alternate execution paths.

#### Scenario: Command succeeds through the canonical path
- **WHEN** a declared and policy-allowed command is invoked
- **THEN** Macaca SHALL route it through SDK/facade helpers into service runtime dispatch and the active location place search provider adapter
- **AND** trace evidence SHALL show declaration, admission, policy, entitlement, resource, provider selection, command result, and replay pointer events

#### Scenario: Provider is absent
- **WHEN** no provider is registered for `pack.location.place.search.v1`
- **THEN** the unavailable provider SHALL return structured unavailable diagnostics
- **AND** SDK discovery SHALL report unavailable state while preserving the same provider-neutral command/result contract

#### Scenario: Provider supports only a subset
- **WHEN** the active provider supports search and nearby but not suggestions or rich details
- **THEN** SDK discovery SHALL mark unsupported commands or fields as non-callable
- **AND** direct invocation SHALL return typed unsupported diagnostics without falling through to application-specific logic

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, mock, self-hosted, native-host, or unavailable provider is selected
- **THEN** callers SHALL observe the same provider-neutral DTO contract
- **AND** OS-layer code SHALL identify only provider class, descriptor version, and capability metadata in traces rather than branching on provider names

### Requirement: Location Place Search SHALL expose industrial SDK discovery and developer documentation

SDK discovery for `pack.location.place.search.v1` SHALL expose pack metadata, lifecycle, command schemas, DTO schemas, permission scopes, effective availability, provider class, field capability matrix, policy templates, cost/rate hints, attribution obligations, examples, diagnostics, version compatibility, and documentation links. The implementation SHALL provide detailed developer documentation under `docs/developer-packs/location/place-search.md`.

#### Scenario: Developer inspects the pack
- **WHEN** SDK discovery inspects `pack.location.place.search.v1`
- **THEN** it SHALL return command namespace `place_search.*`, supported commands, required scopes, field capability matrix, policy templates, examples, lifecycle, health, diagnostics, attribution rules, compatibility metadata, and documentation URL
- **AND** examples SHALL use generic synthetic data rather than application-specific workflows or provider-name routing

#### Scenario: Documentation covers app developer usage
- **WHEN** a developer opens `docs/developer-packs/location/place-search.md`
- **THEN** the guide SHALL explain manifest declarations, required versus optional behavior, scopes, command DTOs, result DTOs, field masks, pagination, autocomplete sessions, details, attribution, retention, unavailable diagnostics, trace/audit behavior, and replay workflow
- **AND** it SHALL include minimal app-facing examples that use synthetic place data and canonical SDK calls

#### Scenario: Documentation covers provider authors
- **WHEN** a provider author reads the guide
- **THEN** it SHALL document descriptor fields, adapter responsibilities, conformance tests, attribution translation, field capability reporting, unsupported-field behavior, redaction rules, health/snapshot behavior, and replacement strategy
- **AND** it SHALL forbid application-specific business routing in provider-neutral layers

### Requirement: Location Place Search observability SHALL be sanitized, replayable, and auditable

The pack SHALL emit sanitized trace, audit, health, snapshot, and replay evidence for declaration, admission, policy, entitlement, resource reservation, command request, provider selection, command result, unavailable state, attribution recording, session purge, and snapshot recording.

#### Scenario: Successful command emits bounded evidence
- **WHEN** a place search command succeeds
- **THEN** Macaca SHALL emit sanitized events containing pack id, command name, service id, descriptor version, trace id, application/session/task/tenant ids when available, provider class, policy decision, latency, result count, field mask hash, spatial constraint class, attribution id, retention class, and resource counters
- **AND** it SHALL exclude raw provider payloads, secrets, credentials, raw media, provider session tokens, and unbounded text

#### Scenario: Redaction coarsens sensitive query context
- **WHEN** policy marks query text, exact coordinates, or user-intent data sensitive
- **THEN** trace and audit events SHALL store only hashes, coarse spatial classes, bounded snippets, or redacted markers according to policy
- **AND** replay SHALL remain possible through stable evidence identifiers rather than raw sensitive payloads

#### Scenario: Snapshot records provider health
- **WHEN** the service runtime records a place search snapshot
- **THEN** the snapshot SHALL include descriptor version, provider health, supported command matrix, supported field matrix, policy template hash, rate/quota class, last health transition, unavailable diagnostics, and sanitized replay pointers
- **AND** it SHALL exclude raw secrets, credentials, raw provider payloads, exact disallowed coordinates, raw media, and unbounded output

#### Scenario: Replay verifies the service path after restart
- **WHEN** a session or task is replayed after refresh or restart
- **THEN** Macaca SHALL reconstruct the place search command chain from bounded trace/audit evidence
- **AND** replay diagnostics SHALL prove the command used the canonical service runtime path

### Requirement: Location Place Search implementation SHALL preserve Macaca architecture boundaries

The `pack.location.place.search.v1` implementation SHALL keep concrete providers behind service/runtime provider adapters. The microkernel, SDK, shells, and generic application framework SHALL remain provider-neutral and free of application-specific or provider-specific routing branches.

#### Scenario: Boundary gates scan imports
- **WHEN** dependency-boundary gates scan the implementation
- **THEN** they SHALL find no concrete place search provider imports in the microkernel, SDK, shells, or generic application framework
- **AND** provider construction SHALL appear only in approved runtime composition roots or plugin/remote provider registration paths

#### Scenario: No-direct-provider-call gate scans commands
- **WHEN** no-direct-provider-call gates scan place search commands
- **THEN** every callable operation SHALL be reachable only through descriptor-owned service registrations and typed service runtime dispatch
- **AND** SDK helpers SHALL only build canonical service commands

#### Scenario: Pack remains separate from neighboring location packs
- **WHEN** architecture review compares location packs
- **THEN** place search SHALL own POI discovery, suggestions, place details, categories, attribution, and search-session retention
- **AND** geocode, route, maps, timezone, and device-location capture SHALL remain owned by their respective packs or services
