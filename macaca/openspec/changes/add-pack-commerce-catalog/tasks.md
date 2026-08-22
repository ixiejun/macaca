## 1. Research, Scope, And Governance

- [x] 1.1 Re-read architecture governance, microkernel boundaries, serviceization allowlist, design-pattern guidance, umbrella catalog proposal, and this child proposal before implementation.
- [x] 1.2 Record supplier/API findings for Shopify, Stripe Products/Prices, Square Catalog/Inventory, commercetools, BigCommerce-style storefront APIs, and similar commerce catalog providers.
- [x] 1.3 Confirm the pack scope: schema discovery, products, variants, SKUs, attributes, options, modifiers, price books, price contexts, availability snapshots, taxonomy, publication scopes, media references, search, mutation planning, export, freshness, attribution, and redaction.
- [x] 1.4 Explicitly exclude cart mutation, order creation, checkout, payment, receipt generation, entitlement provisioning, fulfillment, shipment, inventory adjustment, storefront UI, tax calculation, promotions, and application-specific merchandising workflows.
- [x] 1.5 Inventory existing descriptors, SDK clients, service runtime hooks, policy gates, approval gates, resource gates, entitlement gates, trace/audit helpers, artifact handles, mock providers, and unavailable providers that can be reused.
- [x] 1.6 Record GitNexus CRITICAL/HIGH findings as memo-only evidence before implementation commits.

## 2. Contract, Descriptor, And DTO Schema

- [x] 2.1 Define descriptor metadata for `pack.commerce.catalog.v1`, including family, lifecycle, stability, command schemas, scopes, policy template, resource budgets, redaction profile, SDK metadata, docs link, compatibility, health, and diagnostics.
- [x] 2.2 Define `CatalogScope`, `CatalogProviderCapability`, `CatalogFreshness`, `CatalogAttribution`, and `CatalogRedactionPolicy`.
- [x] 2.3 Define `CatalogProduct`, lifecycle/publication state, localized content, product type, vendor/brand, taxonomy, media references, and provider version token.
- [x] 2.4 Define `CatalogVariant`, SKU, barcode, option values, purchasable flag, inventory tracking flag, shipping/customs metadata, media references, default price references, and provider version token.
- [x] 2.5 Define `CatalogAttribute`, `CatalogOption`, `CatalogModifier`, validation rules, localization, and provider support flags.
- [x] 2.6 Define `CatalogPrice`, `PriceBook`, `PriceContext`, recurring terms, customer group, channel, country, tax-inclusion flag, effective dates, and provider price reference.
- [x] 2.7 Define `AvailabilitySnapshot`, location/channel availability, freshness, provider attribution, and inventory-service handoff metadata.
- [x] 2.8 Define `CatalogTaxonomyNode`, `CatalogPublicationScope`, `CatalogChannel`, categories, collections, stores, channels, and product selections.
- [x] 2.9 Define `CatalogSearchRequest`, `CatalogSearchResult`, `CatalogProjection`, filters, facets, sort, localization, price context, availability context, scores, and unsupported filter diagnostics.
- [x] 2.10 Define `CatalogMutationPlan`, `CatalogMutationResult`, `CatalogArtifactHandle`, export format, checksum, expiry, retention, redaction, and replay metadata.
- [x] 2.11 Define typed `success`, `partial`, `denied`, `unavailable`, `unsupported`, `conflict`, `quota_exceeded`, `stale_data`, and `failure` result envelopes for every command family.
- [x] 2.12 Add descriptor hashing and compatibility tests for valid descriptors, rejected invalid descriptors, stable hashes, and schema evolution.

## 3. Command Surface And Catalog Semantics

- [x] 3.1 Implement command schemas for `catalog.inspect_provider` and `catalog.describe_schema`.
- [x] 3.2 Implement command schemas for `catalog.list_products`, `catalog.get_product`, `catalog.list_variants`, and `catalog.get_variant`.
- [x] 3.3 Implement command schemas for `catalog.search_catalog`, portable filters, facets, sort, projection, localization, price context, and availability context.
- [x] 3.4 Implement command schemas for `catalog.list_taxonomy`, categories, collections, stores, channels, and publication scopes.
- [x] 3.5 Implement command schemas for `catalog.get_price` and price projection under `PriceContext`.
- [x] 3.6 Implement command schemas for `catalog.check_availability` as a read-only availability snapshot.
- [x] 3.7 Implement command schemas for `catalog.plan_product_mutation`, `catalog.product_request`, `catalog.plan_variant_mutation`, and `catalog.variant_request`.
- [x] 3.8 Implement command schemas for `catalog.plan_media_mutation`, `catalog.media_request`, `catalog.plan_export`, `catalog.export_catalog`, and `catalog.get_artifact_handle`.
- [x] 3.9 Add validation for provider schema support, required fields, localization, version tokens, idempotency, publication state, unsupported filters, pagination, async jobs, export bounds, and stale-data conditions.

## 4. Permission, Policy, Resource, Entitlement, And Approval

- [x] 4.1 Add declaration validation for `commerce.catalog.read`, `commerce.catalog.search`, `commerce.catalog.price`, `commerce.catalog.availability`, `commerce.catalog.write`, `commerce.catalog.publish`, and `commerce.catalog.export`.
- [x] 4.2 Require policy decisions before every command and approval before product/variant/media mutation, publish/unpublish, archive, and retained exports.
- [x] 4.3 Require entitlement checks for provider access, search support, price support, availability support, write support, publish support, export support, and store/channel access.
- [x] 4.4 Reserve and meter resources for search fan-out, facets, pagination, media metadata, export size, provider quotas, storage, and snapshots.
- [x] 4.5 Return typed denied/unavailable/unsupported/conflict/quota/stale-data outcomes before provider calls when preconditions fail.
- [x] 4.6 Add tests proving denied, unavailable, unsupported, conflict, quota, and stale-data paths do not call concrete providers when preconditions fail.

## 5. Service Provider, Provider Strategy, And Unavailable Behavior

- [x] 5.1 Add the catalog service provider interface with descriptor, lifecycle, health, snapshot, shutdown, timeout, cancellation, async search/export support, and command dispatch.
- [x] 5.2 Implement provider Strategy adapters behind the service interface without provider-name routing in OS-layer command logic.
- [x] 5.3 Implement a mock provider with synthetic products, variants, price books, availability snapshots, taxonomy, search facets, mutation plans, stale-data states, and configurable capability gaps.
- [x] 5.4 Implement an unavailable provider that returns explicit unavailable diagnostics for every command without fake success.
- [x] 5.5 Normalize provider errors into Macaca result envelopes while preserving sanitized provider class, bounded code, retriable flag, freshness, and replay pointer.
- [x] 5.6 Add provider capability discovery for product model, variant model, price model, availability model, localization, taxonomy, publication, batch/export, search/facet, mutation, limits, freshness, and attribution.

## 6. SDK, Admission, Examples, And Developer Documentation

- [x] 6.1 Extend pack catalog and SDK discovery for `pack.commerce.catalog.v1` with schemas, scopes, examples, availability, health, diagnostics, compatibility, provider class, and docs metadata.
- [x] 6.2 Extend application admission so required declarations block on unavailable/denied states and optional declarations degrade explicitly with effective capability mementos.
- [x] 6.3 Add SDK command helper builders that only construct canonical traced service calls and never construct providers.
- [x] 6.4 Add generic app-facing examples for searching catalog, reading product projection, resolving price, checking availability, planning mutation, publishing, exporting, and handling unsupported filters.
- [x] 6.5 Create `docs/developer-packs/commerce/catalog.md` with purpose, manifest declaration, scopes, commands, DTOs, examples, unavailable diagnostics, provider replacement, trace/audit behavior, redaction, schema discovery, search semantics, mutation planning, and boundaries with cart/order/payment.
- [x] 6.6 Cross-link the developer guide from SDK discovery metadata and the industrial pack catalog index.

## 7. Trace, Audit, Replay, And Redaction

- [x] 7.1 Emit sanitized declaration, admission, provider-inspection, policy, entitlement, approval, resource, service-call, mutation-planning, publication, unavailable, health, snapshot, and result events.
- [x] 7.2 Add trace schemas for `catalog_pack_declared`, `catalog_pack_admission_validated`, `catalog_pack_policy_decision`, `catalog_pack_provider_inspected`, `catalog_pack_service_call_requested`, `catalog_pack_service_call_succeeded`, `catalog_pack_service_call_failed`, `catalog_pack_mutation_planned`, `catalog_pack_publication_requested`, `catalog_pack_unavailable`, and `catalog_pack_snapshot_recorded`.
- [x] 7.3 Add replay tests proving every command is trace-addressable through the canonical service runtime path.
- [x] 7.4 Add snapshot tests proving descriptor, provider health, command availability, schema/version support, search/export support, policy-template hash, redaction profile, resource counters, freshness, and replay pointers are retained without raw payload leakage.
- [x] 7.5 Add redaction tests proving credentials, raw provider payloads, unpublished secrets, full media bytes, provider-specific search DSLs, and unbounded catalog exports never enter logs, traces, snapshots, or SDK diagnostics.

## 8. Boundary, Quality, And Validation Gates

- [x] 8.1 Add dependency gates proving kernel, SDK, shells, and generic application framework do not import concrete catalog providers.
- [x] 8.2 Add no-direct-provider-call tests proving all callable operations traverse descriptor-owned service registration and typed service commands.
- [x] 8.3 Add canonical execution-path tests covering read-only, search, price, availability, planning, mutation, publication, export, denied, unavailable, unsupported, conflict, quota, and stale-data paths.
- [x] 8.4 Add provider replacement tests for built-in, plugin, remote, mock, and unavailable providers.
- [x] 8.5 Add file-size and module-ownership checks for any new implementation files.
- [x] 8.6 Run `openspec validate add-pack-commerce-catalog --strict`.
- [x] 8.7 Run targeted cargo checks/tests, dependency-boundary gates, audit replay checks, redaction checks, and catalog/cart/order boundary checks before marking implementation tasks complete.
