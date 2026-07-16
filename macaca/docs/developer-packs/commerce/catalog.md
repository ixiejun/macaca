# Commerce Catalog Pack

`pack.commerce.catalog.v1` describes provider-neutral product catalog,
variant, price, availability, taxonomy, search, mutation-plan, publication, and
export capabilities. The descriptor is discoverable through SDK catalogs, but
commands remain unavailable until a catalog provider is installed through the
runtime composition root.

## Manifest Declaration

Declare the pack as required only when catalog access is mandatory for
application readiness. Optional declarations degrade with structured
unavailable diagnostics.

```toml
[service_contract]
optional_packs = ["pack.commerce.catalog.v1"]
```

## Permissions

Use the narrowest scope: `commerce.catalog.read`,
`commerce.catalog.search`, `commerce.catalog.price`,
`commerce.catalog.availability`, `commerce.catalog.write`,
`commerce.catalog.publish`, and `commerce.catalog.export`.

## Capability Model

Macaca models catalog data as tenant, store, channel, locale, currency, product
and variant references, attributes, options, modifiers, price books, price
contexts, availability snapshots, taxonomy nodes, publication scopes, channels,
portable search requests, mutation plans, mutation results, freshness,
attribution, redaction policies, and artifact handles. Raw provider payloads,
provider search DSLs, unpublished secrets, media bytes, credentials, and
unbounded catalog exports stay behind provider adapters.

## Commands And Results

`catalog.inspect_provider`, `catalog.describe_schema`,
`catalog.list_products`, `catalog.get_product`, `catalog.list_variants`,
`catalog.get_variant`, `catalog.search_catalog`, `catalog.list_taxonomy`,
`catalog.get_price`, `catalog.check_availability`,
`catalog.plan_product_mutation`, `catalog.product_request`,
`catalog.plan_variant_mutation`, `catalog.variant_request`,
`catalog.plan_media_mutation`, `catalog.media_request`,
`catalog.plan_export`, `catalog.export_catalog`, and
`catalog.get_artifact_handle` are descriptor-owned schema names.

Every command uses a `CommerceCommandEnvelope`. Results use
`CatalogResultEnvelope<T>` with success, paged, partial, denied, unavailable,
unsupported, conflict, quota-exceeded, stale-data, approval-required,
version-conflict, export-accepted, and failure states.

## App-Facing Examples

- Inspect provider classes before declaring catalog features as required.
- Search with portable filters and handle unsupported-filter diagnostics.
- Resolve prices and availability through references rather than provider
  payloads.
- Plan product, variant, media, publish, and export mutations before requesting
  side effects.
- Read product projections, variant projections, taxonomy refs, price refs, and
  availability snapshots through bounded handles.
- Publish and export only after mutation plans, entitlement checks, attribution
  requirements, and artifact retention are represented in the command envelope.

## Trace And Audit

Traces should record pack id, command name, product or variant refs, store and
channel refs, descriptor hash, provider class, freshness class, result status,
idempotency hash, artifact id, and redaction profile. They must not record raw
provider payloads, unpublished secrets, media bytes, credentials, provider DSLs,
or unbounded exports.

## Boundaries

Catalog does not mutate carts, create orders, execute checkout, process
payments, issue receipts, provision entitlements, adjust inventory, or render
storefront UI. Those capabilities require separate pack declarations.
