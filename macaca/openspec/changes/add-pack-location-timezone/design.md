# Location Timezone Pack Design

## Context

`pack.location.timezone.v1` gives Macaca applications a stable time-zone capability without forcing developers to depend on a specific provider, host API, or embedded tzdb implementation. The difficult parts are not simple offset math; they are civil-rule changes, IANA aliasing, daylight-saving gaps and folds, historical transitions, coordinate boundary ambiguity, localized display names, and reproducible behavior across provider/database versions.

This pack lives behind a service boundary. The microkernel owns trace, identity, service-call evidence, and policy primitives only. The timezone service owns lookup, normalization, conversion, transition, display-name, and database-inspection behavior through replaceable providers.

## Supplier Capability Matrix

| Supplier/API | Borrowed capability | Macaca mapping |
| --- | --- | --- |
| IANA tzdb | Canonical zone IDs, links/aliases, historical rules, releases, transition rules | `TimezoneZone`, `TimezoneDatabaseInfo`, versioned conversion and transition semantics |
| Google Time Zone API | Coordinate+timestamp lookup, zone id/name, raw offset, DST offset, status model | `lookup_by_coordinates`, `get_offset`, structured unavailable/invalid/result errors |
| GeoNames Timezone API | Coordinate lookup, GMT/DST offsets, country metadata, remote service error behavior | provider adapter model, `TimezoneLookupResult`, provenance and remote health |
| Mapbox Tilequery/custom tiles | Point-in-polygon lookup through tile-backed geospatial data | self-hosted/remote boundary provider class and bounded boundary provenance |
| timezone-boundary-builder | OSM-derived polygonal timezone boundaries and release provenance | offline boundary lookup, `TimezoneBoundaryProvenance`, ambiguity/accuracy metadata |
| Unicode CLDR/ICU | Localized names, exemplar cities, metazones, Windows/IANA mapping | `get_display_names`, `resolve_zone`, `TimezoneIdentifierMapping` |
| Java `java.time` / JS Temporal | Instant/local separation, gap/fold resolver strategies, zone IDs | `convert_instant`, `resolve_local_time`, explicit `LocalTimeResolutionStrategy` |
| Windows time zones | Host-specific identifiers and mapping concerns | provider-neutral mapping metadata without OS-layer provider branching |

## Goals

- Provide coordinate lookup, zone-id normalization, offset calculation, transition listing, instant conversion, local-time gap/fold resolution, localized display names, identifier mapping, and database inspection.
- Model tzdb/database version, source, freshness, release date, boundary provenance, and provider health so results are auditable and reproducible.
- Enforce policy, entitlement, resource, and redaction before provider calls.
- Support built-in, plugin, remote, host-native, offline-data, mock, and unavailable providers through descriptors.
- Give developers detailed documentation and conformance expectations for time-zone correctness.

## Non-Goals

- Do not own device geolocation capture, map rendering, place search, geocoding, routing, calendar event scheduling, workflow scheduling, holiday/business calendar rules, or application-specific local-time policy.
- Do not make OS-layer code branch on Google, GeoNames, CLDR, ICU, Windows, IANA, tzdb file paths, country names, or application workflows.
- Do not expose raw provider payloads, raw polygon data, credentials, secrets, package bytes, prompts, exact coordinates when redaction policy forbids them, or unbounded diagnostics in observability surfaces.

## Ownership And Boundaries

- Pack id: `pack.location.timezone.v1`.
- Capability family: `location`.
- Backing service: timezone service.
- SDK surface: `sdk.packs.location.timezone`.
- Command namespace: `timezone.*`.
- Application framework owns manifest declarations and app-scoped permission projection.
- Service runtime owns typed dispatch, decorators, provider lifecycle, health, snapshots, and unavailable behavior.
- Runtime host owns concrete provider composition through approved composition roots.
- Shells render diagnostics and call SDK/facade clients only.

## Command Surface

| Command | Purpose | Required behavior |
| --- | --- | --- |
| `timezone.lookup_by_coordinates` | Resolve coordinate/time to one or more candidate IANA zones | Requires coordinate precision policy, timestamp, boundary provenance, confidence, ambiguity metadata, and redaction |
| `timezone.resolve_zone` | Validate and normalize IANA, alias/link, Windows, CLDR, or provider-specific identifiers | Returns canonical IANA id when known, mapping metadata, deprecation/link status, and unsupported diagnostics |
| `timezone.get_offset` | Calculate UTC offset, standard/raw offset, DST offset, abbreviation, and rule provenance for instant/zone | Requires zone id, instant, tzdb/provider version, and structured stale/unknown handling |
| `timezone.list_transitions` | List offset/DST transitions for a zone and time range | Enforces bounded range/page size and returns transition instants, offsets, names, and rule provenance |
| `timezone.convert_instant` | Convert an absolute instant into local date/time in a zone | Returns local datetime, offset, abbreviation, calendar date, and database version |
| `timezone.resolve_local_time` | Resolve local date/time into instant candidates | Handles nonexistent gaps and ambiguous folds using explicit resolver strategy |
| `timezone.get_display_names` | Return localized display names, exemplar cities, generic/standard/daylight names, and metazone hints | Uses locale fallback policy and CLDR/provider provenance |
| `timezone.inspect_database` | Inspect provider/tzdb/boundary/display-name database versions and freshness | Returns versions, release dates, freshness, health, source class, and update recommendations |
| `timezone.inspect_mapping` | Inspect identifier mapping support between IANA, Windows, CLDR, aliases, and provider-specific ids | Returns mapping confidence, canonical ids, aliases, unsupported ids, and version metadata |

## DTO Model

- `TimezoneCommandContext`: application id, tenant id, session id, task id, trace id, locale, purpose, region policy, retention class, and optional approval id.
- `TimezoneCoordinateQuery`: latitude/longitude, timestamp/instant, precision class, spatial redaction policy, boundary hint, and max candidates.
- `TimezoneZone`: canonical IANA id, aliases, country/territory hints when available, coordinates/representative city when allowed, deprecation/link status, and provider-neutral provenance.
- `TimezoneLookupResult`: primary zone, candidate zones, confidence, boundary distance class, ambiguity reason, data version, and boundary provenance.
- `TimezoneOffset`: total offset seconds, raw/standard offset seconds, daylight offset seconds, abbreviation, is_dst, rule id/hash, effective interval, and database version.
- `TimezoneTransition`: transition instant, local before/after, offset before/after, abbreviation before/after, gap/fold classification, and rule provenance.
- `TimezoneLocalResolution`: zero, one, or multiple candidate instants plus resolver strategy, gap/fold diagnostics, selected instant, and policy warnings.
- `TimezoneDisplayNames`: locale, canonical zone id, generic/standard/daylight names, exemplar city, GMT format, metazone id, and fallback chain.
- `TimezoneDatabaseInfo`: tzdb version, boundary dataset version, display-name dataset version, release date, source class, freshness, health, and update recommendation.
- `TimezoneError`: denied, unavailable, unsupported, invalid coordinate, invalid zone id, ambiguous boundary, stale database, range too large, nonexistent local time, ambiguous local time, quota exceeded, provider failure, or conflict.

## Permission, Policy, Resource, Entitlement, And Approval

Initial scopes:

- `location.timezone.lookup.read`: coordinate-to-zone lookup.
- `location.timezone.offset.read`: offsets, conversions, and transitions.
- `location.timezone.names.read`: localized display names and identifier mapping.
- `location.timezone.database.inspect`: database version/freshness inspection.

Policy requirements:

- Exact coordinates are sensitive. The policy decorator can deny lookup, coarsen coordinates, log only spatial classes, or require approval.
- Lookup by coordinate must include a timestamp/instant because some providers calculate offsets at the requested time.
- Transition listing requires bounded date ranges and page sizes.
- Local-time resolution must declare a resolver strategy: reject, earlier, later, compatible, or explicit offset.
- Database freshness policy can warn, deny, or mark results stale when tzdb/boundary/display-name data is older than tenant policy allows.
- Offline providers must declare dataset versions; remote providers must declare health, quota class, and unavailable behavior.

## Service Runtime And Provider Strategy

Provider Strategy categories:

- Embedded tzdb provider: fast deterministic offset/transition/zone normalization, no network calls, requires update lifecycle.
- Boundary lookup provider: coordinate-to-zone using polygon data or tilequery-backed point-in-polygon lookup.
- Remote API provider: Google/GeoNames-like coordinate lookup with structured quota/unavailable behavior.
- Host-native provider: platform-provided timezone/display-name behavior with explicit host capability declarations.
- Display-name provider: CLDR/ICU-backed localized names and mappings.
- Unavailable provider: deterministic explicit unavailable behavior for tests and absent services.
- Mock provider: synthetic deterministic data for docs and conformance tests.

Providers may be combined behind one service descriptor, but composition must remain descriptor-driven. SDK, shells, kernel, and generic application framework never construct or branch on concrete provider names.

## SDK Discovery And Developer Documentation

SDK discovery returns pack id, lifecycle, command schemas, DTO schemas, permission scopes, effective availability, provider classes, command capability matrix, supported identifier systems, dataset versions, freshness, policy templates, examples, diagnostics, compatibility, and documentation links.

The implementation SHALL create `docs/developer-packs/location/timezone.md` with:

- Required and optional manifest declarations.
- Permission scopes and coordinate privacy implications.
- Command-by-command DTO reference.
- IANA/Windows/alias mapping guidance.
- Instant versus local date-time explanation.
- DST gap/fold resolver strategies with examples.
- Database version and freshness guidance.
- Boundary ambiguity and coordinate precision guidance.
- Error taxonomy and unavailable-provider troubleshooting.
- Trace/audit event reference and replay workflow.
- Provider author conformance checklist.

## Trace, Audit, Health, Snapshot, And Replay

Required sanitized events:

- `timezone.pack_declared`
- `timezone.admission_validated`
- `timezone.policy_decision`
- `timezone.entitlement_checked`
- `timezone.resource_reserved`
- `timezone.command_requested`
- `timezone.provider_selected`
- `timezone.command_succeeded`
- `timezone.command_failed`
- `timezone.unavailable`
- `timezone.database_stale`
- `timezone.snapshot_recorded`

Events include pack id, command name, service id, descriptor version, trace id, application/session/task/tenant ids when present, provider class, database versions, policy decision, bounded error code, coordinate precision class, zone id hash or canonical id when permitted, time range class, latency, and resource counters. Events exclude raw provider payloads, raw polygon geometry, credentials, exact coordinates when redaction forbids them, and unbounded diagnostics.

Snapshots include provider health, command matrix, tzdb version, boundary dataset version, display-name dataset version, freshness status, policy template hash, unavailable diagnostics, and sanitized replay pointers.

## Design Patterns

- **Facade**: SDK exposes discovery and command builders while `SystemFacade` carries canonical service calls.
- **Command**: each operation is a typed command/result DTO.
- **Adapter**: provider adapters translate tzdb, CLDR/ICU, remote APIs, host APIs, or boundary datasets into Macaca DTOs.
- **Strategy**: provider composition, boundary lookup, display-name lookup, database freshness policy, and unavailable behavior are descriptor-driven.
- **Decorator**: trace, policy, resource, entitlement, approval, metering, and redaction wrap every call.
- **Specification**: admission validates scopes, command schemas, zone ids, coordinate precision, range bounds, resolver strategy, and dataset freshness.
- **Observer**: trace, audit, health, stale-database, and service events are subscribable and replayable.
- **Memento**: snapshots record versions and replay pointers for reproducibility.
- **Abstract Factory**: providers are constructed only through approved runtime composition roots.

## Risks And Mitigations

- Risk: treating timezone as simple current offset. Mitigation: require instant-aware offset/conversion and transition APIs.
- Risk: DST gaps/folds cause silent wrong scheduling. Mitigation: `resolve_local_time` requires explicit resolver strategy and returns candidates/diagnostics.
- Risk: stale tzdb causes incorrect results. Mitigation: dataset version/freshness is part of results, snapshots, SDK discovery, and policy.
- Risk: coordinate lookup near borders is ambiguous. Mitigation: return confidence, boundary distance class, candidates, and provenance.
- Risk: localized names differ across datasets. Mitigation: expose display-name dataset version, locale fallback chain, and provider provenance.
- Risk: SDK helpers become alternate execution path. Mitigation: helpers only build canonical service commands; gates enforce no direct provider calls.
