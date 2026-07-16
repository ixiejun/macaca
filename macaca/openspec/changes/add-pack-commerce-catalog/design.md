# Commerce Catalog Pack Design

## Context

`pack.commerce.catalog.v1` is Macaca's provider-neutral commerce catalog
capability. It normalizes products, variants, SKUs, attributes, prices,
availability views, categories, publication scopes, media, and search across
commerce providers. The pack is a source-of-truth and discovery capability; it
does not own cart, order, payment, receipt, entitlement, or fulfillment
semantics.

Provider differences are intentionally hidden behind service provider Strategy
adapters and exposed only through capability descriptors, schema discovery, and
typed unsupported/degraded results.

## Supplier Capability Matrix

| Supplier family | Relevant capabilities | Constraints Macaca must model |
| --- | --- | --- |
| Shopify | Products, variants, options, metafields, inventory items/levels, images, localized pricing/content, price lists | Variant-first purchasable units, inventory item separation, access scopes, bulk operations, price-list/channel contexts |
| Stripe | Products, Prices, recurring/one-time terms, lookup keys, active/archive state | Product/price separation, limited inventory semantics, billing-oriented catalog, recurring price metadata |
| Square | Catalog items, variations, categories, modifiers, taxes, discounts, images; Inventory API for counts | Catalog/inventory separation, seller/location context, object versioning, batch upserts |
| commercetools | Products, product types, variants, attributes, categories, projections, product search, selections, standalone prices, stores/channels, inventory | Projection/search semantics, localization, price selection, publish state, versioned updates, large catalogs |
| BigCommerce-style storefronts | Products, variants, options, modifiers, custom fields, price lists, channels, categories, inventory summaries, media | Store/channel publication, price-list context, custom fields, inventory summary freshness |

## Goals

- Provide catalog schema discovery, product/variant read and search, taxonomy,
  media references, price lookup, availability lookup, mutation planning,
  product/variant create/update/archive, publish/unpublish, projection search,
  and export.
- Make provider limitations discoverable before invocation, including product
  model, variant model, price model, inventory/availability model, localization,
  publication, batch, and export support.
- Keep catalog mutation separate from inventory adjustment, order/cart checkout,
  payments, receipts, and entitlements.
- Preserve trace, audit, policy, entitlement, resource, approval, idempotency,
  redaction, and replay across every command.

## Non-Goals

- Cart, order, checkout, payment, receipt, entitlement, fulfillment, shipment,
  inventory adjustment, or storefront UI rendering.
- Provider-specific merchandising rules, tax calculation, promotion engines,
  template rules, price-selection business policy, or application-specific
  catalog workflows.
- Raw provider payloads, provider search DSLs, credentials, unpublished secrets,
  unbounded exports, or full media bytes in observability.

## Ownership And Boundaries

- Pack id: `pack.commerce.catalog.v1`.
- Family: `commerce`.
- Backing service owner: commerce catalog service provider family.
- SDK surface: `sdk.packs.commerce.catalog`.
- Command namespace: `catalog.*`.
- Kernel ownership: identity, service-call evidence, policy facade, trace/audit
  primitives, and resource primitives only.
- Application framework ownership: manifest declaration, app-scoped permission
  declarations, lifecycle projection, and effective-capability mementos.
- Runtime-host ownership: provider registration, decorators, mock/unavailable
  providers, and adapter composition through approved composition roots.
- Service ownership: provider capability discovery, schema normalization,
  command dispatch, mutation planning, provider strategy selection, redaction,
  and sanitized audit.

## Command Surface

| Command | Purpose | Side-effect class |
| --- | --- | --- |
| `catalog.inspect_provider` | Return provider capability, model support, limits, freshness, and attribution | Read-only |
| `catalog.describe_schema` | Return product/variant/attribute/price/availability schema and lifecycle metadata | Read-only |
| `catalog.list_products` | List product summaries with bounded pagination | Read-only |
| `catalog.get_product` | Retrieve one normalized product projection | Read-only |
| `catalog.list_variants` | List purchasable variants/SKUs for products or search criteria | Read-only |
| `catalog.get_variant` | Retrieve one normalized variant/SKU | Read-only |
| `catalog.search_catalog` | Search products/variants with filters, facets, sorting, localization, and projection context | Read-only/async |
| `catalog.list_taxonomy` | Retrieve categories, collections, channels, stores, or product selections | Read-only |
| `catalog.get_price` | Resolve price for variant/product under `PriceContext` | Read-only |
| `catalog.check_availability` | Read availability snapshot by variant/location/channel | Read-only |
| `catalog.plan_product_mutation` | Validate create/update/archive/publish changes without provider mutation | Planning |
| `catalog.product_request` | Apply approved product create/update/archive/publish mutation | Mutating |
| `catalog.plan_variant_mutation` | Validate variant/SKU changes without provider mutation | Planning |
| `catalog.variant_request` | Apply approved variant create/update/archive mutation | Mutating |
| `catalog.plan_media_mutation` | Validate media attach/detach/update request | Planning |
| `catalog.media_request` | Apply approved media metadata/reference mutation | Mutating metadata |
| `catalog.plan_export` | Plan catalog export scope, format, resource, redaction, and retention | Planning |
| `catalog.export_catalog` | Produce export artifact handle through approved path | Mutating/export |
| `catalog.get_artifact_handle` | Retrieve export artifact metadata without raw payload leakage | Read-only |

Every command must define typed command DTOs, typed success DTOs, typed partial
or async result shapes, typed denied/unavailable/unsupported/conflict/quota/
stale-data/failure DTOs, idempotency for side effects, pagination/async behavior,
redaction policy, and replay metadata.

## Provider-Neutral DTO Model

- `CatalogScope`: application, tenant, session, task, store/channel, locale,
  currency, customer group, distribution channel, product/variant handles, and
  permission scope.
- `CatalogProviderCapability`: product model, variant model, price model,
  availability model, localization support, taxonomy support, publication
  support, batch/export support, search/facet support, mutation support,
  freshness model, limits, and attribution.
- `CatalogProduct`: product handle, title, descriptions, status, product type,
  vendor/brand, taxonomy, attributes, media references, publication scopes,
  created/updated evidence, and provider version token.
- `CatalogVariant`: variant handle, SKU, barcode, option values, purchasable
  flag, inventory tracking flag, shipping/customs metadata, media references,
  default price references, and provider version token.
- `CatalogAttribute`, `CatalogOption`, `CatalogModifier`: typed and localized
  metadata with validation rules and provider support flags.
- `CatalogPrice`, `PriceBook`, `PriceContext`: amount, currency, billing mode,
  recurring terms, customer group, channel, country, tax-inclusion flag,
  effective dates, and provider price reference.
- `AvailabilitySnapshot`: quantity status, availability status, location/channel,
  freshness, provider attribution, and inventory-service handoff metadata.
- `CatalogTaxonomyNode`, `CatalogPublicationScope`, `CatalogChannel`: category,
  collection, store, channel, product selection, and publication state.
- `CatalogSearchRequest`, `CatalogSearchResult`, `CatalogProjection`: query,
  filters, facets, sort, localization, price context, availability context,
  result rows, scores, and unsupported filter diagnostics.
- `CatalogMutationPlan`, `CatalogMutationResult`, `CatalogArtifactHandle`:
  normalized mutation, provider preconditions, approval requirements, checksum,
  retention, redaction, and replay pointer.

## Permission, Policy, Resource, Entitlement, And Approval

Initial scopes:

- `commerce.catalog.read`
- `commerce.catalog.search`
- `commerce.catalog.price`
- `commerce.catalog.availability`
- `commerce.catalog.write`
- `commerce.catalog.publish`
- `commerce.catalog.export`

Policy defaults:

- Scope every command to application id, tenant id, session id, task id, trace id,
  store/channel, locale, currency, customer group, and product/variant handles.
- Require approval for product/variant/media mutation, publish/unpublish, archive,
  and retained exports.
- Require idempotency keys for mutation and export commands.
- Require provider version/concurrency tokens where providers expose them.
- Return `denied`, `unavailable`, `unsupported`, `conflict`,
  `quota_exceeded`, `stale_data`, or `failure` before provider calls when
  preconditions fail.
- Enforce resource budgets for search fan-out, facets, pagination, media
  metadata, export size, provider quota, retained artifacts, and snapshots.

## Trace, Audit, Health, Snapshot, And Replay

Required event families:

- `catalog_pack_declared`
- `catalog_pack_admission_validated`
- `catalog_pack_policy_decision`
- `catalog_pack_provider_inspected`
- `catalog_pack_service_call_requested`
- `catalog_pack_service_call_succeeded`
- `catalog_pack_service_call_failed`
- `catalog_pack_mutation_planned`
- `catalog_pack_publication_requested`
- `catalog_pack_unavailable`
- `catalog_pack_snapshot_recorded`

Events include pack id, command name, trace id, application/session/task/tenant
identifiers, product/variant handles, store/channel/locale/currency context,
policy decision, provider class, descriptor hash, latency, freshness, bounded
resource counters, result code, and sanitized artifact references. Events must
exclude credentials, raw provider payloads, unpublished secrets, full media
bytes, provider-specific search DSL, and unbounded catalog exports.

Snapshots include descriptor version, provider health, command availability,
schema/version support, search/export support, policy-template hash, redaction
profile, freshness, resource counters, and replay pointers.

## SDK And Developer Documentation

SDK discovery must return pack metadata, lifecycle, service mapping, command
schemas, permission scopes, policy templates, examples, availability, health,
provider class, compatibility, diagnostics, and documentation links.

The required developer guide at `docs/developer-packs/commerce/catalog.md` must
cover:

- Manifest declaration and permission scopes.
- Provider/schema discovery and unsupported/degraded diagnostics.
- DTO reference for products, variants, attributes, options, prices,
  availability, taxonomy, search, mutation plans, exports, and artifacts.
- Examples for searching, reading product projections, resolving price,
  checking availability, planning product mutation, publishing, exporting, and
  handling stale or unsupported results.
- Provider replacement, mock/unavailable provider behavior, trace/audit
  interpretation, redaction guarantees, and boundaries with cart/order/payment.

## Design Patterns

- **Facade**: SDK exposes discovery and command builders while hiding providers.
- **Command**: every operation is a typed command/result DTO.
- **Strategy**: Shopify-like, Stripe-like, Square-like, commercetools-like, and
  other catalog providers adapt behind one service contract.
- **Decorator**: trace, policy, entitlement, resource, approval, metering, and
  redaction wrap every service call.
- **State**: product lifecycle, publication, provider health, async search, and
  export jobs use explicit states.
- **Specification**: admission validates declarations, scopes, provider schema,
  concurrency tokens, lifecycle transitions, and resource limits.
- **Observer**: trace, audit, provider, publication, and snapshot events are
  subscribable.
- **Memento**: effective capability reports, mutation plans, publication
  evidence, and artifact handles are replayable bounded records.
- **Abstract Factory**: providers register only through approved runtime-host or
  plugin composition roots.

## Risks And Mitigations

- Risk: catalog becomes cart/order/payment logic. Mitigation: price and
  availability are read views; cart/order/payment state changes live in separate
  packs.
- Risk: provider search DSL leaks into app contracts. Mitigation: typed
  `CatalogSearchRequest` supports portable filters/facets and returns
  unsupported diagnostics.
- Risk: mutation overwrites provider data. Mitigation: planning commands,
  idempotency, provider version tokens, approval, and conflict results run before
  side effects.
- Risk: large catalog exports leak data. Mitigation: exports return artifact
  handles with retention, checksum, redaction, resource budgets, and replay
  pointers.
