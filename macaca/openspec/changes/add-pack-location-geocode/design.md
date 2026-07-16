# Location Geocode Pack Design

## Context

`pack.location.geocode.v1` is a child proposal of the developer-pack
industrial capability catalog. It provides geocoding resources as a serviceized
capability: forward geocoding, reverse geocoding, structured address parsing,
address normalization, candidate ranking, precision/confidence diagnostics,
batch geocoding, attribution, retention policy evidence, and artifact handles.

Geocoding providers differ in important ways: result precision, component
schemas, score semantics, caching restrictions, permanent storage rules,
country/region filters, political views, batch behavior, and attribution. Macaca
needs a provider-neutral contract that applications can declare and invoke
without learning provider credentials, result-specific legal restrictions, or
provider-specific response shapes.

## Supplier Capability Matrix

| Supplier or ecosystem | Relevant capability | Macaca interpretation |
| --- | --- | --- |
| Google Maps Geocoding | Forward/reverse geocoding, address components, geometry location type, place ID references, partial matches, plus codes | Candidate records, component sets, precision classes, provider references, ambiguity diagnostics |
| Mapbox Geocoding | Forward/reverse, structured input, bbox/proximity/country filters, worldview, match codes, permanent/temporary modes | Query constraints, confidence diagnostics, retention policy, provider-neutral filters |
| HERE Geocoding and Search | Structured addresses, result scoring, match quality, access positions, map view, political view | Address normalization, confidence, access point references, viewport hints |
| TomTom Geocoding | Fuzzy/structured geocoding, reverse geocoding, scores, address ranges, entry points, bounding boxes | Fuzzy match metadata, precision, entrance/access references, bounding geometry |
| Esri World Geocoding | Address candidates, reverse geocode, batch geocode, match scores, categories, spatial references | Candidate ranking, batch jobs, spatial reference metadata, category references |
| Azure Maps Search | Address search, structured search, reverse, batch, entity type filters, score and viewport biasing | Batch and filter model, confidence scores, bounded result paging |
| Apple CLGeocoder | Forward/reverse geocoding, placemarks, postal address fields, platform constraints | Postal component mapping and device/platform provider adapter boundary |
| Nominatim / Pelias | Open search/reverse, OSM-derived display names, bounding boxes, class/type, usage/attribution policy | Open provider adapter, attribution and rate-limit enforcement, result class/type mapping |

## Goals

- Provide stable pack id `pack.location.geocode.v1` and command namespace
  `geocode.*`.
- Normalize queries, structured address fields, reverse lookup queries,
  candidates, geometry, precision, confidence, ambiguity diagnostics, retention
  policies, attribution bundles, batch jobs, and artifacts.
- Support provider inspection, schema discovery, query validation, forward
  geocoding, reverse geocoding, address normalization, confidence inspection,
  batch planning/request/status/cancel, retention policy inspection,
  attribution inspection, and artifact retrieval through typed DTOs.
- Preserve a single canonical execution path through SDK/facade clients,
  service runtime decorators, and replaceable geocode service providers.
- Return structured `success`, `partial`, `approval_required`, `denied`,
  `unavailable`, `unsupported`, `conflict`, `ambiguous`, `no_match`,
  `stale_version`, `quota_exceeded`, `rate_limited`, `timeout`, `cancelled`,
  and `failure` results.
- Emit sanitized trace, audit, health, snapshot, and replay evidence for every
  declaration, admission, policy decision, service call, provider decision, and
  unavailable state.
- Require detailed developer documentation at
  `docs/developer-packs/location/geocode.md`.

## Non-Goals

- No place search, category/POI discovery, autocomplete UI, route calculation,
  map rendering, timezone lookup, device location capture, address
  verification/KYC workflow, delivery optimization, or application-specific
  address business rules.
- No provider-specific ranking policy beyond normalized confidence metadata.
- No raw API keys, tokens, credentials, raw provider responses, unbounded
  address lists, private manifests, package bytes, private keys, signatures, or
  unsanitized location/address data in observability surfaces.

## Ownership And Boundaries

- Pack id: `pack.location.geocode.v1`.
- Family: `location`.
- Backing service owner: replaceable geocode service provider.
- SDK surface: `sdk.packs.location.geocode`.
- Command namespace: `geocode.*`.
- Microkernel ownership: service-call evidence, policy facade, resource facade,
  trace/audit primitives, and scheduling primitives only.
- Application framework ownership: manifest declaration, app-scoped permission
  declarations, lifecycle projection, and effective capability mementos.
- Runtime-host ownership: provider registration, service runtime decorators,
  transport adapters, health/snapshot bridge, and unavailable/mock provider
  composition through approved composition roots.

## Command Surface

All commands carry trace context, application/session/task/tenant identifiers
when available, policy context, idempotency key for batch jobs, redaction
profile, resource budget, retention intent, and replay metadata.

| Command | Purpose | Notes |
| --- | --- | --- |
| `geocode.inspect_provider` | Return provider capability metadata | Reports forward/reverse/structured/batch/permanent support, limits, health, and unavailable reasons |
| `geocode.discover_schema` | Return address and result schema metadata | Exposes component fields, filters, languages, countries, precision classes, confidence fields, and retention rules |
| `geocode.validate_query` | Validate query without provider side effects | Checks address/coordinate shape, language, region, bbox/proximity filters, retention intent, and policy |
| `geocode.forward` | Convert address/query to candidate coordinates | Returns ranked candidates with components, precision, confidence, attribution, and retention policy |
| `geocode.reverse` | Convert coordinates to address candidates | Enforces coordinate precision, region policy, radius, result type filters, and redaction |
| `geocode.normalize_address` | Normalize structured or free-form address | Returns component set, formatted labels, missing/ambiguous components, and confidence metadata |
| `geocode.inspect_confidence` | Explain candidate confidence/ambiguity | Provides bounded reason codes, match classes, precision class, and provider score references |
| `geocode.plan_batch` | Validate batch geocode job | Checks input count, sensitivity, retention intent, quota, provider support, and artifact policy |
| `geocode.request_batch` | Start batch geocode job | Requires idempotency, timeout/cancellation, partial-result handling, and artifact metadata |
| `geocode.inspect_batch` | Inspect batch status/results metadata | Returns progress, counters, partial failures, artifact handles, and redaction state |
| `geocode.cancel_batch` | Cancel batch job where supported | Returns cancellation status and bounded audit evidence |
| `geocode.inspect_retention` | Inspect caching/permanent-storage policy | Returns temporary/permanent/storage restrictions and attribution requirements |
| `geocode.inspect_attribution` | Return attribution/source requirements | Provides source notices and display requirements for candidates/artifacts |
| `geocode.get_artifact` | Retrieve batch/export artifact metadata | Does not expose raw provider payloads or unbounded address lists |

## Provider-Neutral DTO Model

- `GeocodeScope`: application id, tenant id, session/task identifiers, region
  policy reference, provider reference, and trace context.
- `GeocodeQuery`: free-form query, structured address reference, language,
  region, country filters, bounding box, proximity hint, result type filters,
  retention intent, and redaction class.
- `ReverseGeocodeQuery`: coordinate, radius, precision class, result type
  filters, language, region policy, retention intent, and redaction class.
- `AddressComponentSet`: house number, street, unit, neighborhood, locality,
  district, region, postal code, country, country code, formatted labels,
  administrative levels, and missing/ambiguous component metadata.
- `GeocodeGeometry`: coordinate, bounding box, viewport hint, access point,
  entrance point, centroid, spatial reference, and precision class.
- `LocationPrecisionClass`: rooftop, parcel, address_point, interpolated,
  street, locality, administrative_area, postal_code, centroid, poi_reference,
  approximate, unknown.
- `GeocodeConfidence`: normalized score, provider score reference, match type,
  partial match flag, ambiguity class, result rank, component match summary, and
  bounded explanation codes.
- `GeocodeCandidate`: candidate handle, component set, geometry, precision,
  confidence, provider reference, source class, attribution bundle, retention
  policy, freshness, and redaction metadata.
- `GeocodeRetentionPolicy`: temporary/permanent mode, storage allowed flag,
  cache TTL class, derived-data restrictions, attribution requirement, and
  provider terms reference.
- `GeocodeBatchJob`: job handle, input count, completed count, failed count,
  partial-result state, artifact handles, retention policy, cancellation state,
  and replay cursor.
- `GeocodeArtifactHandle`: artifact id, content class, redaction state,
  retention deadline, size class, checksum/hash, and retrieval permissions.

## Permission, Policy, Resource, Entitlement, And Approval Model

Initial permission scopes:

- `location.geocode.forward`
- `location.geocode.reverse`
- `location.geocode.normalize`
- `location.geocode.confidence.read`
- `location.geocode.batch`
- `location.geocode.retention.read`
- `location.geocode.attribution.read`
- `location.geocode.artifact.read`

Policy checks run before provider calls. Inputs include caller subject,
application id, tenant id, command, address sensitivity, coordinate precision,
country/region policy, retention intent, batch size, result field mask,
provider storage mode, attribution requirement, resource budget, approval state,
and entitlement state.

Approval is required when policy marks a geocode operation as sensitive, such as
processing private residential addresses, precise reverse geocoding, retained
batch artifacts, permanent storage modes, or regulated region/data-boundary
crossings.

Resource checks cover query count, batch size, candidate count, address length,
component count, provider quota, network budget, timeout, artifact size,
retained snapshots, retained artifacts, and event volume.

Entitlement checks determine whether the calling application/tenant may use
forward geocode, reverse geocode, structured address parsing, batch geocode,
permanent storage, high-confidence/rooftop precision, and retained artifacts.

## Service Runtime And Provider Strategy

The geocode service provider is a Strategy behind the service runtime. The
runtime composes provider adapters, unavailable providers, mock providers,
policy decorators, resource decorators, entitlement decorators, metering,
redaction, retention enforcement, attribution enforcement, trace, audit,
timeout/cancellation, and health/snapshot behavior.

Provider adapters may target Google Maps, Mapbox, HERE, TomTom, Esri, Azure
Maps, Apple CLGeocoder, Nominatim/Pelias-compatible providers, offline geocoder
providers, built-in local providers, remote providers, plugin providers, or mock
providers. Provider-specific capabilities are descriptor data, not OS routing
branches.

The unavailable provider is first-class. It exposes descriptor metadata, health
state, unsupported command diagnostics, and stable error DTOs without crashing,
hanging, silently falling back, contacting undeclared providers, or faking
success.

## SDK Discovery And Developer Documentation

SDK discovery must return pack metadata, command schemas, permission scopes,
forward/reverse support, structured address support, batch support, supported
countries/languages, precision classes, confidence fields, retention modes,
attribution requirements, examples, availability, diagnostics, provider class,
compatibility hash, redaction profile, and documentation link.

SDK helper builders only build canonical traced service calls. They must never
construct providers, hold credentials, call provider APIs directly, search
places, calculate routes, render maps, capture device location, verify identity
documents, or bypass retention/policy.

Developer documentation at `docs/developer-packs/location/geocode.md` must
cover purpose, non-goals, manifest declaration, permission scopes, command DTOs,
result DTOs, provider mapping, forward/reverse examples, batch examples,
retention/attribution rules, unavailable diagnostics, trace/audit events,
redaction, snapshot/replay, and provider-author conformance checks.

## Trace, Audit, Health, Snapshot, And Replay

Events include pack id, descriptor version, command name, trace id,
application/session/task/tenant identifiers when available, query hash,
coordinate hash at approved precision, policy decision, approval state, provider
class, latency, bounded resource counters, capability hash, retention policy
hash, attribution hash, and bounded error code.

Events, snapshots, SDK diagnostics, and examples must exclude raw credentials,
API keys, access tokens, raw provider responses, private address lists,
unbounded batch dumps, private manifests, package bytes, private keys,
signatures, and unsanitized location/address data.

Snapshots include descriptor version, provider capability hash, command
availability, provider health, schema hash, supported precision classes,
retention-policy hash, attribution hash, batch summary, resource counters,
artifact summaries, event cursors, and sanitized replay pointers.

## Design Patterns

- **Facade**: `SystemFacade` and focused SDK clients expose discovery and typed
  command builders while hiding service runtime and provider composition.
- **Command**: every operation is represented as a typed command/result DTO.
- **Adapter/Bridge**: Google, Mapbox, HERE, TomTom, Esri, Azure, Apple,
  Nominatim/Pelias, offline, built-in, plugin, remote, mock, and unavailable
  providers adapt into the same contract.
- **Strategy**: provider selection, confidence mapping, retention handling,
  batch behavior, attribution behavior, and unavailable behavior are
  replaceable.
- **Decorator**: trace, audit, policy, resource, entitlement, approval,
  metering, timeout, cancellation, retention, attribution, and redaction wrap
  every call.
- **State**: batch jobs, artifacts, provider lifecycle, and retention states are
  explicit and replayable.
- **Observer**: trace, audit, health, and service events are subscribable by
  shells without giving shells semantic ownership.
- **Memento**: effective capability reports, snapshots, provider capability
  hashes, schema hashes, retention hashes, and audit cursors preserve bounded
  recovery state.
- **Specification**: admission validates pack id, command availability,
  permissions, provider health, entitlement, resource budgets, retention,
  attribution, and policy templates.
- **Abstract Factory**: concrete provider adapters are constructed only in
  approved composition roots.

## Risks And Mitigations

- Risk: geocode becomes place search or autocomplete. Mitigation: geocode owns
  address/coordinate candidates only; place-search owns POI/search semantics.
- Risk: provider retention restrictions are ignored. Mitigation: every result
  carries `GeocodeRetentionPolicy` and permanent/batch modes are policy gated.
- Risk: reverse geocoding leaks private locations. Mitigation: coordinate
  precision, region policy, redaction, and approval gates run before calls.
- Risk: SDK helpers become provider SDK wrappers. Mitigation: helpers only build
  canonical service commands and never hold credentials.
- Risk: confidence scores are misrepresented. Mitigation: normalized confidence
  includes provider score references and bounded explanation codes rather than
  pretending all providers share the same score semantics.
