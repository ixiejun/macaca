# Location Place Search Pack Research

## Purpose

This note records supplier/API research, Macaca provider-neutral mapping,
boundary decisions, and GitNexus memo evidence for
`pack.location.place.search.v1`. The place-search pack owns text search,
nearby search, autocomplete/suggestions, suggestion resolution, details with
field masks, category listing, field capability inspection, attribution
inspection, autocomplete session purge, freshness, and redaction. It must not
own map rendering, geocoding, routing, timezone lookup, device location capture,
media processing, or application-specific POI/business workflows.

## Source Baseline

- Google Places API:
  <https://developers.google.com/maps/documentation/places/web-service>
- Mapbox Search Box and Search APIs:
  <https://docs.mapbox.com/api/search/>
- HERE Geocoding and Search:
  <https://www.here.com/docs/category/geocoding-search>
- Foursquare Places API:
  <https://docs.foursquare.com/data-products/docs/places-api>
- TomTom Search API:
  <https://developer.tomtom.com/search-api/documentation/search-service/search-service>
- Yelp Fusion API:
  <https://docs.developer.yelp.com/reference/v3_business_search>
- Apple MapKit search:
  <https://developer.apple.com/documentation/mapkit/mklocalsearch>
- Pelias open-data search:
  <https://github.com/pelias/documentation>

## Supplier API Notes

- Google Places, Mapbox Search, HERE, Foursquare, TomTom, Yelp, Apple MapKit,
  and Pelias expose text search, nearby/category queries, autocomplete,
  details, categories, photos/media references, opening hours, ratings, rich
  fields, attribution, session tokens, quotas, and field-mask/cost behavior.
  Macaca should normalize these into field capability matrices, session handles,
  attribution requirements, and cost/resource classes.
- Provider APIs differ on field availability, category taxonomies, autocomplete
  session retention, media licensing, and rich profile data. Macaca should
  require explicit field masks and entitlement checks before costly or sensitive
  provider calls.
- Open-data search engines contribute replaceable/self-hosted provider
  patterns. Macaca should model them through provider descriptors without
  branching on provider names in OS-layer commands.

## Macaca-Owned Abstractions

`pack.location.place.search.v1` should define
`PlaceSearchCommandContext`, `PlaceSearchSpatialConstraint`, `PlaceQuery`,
`PlaceSummary`, `PlaceDetails`, `PlaceSuggestion`, `PlaceCategory`,
`PlaceAttribution`, `PlaceSearchQuality`, `PlaceMediaReference`,
`PlaceExternalReference`, `PlaceSearchSession`, and `PlaceSearchError`.

The DTOs must carry query text redaction class, spatial constraint,
coordinate precision, category taxonomy reference, field mask, details field
capability, autocomplete session handle, provider attribution, quality/freshness
metadata, media reference bounds, external reference class, retention policy,
and replay pointers. Raw provider payloads, exact private coordinates, raw
session tokens, credentials, unbounded result dumps, and application business
ranking rules are rejected.

## Boundary Decisions And Non-Goals

- Maps owns tiles/rendering and map artifacts.
- Geocode owns address/coordinate conversion.
- Route owns path calculation and ETA.
- Timezone owns zone lookup.
- Device owns location capture.
- Media owns media processing beyond bounded place media references.
- Applications own product ranking, booking, delivery, review moderation, and
  other business workflows.

## Existing Macaca Platform Inventory

- `crates/foundation/macaca-proto/src/domain_pack_contract/` provides reusable
  descriptor metadata, lifecycle/availability, policy templates, diagnostics,
  SDK metadata, provider snapshots, unavailable diagnostics, and effective
  capability expansion.
- `crates/facade/macaca-sdk/src/system_facade.rs` provides the Facade pattern;
  place-search SDK helpers should only build canonical traced service calls.
- Generic policy, approval, resource, entitlement, trace, audit, artifact,
  mock-provider, and unavailable-provider concepts are reusable, but current
  evidence does not prove place-search-specific DTOs, descriptors, providers,
  SDK helpers, ABI metadata, tests, or developer docs.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
