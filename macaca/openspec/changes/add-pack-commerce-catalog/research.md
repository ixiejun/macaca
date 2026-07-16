# Commerce Catalog Pack Research

## Purpose

This note records supplier/API research, Macaca provider-neutral mapping,
explicit non-goals, existing platform inventory, and GitNexus memo evidence for
`pack.commerce.catalog.v1`. The catalog pack must expose products, variants,
SKUs, attributes, options, price contexts, availability snapshots, taxonomy,
media references, search, mutation planning, export, freshness, attribution, and
redaction through typed service commands. It must not own carts, orders,
checkout, payment, receipt, entitlement, fulfillment, shipment, inventory
adjustment, or application-specific merchandising workflows.

## Source Baseline

- Shopify Admin products, variants, price lists, and inventory:
  <https://shopify.dev/docs/api/admin-graphql/latest/objects/Product>,
  <https://shopify.dev/docs/api/admin-graphql/latest/objects/ProductVariant>,
  <https://shopify.dev/docs/api/admin-graphql/latest/objects/PriceList>, and
  <https://shopify.dev/docs/api/admin-graphql/latest/objects/InventoryItem>
- Stripe Products and Prices:
  <https://docs.stripe.com/api/products> and
  <https://docs.stripe.com/api/prices>
- Square Catalog and Inventory APIs:
  <https://developer.squareup.com/reference/square/catalog-api> and
  <https://developer.squareup.com/reference/square/inventory-api>
- commercetools Products, Product Projections, Product Search, Standalone
  Prices, Stores, Channels, and Inventory:
  <https://docs.commercetools.com/api/projects/products>,
  <https://docs.commercetools.com/api/projects/productProjections>,
  <https://docs.commercetools.com/api/projects/product-search>,
  <https://docs.commercetools.com/api/projects/standalone-prices>, and
  <https://docs.commercetools.com/api/projects/inventory>
- BigCommerce Catalog APIs:
  <https://developer.bigcommerce.com/docs/rest-catalog>

## Supplier API Notes

- Shopify contributes products, variants, options, metafields, media, price
  lists, inventory-item separation, channel/publication concerns, and bulk-style
  catalog operations. Macaca should model variant/SKU and price-context
  separation instead of exposing Shopify object shapes.
- Stripe contributes product identity and price records with recurring or
  one-time billing semantics. Macaca should support billing-oriented catalog
  providers while making limited inventory/search capability explicit.
- Square contributes catalog items, item variations, categories, modifiers,
  taxes, discounts, images, locations, and inventory objects. Macaca should keep
  catalog metadata separate from inventory mutation and promotion execution.
- commercetools contributes product types, variants, attributes, projections,
  localized search, standalone prices, stores, channels, and versioned updates.
  Macaca should preserve schema discovery, localized projection, price context,
  and version conflict behavior.
- BigCommerce contributes products, variants, options, modifiers, categories,
  custom fields, price lists, channel publication, and inventory summaries.
  Macaca should normalize storefront and management capability differences.

## Macaca-Owned Abstractions

`pack.commerce.catalog.v1` should define `CatalogScope`,
`CatalogProviderCapability`, `CatalogProduct`, `CatalogVariant`,
`CatalogAttribute`, `CatalogOption`, `CatalogModifier`, `CatalogPrice`,
`PriceBook`, `PriceContext`, `AvailabilitySnapshot`, `CatalogTaxonomyNode`,
`CatalogPublicationScope`, `CatalogChannel`, `CatalogSearchRequest`,
`CatalogSearchResult`, `CatalogProjection`, `CatalogMutationPlan`,
`CatalogArtifactHandle`, `CatalogFreshness`, `CatalogAttribution`, and
`CatalogRedactionPolicy`.

The DTOs must carry schema support, product/variant handles, localized content,
attribute metadata, price projection context, availability freshness, taxonomy
scope, media handles, pagination, unsupported filter diagnostics, version
tokens, idempotency, capability hashes, redaction classes, bounded provider
reason codes, and replay pointers. Provider search DSLs, unpublished secrets,
credentials, raw provider payloads, full media bytes, and unbounded catalog
exports are rejected.

## Explicit Non-Goals

- Do not implement concrete Shopify, Stripe, Square, commercetools,
  BigCommerce, search-engine, tax-engine, promotion-engine, media-store, or
  inventory adapters in this research phase.
- Do not define cart mutation, order creation, checkout, payment, receipt,
  entitlement, fulfillment, shipment, inventory adjustment, storefront UI, or
  application-specific merchandising semantics inside this pack.
- Do not expose provider-native product payloads, provider search DSLs,
  provider routing, raw media bytes, credentials, unpublished secrets, or
  application-specific price-selection policy as stable SDK contracts.

## Existing Macaca Platform Inventory

- `crates/foundation/macaca-proto/src/domain_pack_contract/` already provides
  descriptor metadata, lifecycle/availability, policy templates, SDK metadata,
  diagnostics, provider snapshots, unavailable diagnostics, and effective
  capability expansion concepts that catalog descriptors can reuse.
- `crates/facade/macaca-sdk/src/system_facade.rs` and focused SDK clients
  provide the Facade pattern expected for discovery and command construction;
  catalog SDK helpers should only build canonical traced service calls.
- `crates/runtime/macaca-host-composition/src/runtime_host.rs` and
  `crates/kernel/macaca-kernel/src/domain_pack_registration.rs` provide generic
  provider registration/composition mechanics for optional domain-pack
  providers.
- `crates/kernel/macaca-kernel/src/policy.rs`,
  `crates/runtime/macaca-runtime-host/src/service_policy_engine.rs`,
  `crates/kernel/macaca-kernel/src/audit.rs`,
  `crates/foundation/macaca-proto/src/audit_redaction.rs`, and
  `crates/runtime/macaca-runtime-host/src/service_call_audit.rs` provide
  reusable policy, redaction, trace, and audit substrate.
- Current evidence does not prove catalog-specific DTOs, descriptors, command
  schemas, providers, SDK helpers, WASM ABI metadata, trace schemas, replay
  tests, redaction tests, dependency gates, or developer documentation.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
