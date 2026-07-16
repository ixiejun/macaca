# Change: Add Industrial Location Place Search Pack

## Why

Macaca applications need a real `pack.location.place.search.v1` capability for point-of-interest discovery, interactive place search, nearby/category search, place detail retrieval, autocomplete, photos/media references, opening-hour/contact metadata, and attribution compliance. The current catalog entry is too shallow to be useful for developers because it does not define a supplier-grade command surface, data governance model, quality semantics, or provider replacement contract.

This proposal turns place search into a provider-neutral, serviceized industrial pack. Applications declare it in manifests, SDK clients discover callable commands, policy/entitlement/resource gates run before provider calls, and all results are observable through sanitized trace and audit evidence.

## Supplier/API Baseline

The design is based on supplier-level comparison of mature place search platforms:

- Google Places API: Text Search, Nearby Search, Place Details, Autocomplete, field masks, photos, place IDs, ratings, business status, opening hours, address components, types, localization, and attribution requirements. Official docs: https://developers.google.com/maps/documentation/places/web-service/overview and https://developers.google.com/maps/documentation/places/web-service/text-search
- Mapbox Search Box API: interactive suggestions, retrieve-by-suggestion, session tokens, standalone search, category search, POI/address features, proximity, bounding boxes, navigation/use-case metadata, and attribution. Official docs: https://docs.mapbox.com/api/search/search-box/
- HERE Geocoding and Search API v7: discover, browse, autosuggest, lookup, category filters, position bias, language, result scoring, access/place IDs, contacts, opening hours, food types, and supplier attribution. Official docs: https://docs.here.com/geocoding-and-search/docs/introduction-to-here-geocoding-search-api-v7
- Foursquare Places API: global POI data, place search, place details, categories, chains, photos, tips/review-derived fields, popularity, tastes, hours, and licensing restrictions. Official docs: https://docs.foursquare.com/data-products/docs/places-api
- TomTom Search API: fuzzy search, POI search, category search, geometry bias, radius constraints, typeahead behavior, result scoring, and POI classification. Official docs: https://developer.tomtom.com/search-api/documentation/product-information/introduction and https://developer.tomtom.com/search-api/documentation/search-service/points-of-interest-search
- Yelp Fusion/Places APIs: business search, business details, phone/search constraints, categories, ratings, price, hours, photos, review excerpts, locale, plan-gated fields, and display/attribution rules. Official docs: https://docs.developer.yelp.com/docs/places-intro, https://docs.developer.yelp.com/reference/v3_business_search, and https://docs.developer.yelp.com/reference/v3_business_info
- Apple MapKit: `MKLocalSearch`, `MKLocalSearch.Request`, `MKLocalSearchCompleter`, place IDs, local search results, POI category filters, region bias, and host platform privacy mediation. Official docs: https://developer.apple.com/documentation/mapkit/mklocalsearch and https://developer.apple.com/documentation/mapkit/mklocalsearchcompleter
- Pelias/open-data search: open-source place search, autocomplete, place endpoint, source/layer metadata, confidence, label construction, and self-hosted replacement patterns. Official docs: https://github.com/pelias/documentation and https://pelias.io/

## Macaca Provider-Neutral Mapping

Macaca SHALL map supplier-specific features into stable DTOs rather than exposing provider payloads:

- Text and nearby discovery become `place_search.search` and `place_search.nearby`.
- Interactive suggestion flows become `place_search.suggest` and `place_search.resolve_suggestion`, with session-token semantics owned by the provider adapter.
- Category and chain lookup become `place_search.list_categories` and `place_search.search`.
- Place detail lookup becomes `place_search.get_details`, with field selection and cost hints.
- Photos, media, tips, reviews, and external URLs become bounded `PlaceMediaReference` and `PlaceExternalReference` DTOs; raw supplier assets are never copied into traces.
- Opening hours, business status, contacts, rating, price, accessibility, address, coordinate, viewport, category, provenance, freshness, and attribution become normalized optional fields with provider confidence metadata.
- Licensing and display obligations become `PlaceAttribution`, `retention_policy`, and `display_policy` metadata that SDK clients and shells can inspect.

## What Changes

- Add `pack.location.place.search.v1` as a service-backed industrial pack under the location family.
- Define provider-neutral command DTOs for search, nearby, suggest, resolve suggestion, details, categories, field capability inspection, attribution inspection, and retention purge.
- Define normalized `Place`, `PlaceSummary`, `PlaceDetails`, `PlaceSuggestion`, `PlaceCategory`, `PlaceMediaReference`, `PlaceAttribution`, `PlaceSearchQuality`, and structured error DTOs.
- Define permission scopes for place discovery, details, autocomplete, media references, category metadata, and retained search sessions.
- Define policy, resource, entitlement, approval, data retention, attribution, localization, and region-governance behavior.
- Require SDK discovery metadata, examples, provider conformance tests, unavailable-provider behavior, trace/audit events, snapshot/replay evidence, and detailed developer documentation under `docs/developer-packs/location/place-search.md`.

## Impact

- Affected specs: `pack-location-place-search`, `developer-pack-industrial-capability-catalog`, `sdk-system-facade`, `service-runtime`, `unified-execution-path`.
- Later affected code: protocol DTO crates, pack descriptor registry, application admission, SDK pack clients, place search service provider contract, provider adapters, unavailable provider, trace/audit schemas, boundary gates, and developer documentation.
- Validation: `openspec validate add-pack-location-place-search --strict`, DTO compatibility tests, provider conformance tests, canonical service-path tests, no-direct-provider-call gates, data redaction tests, and docs coverage checks.

## Non-Goals

- This pack does not own forward/reverse geocoding, route calculation, map rendering, timezone lookup, device location capture, booking/order/payment flows, review authoring workflows, or application-specific ranking rules.
- This pack does not hardcode Google, Mapbox, HERE, Foursquare, TomTom, Yelp, Apple, Pelias, or any future provider into OS-layer routing.
- This pack does not expose raw provider payloads, credentials, secrets, unbounded photos, review bodies, or application-specific business logic in SDK examples, traces, audits, snapshots, or logs.
