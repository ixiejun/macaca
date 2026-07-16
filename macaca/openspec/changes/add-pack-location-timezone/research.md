# Location Timezone Pack Research

## Purpose

This note records supplier/API research, Macaca provider-neutral mapping,
boundary decisions, and GitNexus memo evidence for
`pack.location.timezone.v1`. The timezone pack owns coordinate-to-zone lookup,
zone resolution, offset lookup, transition listing, instant conversion,
local-time gap/fold resolution, display names, database inspection, identifier
mapping, freshness, and redaction. It must not own scheduling, calendar event
semantics, geocoding, map rendering, route calculation, place search, device
location capture, or application scheduling workflows.

## Source Baseline

- IANA Time Zone Database:
  <https://www.iana.org/time-zones>
- Google Time Zone API:
  <https://developers.google.com/maps/documentation/timezone>
- GeoNames Timezone API:
  <https://www.geonames.org/export/web-services.html#timezone>
- Mapbox Tilequery API as a boundary lookup reference:
  <https://docs.mapbox.com/api/maps/tilequery/>
- timezone-boundary-builder:
  <https://github.com/evansiroky/timezone-boundary-builder>
- Unicode CLDR time zone names and mappings:
  <https://cldr.unicode.org/translation/time-zones-and-city-names>
- ICU time zone APIs:
  <https://unicode-org.github.io/icu/userguide/datetime/timezone/>
- Java `java.time`:
  <https://docs.oracle.com/en/java/javase/21/docs/api/java.base/java/time/package-summary.html>
- JavaScript Temporal:
  <https://tc39.es/proposal-temporal/docs/>
- Microsoft Windows time zone identifiers:
  <https://learn.microsoft.com/en-us/windows-hardware/manufacture/desktop/default-time-zones>

## Supplier API Notes

- IANA tzdb contributes canonical zone identifiers, historical transitions,
  offset rules, and versioned database semantics. Macaca should preserve tzdb
  version evidence and expose stale-database diagnostics.
- Google Time Zone and GeoNames contribute remote coordinate lookup and DST/UTC
  offset surfaces. Macaca should model them as remote providers with quota,
  precision, and network approval gates.
- timezone-boundary-builder and Mapbox Tilequery-style boundary lookup show
  spatial-boundary approaches. Macaca should expose boundary provenance and
  ambiguity, not raw boundary geometry.
- Unicode CLDR/ICU and Windows mappings contribute display names and identifier
  translation. Macaca should treat display-name datasets and Windows/IANA
  mapping as versioned capability data.
- Java `java.time` and JavaScript Temporal contribute explicit instant, zoned
  date-time, and local-time gap/fold handling. Macaca should require explicit
  local-time resolution strategy for nonexistent or ambiguous local times.

## Macaca-Owned Abstractions

`pack.location.timezone.v1` should define `TimezoneCommandContext`,
`TimezoneCoordinateQuery`, `TimezoneZone`, `TimezoneLookupResult`,
`TimezoneOffset`, `TimezoneTransition`, `TimezoneLocalResolution`,
`TimezoneDisplayNames`, `TimezoneDatabaseInfo`,
`TimezoneBoundaryProvenance`, `TimezoneIdentifierMapping`, and
`TimezoneError`.

The DTOs must carry coordinate precision, zone identifier system, tzdb version,
boundary provenance, offset timestamp, transition range, local-time resolver
strategy, display-name locale, Windows/IANA mapping version, database freshness,
provider attribution, redaction class, and replay pointers. Exact private
coordinates, raw provider payloads, raw boundary geometry, credentials, host
database paths, unbounded transition dumps, and scheduling workflow state are
rejected.

## Boundary Decisions And Non-Goals

- Foundation time owns generic instant, clock, duration, calendar, and timer
  primitives.
- Geocode owns address/coordinate conversion.
- Maps owns rendering.
- Place-search owns POI search.
- Route owns path calculation.
- Workflow schedule and communication calendar own scheduling and calendar
  workflows.
- Device owns location capture.

## Existing Macaca Platform Inventory

- `crates/foundation/macaca-proto/src/domain_pack_contract/` provides reusable
  descriptor metadata, lifecycle/availability, policy templates, diagnostics,
  SDK metadata, provider snapshots, unavailable diagnostics, and effective
  capability expansion.
- `crates/facade/macaca-sdk/src/system_facade.rs` provides the Facade pattern;
  timezone SDK helpers should only build canonical traced service calls.
- Generic policy, approval, resource, entitlement, trace, audit, artifact,
  mock-provider, and unavailable-provider concepts are reusable, but current
  evidence does not prove timezone-specific DTOs, descriptors, providers, SDK
  helpers, ABI metadata, tests, or developer docs.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
