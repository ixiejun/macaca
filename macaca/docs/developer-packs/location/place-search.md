# Location Place Search Pack

`pack.location.place.search.v1` provides provider-neutral point-of-interest
discovery, text search, nearby/category search, autocomplete suggestions,
suggestion resolution, place details, category taxonomy, field capability
inspection, attribution inspection, and retained search-session purge.

The pack is not a booking, review, routing, geocoding, map-rendering, timezone,
or device-location API. It exposes stable command contracts while provider
adapters remain serviceized and replaceable.

## Manifest Declaration

```toml
[service_contract]
optional_packs = ["pack.location.place.search.v1"]
```

Unavailable optional declarations report
`location_place_search_provider_not_installed`. Required declarations block
readiness when provider, field capability, entitlement, permission, attribution,
or region policy is not satisfied.

## Commands

- `place_search.search`: text search with locale, category filters, field mask,
  and bounded pagination.
- `place_search.nearby`: nearby/category discovery from an explicit spatial
  constraint.
- `place_search.suggest`: interactive autocomplete using provider-owned session
  references.
- `place_search.resolve_suggestion`: resolves a suggestion to a place reference
  or bounded detail stub.
- `place_search.get_details`: retrieves details using a required field mask.
- `place_search.list_categories`: returns normalized `PlaceCategory` rows.
- `place_search.inspect_fields`: returns capability, entitlement, cost, and
  unsupported-field metadata.
- `place_search.inspect_attribution`: returns `PlaceAttribution`.
- `place_search.purge_session`: purges retained autocomplete/search session
  state and emits audit evidence.

## DTOs And Results

Core DTOs include `PlaceSearchCommandContext`,
`PlaceSearchSpatialConstraint`, `PlaceQuery`, `PlaceSummary`, `PlaceDetails`,
`PlaceSuggestion`, `PlaceCategory`, `PlaceAttribution`, `PlaceSearchQuality`,
`PlaceMediaReference`, `PlaceExternalReference`, and `PlaceSearchError`.
Result statuses cover success, partial, denied, unavailable, unsupported,
quota-exceeded, stale-reference, ambiguous-reference, entitlement-required,
attribution-required, provider-failure, and conflict.

## Provider Mapping

Google Places, Mapbox Search Box, HERE Search, Foursquare Places, TomTom Search,
Yelp Fusion, Apple MapKit, and Pelias/open-data search map into text search,
nearby search, suggestions, details, categories, field masks, media references,
quality, attribution, and session management. Raw query text, exact
coordinates, provider payloads, provider session tokens, media bytes, and
supplier-specific ranking rules are not OS-layer semantics.

## App-Facing Examples

Applications declare `pack.location.place.search.v1` and call only typed SDK
commands with synthetic query, session, suggestion, place, field-mask,
attribution, and category refs. Examples use synthetic data only and never log
raw query text, exact coordinates, provider session tokens, or media bytes.

- Search by text and nearby category with explicit locale, pagination, spatial
  constraint, and field-mask refs.
- Start an autocomplete session with `place_search.suggest`, resolve a selected
  suggestion, and purge retained session state when finished.
- Request place details with a minimal field mask and inspect unsupported-field
  metadata before expanding requested fields.
- Inspect attribution before rendering results.
- Handle unavailable-provider diagnostics with
  `location_place_search_provider_not_installed` and provider-neutral denied,
  stale-reference, entitlement-required, attribution-required, quota, and
  conflict statuses.

## Conformance

Provider authors must document descriptor fields, adapter responsibilities,
field masks, pagination, autocomplete-session lifecycle, attribution
translation, unsupported-field behavior, redaction rules, health/snapshot
behavior, replacement strategy, unavailable behavior, and replay-safe audit
evidence.
