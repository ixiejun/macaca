# Change: Add Commerce Catalog Pack

## Why

Macaca applications need `pack.commerce.catalog.v1` as an industrial catalog
capability for products, variants, SKUs, price books, availability views,
categories, attributes, media, and storefront search. A usable commerce catalog
pack must normalize very different provider models: some platforms split
products and prices, some model variants and inventory by location, some expose
localized projections and channel-specific prices, and some treat catalog data as
the source for downstream carts and orders.

This proposal defines catalog as a provider-neutral, serviceized pack. It gives
applications typed commands for catalog discovery, read/search, controlled
mutation, price lookup, availability lookup, and export while keeping inventory
adjustment, cart, order, payment, receipt, and entitlement behavior in their own
packs.

## Supplier And API Baseline

The design is based on mature commerce catalog APIs:

- Shopify Admin APIs expose products, variants, options, metafields, images,
  inventory items, inventory levels, localized pricing/content, and price lists.
- Stripe Products and Prices separate sellable product identity from price
  records, recurring/one-time billing terms, lookup keys, and active/archived
  status.
- Square Catalog API models catalog items, item variations, categories, taxes,
  discounts, modifiers, and images; Square Inventory API owns inventory counts
  and changes separately.
- commercetools exposes products, product types, variants, attributes,
  categories, product projections, product search, product selections, standalone
  prices, channels, stores, localization, and inventory availability.
- BigCommerce and similar storefront platforms expose products, variants,
  options, modifiers, categories, price lists, inventory summaries, custom fields,
  media, and channel/storefront publication.

The common denominator is a versioned catalog of sellable or discoverable items,
variant/SKU records, normalized attributes, price projections, availability
snapshots, categories/collections, media references, publication status, and
provider capability metadata.

## Macaca Provider-Neutral Mapping

`pack.commerce.catalog.v1` maps supplier concepts into stable Macaca contracts:

- Provider products/items become `CatalogProduct`.
- Product variants, item variations, SKUs, and price-bearing purchasable units
  become `CatalogVariant`.
- Product options, metafields, custom attributes, product types, and modifiers
  become `CatalogAttribute`, `CatalogOption`, and `CatalogModifier`.
- Price objects, price lists, channel/country/customer-group projections, and
  recurring terms become `CatalogPrice`, `PriceBook`, and `PriceContext`.
- Inventory levels and availability APIs become read-only
  `AvailabilitySnapshot` records. Inventory adjustments/reservations live in
  order/cart/inventory-adjacent service behavior, not catalog mutation.
- Images, videos, PDFs, and manuals become `CatalogMediaReference` handles.
- Categories, collections, product selections, stores, and channels become
  `CatalogTaxonomyNode`, `CatalogPublicationScope`, and `CatalogChannel`.
- Search, facets, filters, localization, and projections become
  `CatalogSearchRequest`, `CatalogSearchResult`, and `CatalogProjection`.

## What Changes

- Add provider-neutral `pack.commerce.catalog.v1` under the commerce family.
- Define commands for provider inspection, schema discovery, product/variant
  read/list/search, taxonomy, media, price lookup, availability lookup, mutation
  planning, product/variant creation/update/archive, publish/unpublish,
  projection search, and export.
- Define DTOs for catalog scope, provider capability, products, variants,
  attributes, options, modifiers, price books, price contexts, availability,
  taxonomy, publication scopes, localization, media, freshness, attribution,
  redaction, and idempotency.
- Require policy, entitlement, resource bounds, publication approval, mutation
  planning, provider concurrency/version checks, and sanitized trace/audit
  evidence.
- Require detailed developer documentation at
  `docs/developer-packs/commerce/catalog.md`.

## Impact

- Affected specs: `pack-commerce-catalog`,
  `developer-pack-industrial-capability-catalog`, `sdk-system-facade`,
  `service-runtime`, `unified-execution-path`.
- Affected code later: protocol DTOs, pack descriptors, admission validators,
  SDK discovery/command builders, catalog service providers, mock/unavailable
  providers, trace/audit schemas, replay tests, redaction tests, and
  dependency-boundary gates.

## Non-Goals

- No cart mutation, order creation, checkout, payment, receipt generation,
  entitlement provisioning, fulfillment, shipment, inventory adjustment, or
  provider-specific storefront UI.
- No application-specific merchandising rules, price-selection business policy,
  catalog templates, tax calculation, or promotion logic in Macaca OS layers.
- No raw provider payloads, credentials, unpublished secrets, unbounded catalog
  exports, or provider-specific search DSLs in logs, traces, snapshots, or SDK
  diagnostics.
- No provider construction or provider-name routing in kernel, SDK, shells, or
  generic application framework.
