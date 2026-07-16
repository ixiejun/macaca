# Location Timezone Pack

`pack.location.timezone.v1` provides provider-neutral coordinate-to-timezone
lookup, zone resolution, offset calculation, transition listing, instant
conversion, local-time gap/fold resolution, display-name lookup, database
inspection, and identifier mapping inspection.

The pack does not own foundation time primitives, workflow scheduling, calendar
semantics, map rendering, geocoding, route planning, or device location capture.
It becomes callable only when a serviceized timezone provider is registered.

## Manifest Declaration

```toml
[service_contract]
optional_packs = ["pack.location.timezone.v1"]
```

Optional declarations degrade with `location_timezone_provider_not_installed`.
Required declarations block readiness when database freshness, identifier
systems, boundary lookup, permissions, policy, or provider availability are not
satisfied.

## Commands

- `timezone.lookup_by_coordinates`: resolves a coordinate reference to a
  `TimezoneZone` with boundary provenance.
- `timezone.resolve_zone`: normalizes IANA, Windows, alias, or provider-neutral
  zone references.
- `timezone.get_offset`: returns `TimezoneOffset` for an instant.
- `timezone.list_transitions`: returns bounded `TimezoneTransition` pages.
- `timezone.convert_instant`: converts instant references across zones.
- `timezone.resolve_local_time`: handles DST gaps/folds using explicit
  strategies: reject, earlier, later, compatible, or explicit offset.
- `timezone.get_display_names`: returns locale-aware `TimezoneDisplayNames`.
- `timezone.inspect_database`: reports `TimezoneDatabaseInfo`.
- `timezone.inspect_mapping`: reports `TimezoneIdentifierMapping`.

## DTOs And Results

Core DTOs include `TimezoneCommandContext`, `TimezoneCoordinateQuery`,
`TimezoneZone`, `TimezoneLookupResult`, `TimezoneOffset`,
`TimezoneTransition`, `TimezoneLocalResolution`, `TimezoneDisplayNames`,
`TimezoneDatabaseInfo`, `TimezoneBoundaryProvenance`,
`TimezoneIdentifierMapping`, and `TimezoneError`. Result statuses cover
success, partial, denied, unavailable, unsupported, invalid-zone,
invalid-coordinate, ambiguous-boundary, stale-database, nonexistent-local-time,
ambiguous-local-time, quota-exceeded, provider-failure, and conflict.

## Provider Mapping

IANA tzdb, Google Time Zone API, GeoNames Timezone API, Mapbox Tilequery-style
boundary lookup, timezone-boundary-builder datasets, Unicode CLDR/ICU,
Java `java.time`, JavaScript Temporal, and Windows timezone identifiers map
into zone references, offset records, transition pages, boundary provenance,
database freshness, display names, and identifier mappings. Raw boundary
geometry, database paths, host identifiers, exact private coordinates,
credentials, and provider payloads are excluded from observability surfaces.

## App-Facing Examples

Applications declare `pack.location.timezone.v1` and call only typed SDK
commands with synthetic coordinate, zone, instant, local-time, display-name,
database, and mapping refs. Examples avoid raw boundary geometry, database
paths, exact private coordinates, host identifiers, credentials, and provider
payloads.

- Look up a timezone by coordinate ref, normalize the returned zone, and inspect
  boundary provenance.
- Calculate an offset, list bounded transitions, and convert instants across
  zones using explicit instant refs.
- Resolve local-time gaps or folds with an explicit strategy such as reject,
  earlier, later, compatible, or explicit offset.
- Retrieve display names and inspect database or identifier-mapping freshness.
- Handle unavailable provider, invalid zone, invalid coordinate,
  ambiguous-boundary, stale-database, nonexistent local time, ambiguous local
  time, quota exceeded, and provider failure with provider-neutral diagnostics.

## Conformance

Provider authors must cover descriptor fields, adapter responsibilities,
tzdb/boundary/display-name versioning, DST gap/fold conformance, unsupported
behavior, redaction rules, health/snapshot behavior, unavailable provider
behavior, snapshot/replay, and no raw payload leakage.
