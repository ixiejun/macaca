# Location Geocode Pack Research

## Purpose

This note records supplier/API research, Macaca provider-neutral mapping,
boundary decisions, existing platform inventory, and GitNexus memo evidence for
`pack.location.geocode.v1`. The geocode pack owns forward geocoding, reverse
geocoding, address normalization, confidence metadata, precision classes,
retention policy, batch jobs, attribution, artifacts, freshness, and redaction.
It must not own map rendering, route calculation, place search, timezone lookup,
device location capture, identity verification, or application address business
workflows.

## Source Baseline

- Google Maps Geocoding API:
  <https://developers.google.com/maps/documentation/geocoding>
- Mapbox Geocoding API:
  <https://docs.mapbox.com/api/search/geocoding/>
- HERE Geocoding and Search:
  <https://www.here.com/docs/category/geocoding-search>
- TomTom Search and Geocoding:
  <https://developer.tomtom.com/search-api/documentation/search-service/search-service>
- Esri World Geocoding:
  <https://developers.arcgis.com/rest/geocode/api-reference/overview-world-geocoding-service.htm>
- Azure Maps Search:
  <https://learn.microsoft.com/en-us/rest/api/maps/search>
- Apple CLGeocoder:
  <https://developer.apple.com/documentation/corelocation/clgeocoder>
- Nominatim usage policy and API:
  <https://operations.osmfoundation.org/policies/nominatim/> and
  <https://nominatim.org/release-docs/latest/api/Overview/>
- Pelias geocoder:
  <https://github.com/pelias/documentation>

## Supplier API Notes

- Google, Mapbox, HERE, TomTom, Esri, and Azure Maps expose forward/reverse
  geocoding, candidate ranking, address components, language/region support,
  bounding boxes, precision, storage restrictions, attribution, quotas, and
  provider-specific errors. Macaca should normalize these as typed candidates,
  confidence, precision class, retention, and attribution diagnostics.
- Apple CLGeocoder contributes host-native geocode/reverse-geocode semantics
  with platform limits and privacy-sensitive coordinate/address handling.
  Macaca should model host-native access as provider capability with approval.
- Nominatim and Pelias contribute open-data geocoding concepts and strong usage
  policy constraints. Macaca should treat rate limits and acceptable use as
  policy/resource constraints, not implementation details.

## Macaca-Owned Abstractions

`pack.location.geocode.v1` should define `GeocodeScope`, `GeocodeQuery`,
`ReverseGeocodeQuery`, `AddressComponentSet`, `GeocodeGeometry`,
`LocationPrecisionClass`, `GeocodeConfidence`, `GeocodeCandidate`,
`GeocodeRetentionPolicy`, `GeocodeBatchJob`, `GeocodeArtifactHandle`,
`GeocodeAttribution`, and `GeocodeRedactionPolicy`.

The DTOs must carry query type, normalized address components, coordinate
precision, candidate geometry, confidence score/class, supported
country/language, retention mode, batch state, provider attribution, artifact
checksum, redaction class, and replay pointers. Private address lists, exact
private coordinates, raw provider responses, raw API keys, provider-specific
place IDs as stable identifiers, and application business address workflows are
rejected.

## Boundary Decisions And Non-Goals

- Maps owns tiles, render artifacts, and map attribution.
- Route owns path and ETA calculation.
- Place-search owns POI discovery and details.
- Timezone owns zone lookup and offset/transition behavior.
- Device owns location capture.
- Identity owns KYC and identity-document workflows.
- Applications own address validation decisions, shipping rules, fraud rules,
  and other business workflows.

## Existing Macaca Platform Inventory

- `crates/foundation/macaca-proto/src/domain_pack_contract/` provides reusable
  descriptor metadata, lifecycle/availability, policy templates, diagnostics,
  SDK metadata, provider snapshots, unavailable diagnostics, and effective
  capability expansion.
- `crates/facade/macaca-sdk/src/system_facade.rs` provides the Facade pattern;
  geocode SDK helpers should only build canonical traced service calls.
- Generic policy, approval, resource, entitlement, trace, audit, artifact,
  mock-provider, and unavailable-provider concepts are reusable, but current
  evidence does not prove geocode-specific DTOs, descriptors, providers, SDK
  helpers, ABI metadata, tests, or developer docs.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
