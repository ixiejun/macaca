# Location Place Search Pack Design

## Context

`pack.location.place.search.v1` provides provider-neutral POI and place discovery to Macaca applications. Mature suppliers expose overlapping but inconsistent concepts: some optimize for interactive autocomplete, some for global POI databases, some for native device-local search, and some for self-hosted open-data search. Macaca needs a stable contract that lets developers build against one pack while preserving provider replacement, policy gates, attribution compliance, traceability, and data minimization.

The pack is a system-service capability, not an application workflow. The microkernel owns identity, policy facade, service-call evidence, trace, and audit primitives only. Place search semantics live behind a replaceable location place search service provider.

## Supplier Capability Matrix

| Supplier/API | Borrowed capability | Macaca mapping |
| --- | --- | --- |
| Google Places API | Text Search, Nearby Search, Details, Autocomplete, field masks, place IDs, photos, opening hours, business status, ratings, attributions | `search`, `nearby`, `get_details`, `suggest`, `media_references`, `PlaceAttribution`, field mask and cost hints |
| Mapbox Search Box | Suggestions, retrieve, session tokens, category search, proximity, bounding boxes, POI/address feature typing | `suggest`, `resolve_suggestion`, `SearchSession`, `category_filter`, `proximity_bias`, `viewport_filter` |
| HERE Geocoding/Search | Discover, browse, autosuggest, lookup, categories, contacts, hours, food types, relevance scoring | `nearby`, `search`, `suggest`, `get_details`, normalized category/contact/hour fields |
| Foursquare Places | Global POI search/details, rich categories, chains, photos, popularity, tastes, licensing obligations | `PlaceCategory`, `chain`, `popularity`, `media_references`, provenance and display policy |
| TomTom Search | Fuzzy search, POI-only search, radius constraints, typeahead, category sets, result scores | `query`, `nearby`, `radius`, `suggest`, `category_filter`, `quality.score` |
| Yelp Places/Fusion | Business search/details, rating, price, phone, hours, photos, review-derived fields, plan-gated data | `business_profile`, `rating_summary`, `price_tier`, `contact`, `entitlement_required_fields` |
| Apple MapKit | Native local search, completion, region bias, POI categories, place ID, host privacy mediation | device-mediated provider class, `suggest`, `search`, `region_bias`, host approval and foreground rules |
| Pelias | Open-source search/autocomplete/place endpoint, source/layer metadata, confidence, self-hosted deployment | plugin/remote/self-hosted provider class, `source`, `layer`, `confidence`, unavailable-safe replacement |

## Goals

- Provide industrial place search commands for text search, nearby/category discovery, autocomplete, suggestion resolution, details, categories, field capability inspection, attribution inspection, and retained-session purge.
- Normalize supplier capabilities without losing important provider-grade semantics such as field masks, session tokens, attribution, cost hints, result confidence, freshness, localization, and data-retention constraints.
- Preserve a single canonical execution path through SDK/facade clients, service runtime decorators, policy/resource/entitlement checks, and provider adapters.
- Support built-in, plugin, remote, mock, self-hosted, native-host, and unavailable providers through descriptor-driven Strategy selection.
- Provide detailed developer documentation and provider conformance guidance so pack adopters can implement real applications without reading provider-specific source code.

## Non-Goals

- Do not own coordinate-to-address geocoding, address normalization, route planning, map tiles/rendering, timezone lookup, device geolocation capture, booking/payment/order workflows, review-authoring workflows, or application-specific ranking.
- Do not route on provider names in OS, SDK, shell, or generic application-framework code.
- Do not store raw provider payloads, raw media, raw reviews, credentials, secrets, package bytes, prompts, or unbounded result bodies in traces, audits, logs, snapshots, examples, or replay records.
- Do not fake success when a provider is absent or when licensing/entitlement forbids requested fields.

## Ownership And Boundaries

- Pack id: `pack.location.place.search.v1`.
- Capability family: `location`.
- Backing service: location place search service.
- SDK surface: `sdk.packs.location.place_search`.
- Command namespace: `place_search.*`.
- Application framework ownership: manifest declarations, app-scoped permission declarations, effective capability projection, and ABI exposure.
- Service runtime ownership: typed command dispatch, lifecycle, health, snapshots, policy/resource/entitlement decorators, unavailable provider, and provider registration.
- Runtime host ownership: concrete provider adapter composition only through approved composition roots.
- Shell ownership: input parsing, SDK calls, diagnostics rendering, and trace display only.

## Command Surface

| Command | Purpose | Required behavior |
| --- | --- | --- |
| `place_search.search` | Text search for POIs, establishments, landmarks, and named places | Accepts query, locale, region, category filters, field mask, bounds/proximity bias, page cursor, and cost hint; returns ranked `PlaceSummary` results |
| `place_search.nearby` | Nearby/category discovery around a coordinate, viewport, route corridor, or place anchor | Requires explicit spatial constraint, radius/bounds policy, category filter, freshness hint, and bounded page size |
| `place_search.suggest` | Interactive autocomplete/typeahead suggestions | Supports provider-managed session token, input cursor, locale, proximity bias, category/type filters, and ephemeral retention |
| `place_search.resolve_suggestion` | Resolve a suggestion into a stable place reference or details stub | Carries suggestion id/session id and returns a `PlaceReference` or bounded `PlaceDetails` subset |
| `place_search.get_details` | Retrieve normalized details for a stable place reference | Requires field mask, cost/entitlement check, retention/display policy, and attribution metadata |
| `place_search.list_categories` | Discover normalized category taxonomy and provider-supported mappings | Returns category ids, labels, parent/child links, locale labels, and provider support hints |
| `place_search.inspect_fields` | Inspect available normalized fields for the effective provider | Returns capability matrix, entitlement-gated fields, unsupported fields, and cost/rate classes |
| `place_search.inspect_attribution` | Inspect display, retention, and attribution obligations for results | Returns `PlaceAttribution` rules without exposing secrets or raw supplier contracts |
| `place_search.purge_session` | Purge retained autocomplete/search session state | Deletes retained ephemeral session artifacts and emits audit evidence |

All commands are typed Command DTOs with typed success, partial, denied, unavailable, unsupported, quota, conflict, stale, and provider-failure result DTOs. Commands with no external side effect still require trace and policy because they can reveal location-sensitive user intent.

## DTO Model

- `PlaceSearchCommandContext`: application id, tenant id, session id, task id, trace id, locale, region policy, privacy purpose, data-retention class, and optional approval id.
- `PlaceSearchSpatialConstraint`: coordinate, viewport, radius, polygon reference, route-corridor reference, or place anchor. Raw device location is supplied only by the device/location services through declared capability boundaries.
- `PlaceQuery`: text, normalized tokens, category ids, chain ids, type filters, open-now filter, price filters, field mask, sort/ranking hint, and page cursor.
- `PlaceSummary`: stable Macaca place reference, display name, coordinate/viewport, address label, category summaries, distance, confidence, freshness, business status, attribution id, and provider-neutral provenance.
- `PlaceDetails`: summary plus contact, opening hours, website/external links, rating summary, price tier, accessibility hints, media references, external references, localized names, and field-level provenance.
- `PlaceSuggestion`: suggestion id, display text, matched spans, category/type hints, distance/proximity metadata, session reference, and expiry.
- `PlaceCategory`: stable category id, label, locale labels, hierarchy, provider mapping hints, and deprecation metadata.
- `PlaceAttribution`: display text/reference, logo/media reference if required, provider class, result ids covered, retention limit, refresh requirement, and license warning codes.
- `PlaceSearchQuality`: rank, score, confidence, match type, field completeness, freshness, provider confidence, and warning codes.
- `PlaceSearchError`: denied, unavailable, unsupported, quota exceeded, invalid query, invalid spatial constraint, ambiguous reference, stale reference, entitlement required, attribution required, provider failure, or conflict.

## Permission, Policy, Resource, Entitlement, And Approval

Initial scopes:

- `location.place.search.read`: search and nearby results.
- `location.place.autocomplete.read`: interactive suggestions and suggestion resolution.
- `location.place.details.read`: detail lookup and field masks.
- `location.place.categories.read`: category taxonomy.
- `location.place.media.reference.read`: media/photo references only, not raw media download.
- `location.place.session.manage`: purge retained search/autocomplete sessions.

Policy requirements:

- Every command is scoped by application, tenant, session, task, trace, declared purpose, and effective capability.
- Region policy can deny or restrict supplier classes, retention, fields, categories, or result counts.
- Spatial constraints are bounded; large-area or high-cardinality search requires explicit budget and policy approval.
- Field masks are mandatory for details and optional for search; entitlement and provider cost classes are checked before calling providers.
- Autocomplete sessions use ephemeral retention by default and SHALL expose purge behavior.
- Raw provider review bodies, full photos, and credentials are never retained by the generic pack.
- Approval is required when host policy marks a command sensitive because of precise location, background/native provider use, external network disclosure, high quota spend, or retained user-intent data.

## Service Runtime And Provider Strategy

The service uses Adapter and Strategy patterns:

- `PlaceSearchProvider` adapters map provider-specific APIs into Macaca DTOs.
- `PlaceSearchProviderDescriptor` declares supported commands, fields, scopes, rate classes, attribution obligations, retention restrictions, regions, and health.
- `UnavailablePlaceSearchProvider` returns explicit unavailable results and deterministic diagnostics.
- `MockPlaceSearchProvider` supports conformance tests and examples with synthetic data.
- Provider selection is descriptor/policy driven. OS code cannot branch on provider names.
- Provider adapters own supplier session-token handling, field mapping, pagination mapping, attribution translation, and payload redaction.

## SDK Discovery And Developer Documentation

SDK discovery returns pack id, version, lifecycle, command schemas, DTO schemas, permission scopes, effective availability, provider class, command availability, field capability matrix, policy templates, cost/rate hints, attribution obligations, examples, diagnostics, and documentation links.

The implementation SHALL create `docs/developer-packs/location/place-search.md` with:

- Manifest declaration examples for required and optional use.
- Permission scope explanation and policy implications.
- Command-by-command request/result DTO reference.
- Field mask, category, autocomplete session, pagination, and attribution guidance.
- Error taxonomy and unavailable-provider troubleshooting.
- Trace/audit event reference and replay workflow.
- Provider author conformance checklist.
- Generic examples using synthetic place data and no application-specific business logic.

## Trace, Audit, Health, Snapshot, And Replay

Required sanitized events:

- `place_search.pack_declared`
- `place_search.admission_validated`
- `place_search.policy_decision`
- `place_search.entitlement_checked`
- `place_search.resource_reserved`
- `place_search.command_requested`
- `place_search.provider_selected`
- `place_search.command_succeeded`
- `place_search.command_failed`
- `place_search.unavailable`
- `place_search.attribution_recorded`
- `place_search.session_purged`
- `place_search.snapshot_recorded`

Events include pack id, command name, service id, descriptor version, trace id, application/session/task/tenant ids when present, provider class, policy decision, bounded error code, latency, result count, field mask hash, spatial constraint class, attribution id, retention class, and resource counters. Events exclude raw queries when policy forbids them, exact coordinates when redaction requires coarsening, raw provider payloads, secrets, credentials, raw photos, and unbounded review text.

Snapshots include descriptor version, provider health, supported command matrix, supported field matrix, policy template hash, rate/quota class, last health transition, unavailable diagnostics, and sanitized replay pointers.

## Design Patterns

- **Facade**: SDK exposes pack discovery and command builders while `SystemFacade` carries canonical service calls.
- **Command**: each operation is a typed command/result DTO with stable error taxonomy.
- **Adapter**: provider adapters translate Google, Mapbox, HERE, Foursquare, TomTom, Yelp, Apple, Pelias, or future APIs into Macaca DTOs.
- **Strategy**: provider selection, field support, policy routing, cost classes, and unavailable behavior are descriptor-driven.
- **Decorator**: trace, policy, resource, entitlement, approval, metering, and redaction wrap every service call.
- **Specification**: admission validates manifest declarations, scopes, command schemas, spatial constraints, field masks, entitlement, and attribution obligations.
- **Observer**: trace, audit, health, and service events are subscribable and replayable.
- **Memento**: effective capability reports, autocomplete sessions, snapshots, and replay pointers preserve bounded recovery state.
- **Abstract Factory**: provider construction is allowed only in approved runtime composition roots.

## Risks And Mitigations

- Risk: place search overlaps with geocode. Mitigation: this pack owns POI discovery and place detail; geocode keeps address-coordinate conversion and address normalization.
- Risk: supplier field richness leaks provider-specific DTOs. Mitigation: expose normalized optional fields plus provider-neutral provenance, field capability, and unsupported-field diagnostics.
- Risk: autocomplete sessions leak user intent. Mitigation: default ephemeral retention, purge command, redacted trace fields, and policy-governed session lifetime.
- Risk: provider attribution is missed by apps. Mitigation: attribution metadata is included in every result set and SDK discovery exposes display obligations.
- Risk: provider cost/field masks create accidental quota spend. Mitigation: details require field masks and cost/rate classes are surfaced before provider calls.
- Risk: SDK helpers become a second execution path. Mitigation: helpers only build canonical service commands and no-direct-provider-call gates enforce this.
