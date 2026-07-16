# Change: Add Location Geocode Pack

## Why

Macaca applications need `pack.location.geocode.v1` as an industrial geocoding
capability for forward geocoding, reverse geocoding, structured address parsing,
address normalization, candidate ranking, confidence diagnostics, batch
geocoding, and retention policy evidence. Mature providers expose these
capabilities through address search, reverse lookup, autocomplete-adjacent
geocoding, rooftop/interpolated precision, component-level matches, postal
address schemas, country/region constraints, caching rules, and permanent versus
temporary result restrictions. Macaca must normalize geocoding without becoming
a place-search engine, route planner, map renderer, timezone resolver, device
location provider, address-verification business workflow, or provider-specific
geocoding SDK.

This proposal defines geocoding as a serviceized, provider-neutral pack. It
gives applications typed geocode commands while keeping concrete Google Maps,
Mapbox, HERE, TomTom, Esri, Azure Maps, Apple CLGeocoder, Nominatim/Pelias-style,
batch, offline, mock, and unavailable providers behind replaceable service
providers.

## Supplier And API Baseline

The design is based on mature geocoding APIs:

- Google Maps Geocoding API exposes address geocoding, reverse geocoding,
  address components, place IDs, geometry location types, bounds, plus codes,
  region/language parameters, partial matches, and result type filters.
- Mapbox Geocoding API exposes forward/reverse geocoding, structured input,
  worldview, country/proximity/bbox filters, result types, confidence/match
  codes, permanent/temporary storage modes, and batch-like workflows.
- HERE Geocoding and Search API exposes geocode/reverse operations, structured
  addresses, result scoring, match quality, address labels, access positions,
  map views, and political view/country filters.
- TomTom Geocoding and Reverse Geocoding APIs expose fuzzy geocoding,
  structured geocoding, reverse geocoding, address ranges, entry points,
  bounding boxes, score/confidence, and country/type filters.
- Esri World Geocoding Service exposes find address candidates, reverse
  geocode, geocodeAddresses batch, locator properties, match scores, location
  types, categories, spatial references, and address component output.
- Azure Maps Search exposes address search, structured address search, reverse
  geocoding, batch search, entity type filters, viewport biasing, and scoring.
- Apple CLGeocoder exposes forward and reverse geocoding on Apple platforms with
  placemark results, postal address data, locality/administrative components,
  and platform rate/usage constraints.
- Nominatim and Pelias-style open geocoders expose search/reverse APIs,
  OpenStreetMap-derived addresses, importance/class/type metadata, display
  names, bounding boxes, and strict usage/attribution policies.

Research references:

- Google Maps Geocoding API:
  https://developers.google.com/maps/documentation/geocoding
- Mapbox Geocoding API:
  https://docs.mapbox.com/api/search/geocoding/
- HERE Geocoding and Search API:
  https://www.here.com/docs/bundle/geocoding-and-search-api-developer-guide/page/README.html
- TomTom Geocoding APIs:
  https://developer.tomtom.com/geocoding-api/documentation/product-information/introduction
- Esri World Geocoding Service:
  https://developers.arcgis.com/rest/geocode/api-reference/overview-world-geocoding-service.htm
- Azure Maps Search:
  https://learn.microsoft.com/rest/api/maps/search
- Apple CLGeocoder:
  https://developer.apple.com/documentation/corelocation/clgeocoder
- Nominatim API and usage policy:
  https://nominatim.org/release-docs/latest/api/Overview/ and
  https://operations.osmfoundation.org/policies/nominatim/
- Pelias geocoder:
  https://github.com/pelias/documentation

## Macaca Provider-Neutral Mapping

`pack.location.geocode.v1` maps supplier concepts into stable Macaca contracts:

- Free-form addresses, structured addresses, postal addresses, coordinate
  inputs, place IDs used only as references, and provider query strings become
  `GeocodeQuery` and `ReverseGeocodeQuery`.
- Address components, postal address fields, administrative levels, locality,
  district, street, premise, postal code, country/region, and formatted labels
  become `AddressComponentSet`.
- Rooftop, parcel, street interpolated, approximate, point-of-interest,
  centroid, entrance/access point, and unknown precision become
  `LocationPrecisionClass`.
- Provider scores, confidence, match quality, partial match flags, match codes,
  result type/category, and ambiguity diagnostics become `GeocodeConfidence`.
- Candidate lists, address candidates, features, placemarks, and results become
  `GeocodeCandidate` with rank, precision, geometry, component set, source
  class, attribution, retention policy, and freshness.
- Provider caching rules, permanent/temporary geocoding flags, attribution
  rules, storage restrictions, and derived-data policy become
  `GeocodeRetentionPolicy`.
- Batch geocode jobs, bulk address geocoding, partial successes, and paged
  results become `GeocodeBatchJob` and bounded artifact handles.

## What Changes

- Add provider-neutral `pack.location.geocode.v1` under the location family.
- Define commands for provider inspection, schema discovery, query validation,
  forward geocode, reverse geocode, address normalization, candidate confidence
  inspection, batch planning/request/status/cancel, retention policy inspection,
  attribution inspection, and artifact retrieval.
- Define DTOs for geocode scope, provider capability, geocode queries,
  structured address fields, reverse queries, candidates, geometry, precision,
  confidence, ambiguity, retention policy, attribution, batch jobs, freshness,
  redaction, and artifact handles.
- Require policy, coordinate/address sensitivity controls, retention policy
  enforcement, provider attribution, resource/quota checks, entitlement checks,
  idempotency for batch jobs, sanitized trace/audit, and deterministic
  unavailable/unsupported behavior.
- Require detailed developer documentation at
  `docs/developer-packs/location/geocode.md`.

## Impact

- Affected specs: `pack-location-geocode`,
  `developer-pack-industrial-capability-catalog`, `sdk-system-facade`,
  `service-runtime`, `unified-execution-path`.
- Affected code later: protocol DTOs, pack descriptors, admission validators,
  SDK discovery/command builders, geocode service providers,
  mock/unavailable providers, trace/audit schemas, replay tests, redaction
  tests, retention/attribution tests, batch tests, and boundary gates.

## Non-Goals

- No place search, autocomplete UI, route calculation, map tile/static rendering,
  timezone lookup, device location capture, address verification/KYC workflow,
  delivery optimization, emergency dispatch, or application-specific address
  business rules.
- No provider-specific ranking policy, postal compliance workflow, provider
  billing policy, or geocoding SDK initialization in Macaca OS layers.
- No raw credentials, API keys, access tokens, raw provider responses, private
  address lists, unbounded batch dumps, private manifests, package bytes,
  private keys, signatures, or unsanitized location/address data in logs,
  traces, snapshots, or SDK diagnostics.
- No provider construction or provider-name routing in kernel, SDK, shells, or
  generic application framework.
