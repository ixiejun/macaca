# Change: Add Industrial Location Timezone Pack

## Why

Macaca applications need a provider-neutral `pack.location.timezone.v1` capability for resolving time zones from coordinates, validating IANA zone identifiers, computing historical and future UTC offsets, handling daylight-saving transitions, converting local/instant times, localizing display names, and inspecting time-zone database freshness. The current template is not industrial-grade because it does not define DST gap/fold semantics, tzdb version governance, boundary-data provenance, or provider conformance requirements.

Time-zone behavior is deceptively high risk: civil authorities change rules, historical offsets differ by date, local times can be nonexistent or ambiguous, and coordinate-to-zone boundaries are data-version dependent. Macaca needs a pack that developers can rely on without embedding application-specific timezone logic or provider-specific SDK calls.

## Supplier/API Baseline

The design is based on mature time-zone systems and APIs:

- IANA Time Zone Database (`tzdb`): canonical zone identifiers, historical/future transition rules, aliases/links, releases, and rule changes. Official source: https://www.iana.org/time-zones
- Google Time Zone API: coordinate plus timestamp lookup, `timeZoneId`, `timeZoneName`, `rawOffset`, `dstOffset`, and status/error model. Official docs: https://developers.google.com/maps/documentation/timezone/overview
- GeoNames Timezone API: coordinate lookup, GMT offsets, DST offsets, country metadata, sunrise/sunset adjunct data, and service error behavior. Official docs: https://www.geonames.org/export/web-services.html#timezone
- Mapbox Tilequery/custom tiles patterns: point-in-polygon lookup against hosted geospatial tiles, useful for self-hosted timezone boundary providers. Official docs: https://docs.mapbox.com/api/maps/tilequery/
- timezone-boundary-builder/open data: polygonal time-zone boundary data derived from OpenStreetMap, used by many offline/self-hosted timezone lookup stacks. Official project: https://github.com/evansiroky/timezone-boundary-builder
- Unicode CLDR/ICU: localized timezone display names, exemplar cities, meta-zones, and Windows/IANA mapping data. Official docs: https://cldr.unicode.org/ and https://unicode-org.github.io/icu/userguide/datetime/timezone/
- Java `java.time` / TZDB provider model and JavaScript Temporal proposal: explicit instant vs local date-time conversion, gaps/folds, resolver strategies, and stable zone IDs. Official docs: https://docs.oracle.com/javase/8/docs/api/java/time/ZoneId.html and https://tc39.es/proposal-temporal/
- Windows time-zone identifiers and mapping concerns: host platforms may expose Windows IDs that need declarative mapping rather than hardcoded business logic. Official docs: https://learn.microsoft.com/windows/win32/intl/time-zones

## Macaca Provider-Neutral Mapping

Macaca SHALL expose stable DTOs and semantics:

- Coordinate-to-zone lookup becomes `timezone.lookup_by_coordinates`.
- Identifier validation/normalization becomes `timezone.resolve_zone`.
- Offset and abbreviation calculation becomes `timezone.get_offset`.
- DST and rule changes become `timezone.list_transitions`.
- Instant/local conversion becomes `timezone.convert_instant` and `timezone.resolve_local_time`.
- Localized names become `timezone.get_display_names`.
- tzdb/provider metadata becomes `timezone.inspect_database`.
- Windows/IANA/alias mapping becomes provider-neutral `TimezoneIdentifierMapping` metadata.

The pack SHALL not own device geolocation capture, map rendering, place search, routing, calendar event semantics, or application scheduling workflows.

## What Changes

- Add `pack.location.timezone.v1` as a service-backed industrial pack under the location family.
- Define command DTOs for coordinate lookup, zone resolution, offset calculation, transition listing, instant conversion, local-time resolution, display names, database inspection, and identifier mapping.
- Define normalized DTOs for `TimezoneZone`, `TimezoneOffset`, `TimezoneTransition`, `TimezoneLocalResolution`, `TimezoneDisplayNames`, `TimezoneDatabaseInfo`, `TimezoneBoundaryProvenance`, and structured errors.
- Define permissions, policy gates, resource budgets, region/data provenance requirements, boundary accuracy reporting, cache/ttl behavior, and unavailable-provider behavior.
- Require detailed developer documentation under `docs/developer-packs/location/timezone.md`.

## Impact

- Affected specs: `pack-location-timezone`, `developer-pack-industrial-capability-catalog`, `sdk-system-facade`, `service-runtime`, `unified-execution-path`.
- Later affected code: protocol DTOs, descriptor/admission validators, SDK pack client, timezone service provider contract, mock/unavailable providers, optional provider adapters, tzdb/boundary data management, trace/audit tests, and boundary gates.
- Validation: `openspec validate add-pack-location-timezone --strict`, DTO compatibility tests, DST gap/fold conformance tests, tzdb version tests, canonical service-path tests, no-direct-provider-call gates, data redaction tests, and docs coverage checks.

## Non-Goals

- This pack does not capture a user's current location, render maps, calculate routes, search places, schedule tasks/calendar events, or define application-specific business-time logic.
- This pack does not hardcode provider names, country-specific business rules, holiday calendars, market hours, prayer times, or application-specific cutoffs.
- This pack does not expose raw provider payloads, credentials, secrets, raw polygon datasets, or unbounded boundary geometry in traces, audits, logs, SDK examples, or snapshots.
