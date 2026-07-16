## 1. Research, Scope, And Governance

- [x] 1.1 Re-read `macaca-os-architecture-governance.md`, `macaca-os-microkernel-boundaries.md`, `macaca-os-serviceization-allowlist.md`, `design_patterns.md`, the umbrella industrial catalog proposal, and this child proposal before implementation.
- [x] 1.2 Record supplier-level comparison notes for Google Places, Mapbox Search Box, HERE Geocoding/Search, Foursquare Places, TomTom Search, Yelp Places/Fusion, Apple MapKit, and Pelias/open-data search.
- [x] 1.3 Confirm final ownership boundaries with the existing `pack.location.maps`, `pack.location.geocode`, `pack.location.route`, and `pack.location.timezone` proposals so POI discovery does not absorb map rendering, geocoding, routing, timezone, or device-location capture.
- [x] 1.4 Record GitNexus CRITICAL/HIGH findings as memo-only evidence before implementation commits, per the current refactor instruction.

## 2. Contract, Descriptor, And DTO Schema

- [x] 2.1 Define provider-neutral command DTOs for `place_search.search`, `place_search.nearby`, `place_search.suggest`, `place_search.resolve_suggestion`, `place_search.get_details`, `place_search.list_categories`, `place_search.inspect_fields`, `place_search.inspect_attribution`, and `place_search.purge_session`.
- [x] 2.2 Define `PlaceSearchCommandContext`, `PlaceSearchSpatialConstraint`, `PlaceQuery`, `PlaceSummary`, `PlaceDetails`, `PlaceSuggestion`, `PlaceCategory`, `PlaceAttribution`, `PlaceSearchQuality`, `PlaceMediaReference`, `PlaceExternalReference`, and `PlaceSearchError`.
- [x] 2.3 Define typed success, partial, denied, unavailable, unsupported, quota-exceeded, stale-reference, ambiguous-reference, entitlement-required, attribution-required, provider-failure, and conflict results.
- [x] 2.4 Define descriptor metadata for pack id, family, lifecycle, command schemas, field capability matrix, permissions, policy template, retention policy, attribution obligations, resource budgets, rate/cost classes, SDK metadata, compatibility, diagnostics, and documentation links.
- [x] 2.5 Add stable descriptor hashing, version compatibility checks, DTO snapshot fixtures, and schema migration tests.

## 3. Permission, Policy, Resource, Entitlement, And Approval

- [x] 3.1 Implement declaration validation for `location.place.search.read`, `location.place.autocomplete.read`, `location.place.details.read`, `location.place.categories.read`, `location.place.media.reference.read`, and `location.place.session.manage`.
- [ ] 3.2 Require field masks for details commands and apply entitlement/cost checks before provider calls for expensive or plan-gated fields such as media references, rating summaries, business profile, external references, and rich opening hours.
- [ ] 3.3 Enforce spatial-boundary, region, locale, retention, provider-class, and result-count policies before dispatch.
- [ ] 3.4 Add resource reservation and quota checks for network calls, provider rate class, retained autocomplete sessions, page size, result count, and snapshot size.
- [ ] 3.5 Add approval behavior for precise-location disclosure, native foreground/background host restrictions, external network disclosure, high-spend searches, and retained user-intent sessions.
- [ ] 3.6 Add tests proving denied, unavailable, unsupported, and quota paths do not call concrete providers.

## 4. Service Provider And Replacement Strategy

- [ ] 4.1 Implement the location place search service provider contract behind the service runtime; do not construct providers in the kernel, SDK, shells, or generic application framework.
- [ ] 4.2 Add `PlaceSearchProviderDescriptor` with supported commands, fields, regions, locale behavior, attribution obligations, retention restrictions, rate/cost classes, and health state.
- [x] 4.3 Add Adapter implementations or fixtures for at least one mock provider and one unavailable provider; provider-specific adapters for external suppliers must remain optional modules or plugin/remote providers.
- [ ] 4.4 Add provider conformance tests for search, nearby, suggest, resolve suggestion, details, categories, field inspection, attribution inspection, purge, redaction, pagination, and unsupported-field reporting.
- [ ] 4.5 Add lifecycle, health, snapshot, shutdown, timeout, cancellation, pagination cursor, and bounded output behavior.

## 5. SDK, Admission, Examples, And ABI

- [x] 5.1 Extend SDK discovery for `pack.location.place.search.v1` with command schemas, DTO schemas, permission scopes, examples, field capability matrix, availability, diagnostics, provider class, compatibility, cost/rate hints, attribution rules, and documentation URL.
- [x] 5.2 Extend application admission so required declarations block when unavailable and optional declarations degrade explicitly with effective capability mementos.
- [x] 5.3 Add SDK command helper builders that only produce canonical traced service calls and never construct providers or branch on provider names.
- [ ] 5.4 Add WASM/application ABI exposure for the pack using provider-neutral DTO schemas and canonical service-call dispatch.
- [x] 5.5 Add generic examples for text search, nearby category search, autocomplete plus resolve, details with field mask, attribution inspection, and unavailable-provider diagnostics using synthetic data only.

## 6. Trace, Audit, Replay, And Boundary Gates

- [ ] 6.1 Emit sanitized `place_search.pack_declared`, `place_search.admission_validated`, `place_search.policy_decision`, `place_search.entitlement_checked`, `place_search.resource_reserved`, `place_search.command_requested`, `place_search.provider_selected`, `place_search.command_succeeded`, `place_search.command_failed`, `place_search.unavailable`, `place_search.attribution_recorded`, `place_search.session_purged`, and `place_search.snapshot_recorded` events.
- [ ] 6.2 Add replay tests proving every command is trace-addressable through the canonical service path after refresh/restart.
- [ ] 6.3 Add dependency-boundary gates proving the microkernel, SDK, shells, and generic application framework do not import concrete place search providers.
- [ ] 6.4 Add no-direct-provider-call gates proving all place search commands enter through descriptor-owned service registrations and typed service runtime dispatch.
- [ ] 6.5 Add redaction tests for query text, exact coordinates, media references, attribution data, provider payloads, credentials, session tokens, and snapshots.
- [ ] 6.6 Run `openspec validate add-pack-location-place-search --strict`, DTO compatibility tests, targeted cargo tests, service-boundary tests, file-size gates, and audit replay checks before marking implementation tasks complete.

## 7. Developer Documentation

- [x] 7.1 Create `docs/developer-packs/location/place-search.md` with purpose, manifest declarations, required/optional behavior, permission scopes, policy implications, command DTOs, result DTOs, examples, field masks, pagination, autocomplete sessions, attribution, retention, unavailable diagnostics, and trace/audit behavior.
- [x] 7.2 Add provider author documentation covering descriptor fields, adapter responsibilities, conformance tests, attribution translation, unsupported-field behavior, redaction rules, health/snapshot behavior, and replacement strategy.
- [x] 7.3 Add at least one minimal app-facing example, one autocomplete-session example, one details-with-field-mask example, and one unavailable-provider diagnostic example using generic synthetic data.
- [x] 7.4 Cross-link the guide from SDK discovery metadata and the industrial pack catalog index before marking `add-pack-location-place-search` complete.
