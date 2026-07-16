# Location Geocode Pack

`pack.location.geocode.v1` provides provider-neutral forward geocoding, reverse
geocoding, address normalization, confidence inspection, batch planning, batch
request/status/cancel, retention inspection, attribution inspection, and
artifact handle discovery.

The pack does not own place search, route calculation, map rendering, device
location capture, identity verification, or application address workflows. It is
unavailable until a serviceized geocode provider is installed.

## Manifest Declaration

```toml
[service_contract]
optional_packs = ["pack.location.geocode.v1"]
```

Unavailable optional declarations report
`location_geocode_provider_not_installed`. Required declarations block readiness
until descriptor-compatible provider schemas, permissions, policy, resources,
and entitlements are available.

## Commands

- `geocode.inspect_provider`: inspect provider-class capability metadata.
- `geocode.discover_schema`: discover supported address and precision schema.
- `geocode.validate_query`: validate `GeocodeQuery` or
  `ReverseGeocodeQuery` without side effects.
- `geocode.forward` and `geocode.reverse`: return bounded
  `GeocodeCandidate` projections.
- `geocode.normalize_address`: returns normalized `AddressComponentSet`.
- `geocode.inspect_confidence`: explains `GeocodeConfidence` classes.
- `geocode.plan_batch`, `geocode.request_batch`, `geocode.inspect_batch`, and
  `geocode.cancel_batch`: manage bounded `GeocodeBatchJob` state.
- `geocode.inspect_retention` and `geocode.inspect_attribution`: expose
  storage and attribution requirements.
- `geocode.get_artifact`: returns `GeocodeArtifactHandle`.

## DTOs And Results

Core DTOs include `GeocodeScope`, `GeocodeQuery`, `ReverseGeocodeQuery`,
`AddressComponentSet`, `GeocodeGeometry`, `LocationPrecisionClass`,
`GeocodeConfidence`, `GeocodeCandidate`, `GeocodeRetentionPolicy`,
`GeocodeBatchJob`, and `GeocodeArtifactHandle`. Result statuses cover success,
partial, approval-required, denied, unavailable, unsupported, conflict,
ambiguous, no-match, stale-version, quota, rate-limited, timeout, cancelled,
retention-denied, attribution-missing, and failure.

## Field Notes

`GeocodeScope` carries tenant, region, retention, precision, and attribution
classes. Query DTOs carry normalized input refs, country/language filters,
field masks, precision class, idempotency key, and approval ref; reverse
queries carry coordinate refs rather than raw private coordinates in logs.
Candidates carry component refs, geometry refs, confidence class, precision,
country/language metadata, freshness, attribution, and replay pointer. Batch
jobs carry request hash, bounded item count, cursor, state, cancellation token,
artifact retention, and result expiry. Retention DTOs describe storage mode,
duration, exportability, and deletion evidence. Structured errors include
ambiguous, no-match, country-unsupported, high-precision-denied,
retention-denied, attribution-missing, quota, rate-limited, timeout,
cancelled, network-denied, and artifact-denied diagnostics.

## Provider Mapping

Google Maps Geocoding, Mapbox Geocoding, HERE Geocoding and Search, TomTom
Geocoding, Esri World Geocoding, Azure Maps Search, Apple CLGeocoder,
Nominatim, and Pelias-style providers map into normalized queries, candidates,
geometry references, confidence, precision classes, retention policy, and
attribution bundles. Raw provider address payloads, private address lists,
credentials, and exact private coordinates are excluded from traces and SDK
diagnostics.

## App-Facing Examples

Applications declare `pack.location.geocode.v1` as required when geocoding is a
readiness dependency, or optional when the UI can degrade with
`location_geocode_provider_not_installed`. All calls pass through typed SDK
commands with synthetic scope, query, candidate, batch, retention, attribution,
and artifact refs.

- Discover schema and validate forward or reverse queries before calling
  `geocode.forward` or `geocode.reverse`.
- Normalize addresses, inspect confidence, and store only bounded candidate
  refs rather than raw private address payloads.
- Plan and request batch geocoding, then inspect or cancel by `GeocodeBatchJob`
  ref without logging unbounded address lists.
- Inspect retention, attribution, and artifact handles before persisting or
  displaying results.
- Handle unavailable provider, denied permission, missing entitlement, no
  match, ambiguous result, retention denied, attribution missing, unsupported
  country, high-precision denied, batch quota exceeded, network denied, and
  artifact denied with provider-neutral diagnostics.

## Conformance

Provider authors must cover descriptor completeness, query scope validation,
idempotency, precision mapping, confidence mapping, retention enforcement,
attribution completeness, batch state-machine behavior, resource bounds, policy
hooks, sanitized trace/audit events, unavailable behavior, snapshot/replay, and
no raw payload leakage.
