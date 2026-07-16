## ADDED Requirements

### Requirement: Macaca SHALL provide Commerce Catalog as a serviceized pack

Macaca SHALL provide `pack.commerce.catalog.v1` as a provider-neutral,
serviceized commerce pack for product catalog, variants, SKUs, attributes,
options, price books, availability snapshots, taxonomy, publication scopes,
media references, search, controlled mutation, export, and artifact handles.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.commerce.catalog.v1` as required and the catalog service provider is registered, healthy, entitled, permissioned, and policy-admissible
- **THEN** admission SHALL expose the pack in the effective capability set with descriptor hash, command schemas, permission scopes, policy template, provider capability, schema metadata, health, freshness, and replay metadata
- **AND** SDK discovery SHALL mark supported commands as callable without exposing credentials, raw provider payloads, unpublished secrets, provider-specific search DSLs, full media bytes, or unbounded catalog exports

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.commerce.catalog.v1` as required but provider, permission, entitlement, policy, resource, host support, or store/channel access is absent
- **THEN** admission SHALL block readiness with structured unavailable or denied diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back, call another provider, or fake success

#### Scenario: Optional declaration is unavailable
- **WHEN** an application declares `pack.commerce.catalog.v1` as optional and the pack is unavailable
- **THEN** admission SHALL produce an explicit degraded effective capability report
- **AND** SDK helpers SHALL refuse to build callable service calls for unavailable commands

### Requirement: Commerce Catalog SHALL expose provider and schema discovery

`pack.commerce.catalog.v1` SHALL expose provider-neutral discovery for product
model, variant model, attribute model, price model, availability model,
localization, taxonomy, publication, search/facet support, mutation support,
batch/export support, freshness, attribution, limits, and unavailable
limitations.

#### Scenario: Provider schema is inspected
- **WHEN** an application invokes `catalog.inspect_provider` or `catalog.describe_schema`
- **THEN** Macaca SHALL return `CatalogProviderCapability` and schema metadata with command support, product/variant/price/availability models, localization support, taxonomy support, publication support, search/facet support, mutation support, export formats, freshness, attribution, and limits
- **AND** the response SHALL use provider-neutral metadata rather than raw provider catalog payloads

#### Scenario: Provider does not support a filter
- **WHEN** an application invokes `catalog.search_catalog` with a portable filter or facet unsupported by the active provider
- **THEN** Macaca SHALL return a typed `unsupported` or degraded result identifying the unsupported filter category
- **AND** it SHALL NOT pass provider-specific search DSLs through SDK diagnostics, traces, or snapshots

### Requirement: Commerce Catalog commands SHALL use typed canonical service calls

Every Commerce Catalog operation SHALL be represented as a typed command and
result DTO, and every invocation SHALL traverse the canonical service runtime
path with trace, policy, resource, entitlement, approval when required, health,
snapshot, and structured error behavior.

#### Scenario: Search command succeeds through service runtime
- **WHEN** a declared and policy-allowed command such as `catalog.search_catalog` is invoked
- **THEN** Macaca SHALL route the command through SDK or facade helpers into the service runtime and catalog service provider
- **AND** it SHALL emit sanitized admission, policy, provider-inspection, service-call, result, and replay events with stable trace identifiers

#### Scenario: Mutation command is denied before provider call
- **WHEN** policy, permission, entitlement, approval, version-token, lifecycle, resource, or provider-capability checks reject `catalog.product_request` or `catalog.variant_request`
- **THEN** Macaca SHALL return a typed `denied`, `unavailable`, `unsupported`, `conflict`, `quota_exceeded`, or `stale_data` result before invoking the concrete provider
- **AND** audit evidence SHALL include bounded reason codes and replay pointers without raw provider payloads

#### Scenario: Export output is bounded
- **WHEN** `catalog.export_catalog` could return a large product, variant, media, or price export
- **THEN** Macaca SHALL produce a `CatalogArtifactHandle` or bounded metadata response
- **AND** traces and snapshots SHALL store only checksums, handles, expiry, retention, redaction, and sanitized metadata

### Requirement: Commerce Catalog SHALL normalize product, variant, price, and availability data

Commerce Catalog SHALL provide normalized DTOs for products, variants, attributes,
options, modifiers, prices, price books, price contexts, availability snapshots,
taxonomy, publication scopes, channels, media references, freshness, attribution,
and redaction.

#### Scenario: Product projection is read
- **WHEN** an application invokes `catalog.get_product` with authorized product and projection context
- **THEN** Macaca SHALL return `CatalogProduct` data with localized fields when requested, product type, taxonomy, attributes, media references, publication scopes, provider version token, freshness, attribution, and redaction metadata
- **AND** provider-specific missing fields SHALL be represented as explicit unavailable or unknown states rather than fabricated values

#### Scenario: Price is resolved
- **WHEN** an application invokes `catalog.get_price` with `PriceContext` such as currency, country, channel, customer group, and effective date
- **THEN** Macaca SHALL return a `CatalogPrice` or unsupported/degraded result with amount, currency, billing mode, recurring terms when applicable, price-book reference, tax-inclusion flag, effective dates, freshness, and attribution
- **AND** price lookup SHALL NOT create a cart, order, checkout session, or payment intent

#### Scenario: Availability is checked
- **WHEN** an application invokes `catalog.check_availability` for a variant, location, channel, or store context
- **THEN** Macaca SHALL return an `AvailabilitySnapshot` with availability status, freshness, provider attribution, and inventory-service handoff metadata
- **AND** the command SHALL NOT adjust inventory, reserve stock, create an order, or mutate fulfillment state

### Requirement: Commerce Catalog SHALL separate planning from mutations

Commerce Catalog SHALL provide plan-before-side-effect commands for product,
variant, media, publication, and export changes so applications can inspect
provider constraints, version tokens, and approval requirements before external
state changes.

#### Scenario: Product mutation is planned
- **WHEN** an application invokes `catalog.plan_product_mutation`
- **THEN** Macaca SHALL validate required fields, schema support, publication state, localization, version token, idempotency requirement, and approval requirement
- **AND** the planning command SHALL NOT mutate provider catalog state

#### Scenario: Product mutation is applied
- **WHEN** an application invokes `catalog.product_request` with an approved plan, valid idempotency key, current provider version token, and supported lifecycle transition
- **THEN** Macaca SHALL call the catalog provider through the service runtime and return `CatalogMutationResult`
- **AND** stale version tokens or unsupported transitions SHALL return typed `conflict` or `stale_data` results before side effects when detectable

#### Scenario: Publication is requested
- **WHEN** an application requests publish or unpublish behavior through a catalog mutation plan
- **THEN** Macaca SHALL require publication scope, approval, provider capability, and policy authorization
- **AND** publication traces SHALL contain product/variant handles and bounded state changes, not raw provider payloads

### Requirement: Commerce Catalog SHALL preserve Macaca boundaries

The Commerce Catalog implementation SHALL remain owned by the catalog service
provider family. The microkernel, SDK, shells, and generic application framework
SHALL remain provider-neutral and SHALL NOT contain concrete provider
construction, provider-name routing, cart/order/payment logic, promotion logic,
tax calculation, inventory adjustment, or application-specific merchandising
workflow.

#### Scenario: Boundary gates scan the implementation
- **WHEN** dependency, no-direct-provider-call, and canonical execution-path gates scan the implementation
- **THEN** they SHALL find no concrete catalog provider imports in the microkernel, SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned service registrations and typed service commands

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, mock, or unavailable catalog provider is selected
- **THEN** callers SHALL observe the same provider-neutral command/result contract
- **AND** trace/audit evidence SHALL identify only sanitized provider class, descriptor metadata, schema support, capability support, freshness, and bounded result codes

### Requirement: Commerce Catalog SHALL provide detailed developer documentation

The Commerce Catalog proposal SHALL require a detailed developer guide for
`pack.commerce.catalog.v1` that makes the pack usable by application developers
and provider implementers without relying on source-code inspection.

#### Scenario: Developer reads the pack guide
- **WHEN** a developer opens `docs/developer-packs/commerce/catalog.md`
- **THEN** the guide SHALL describe purpose, manifest declaration, permission scopes, provider/schema discovery, command DTOs, result DTOs, examples, unavailable diagnostics, provider replacement, trace/audit behavior, redaction guarantees, search semantics, mutation planning, and boundaries with cart/order/payment
- **AND** examples SHALL use generic handles and synthetic data instead of credentials, provider routing keys, application-specific merchandising workflows, real unpublished product data, or provider-specific search DSLs

#### Scenario: SDK discovery links documentation
- **WHEN** SDK discovery returns metadata for `pack.commerce.catalog.v1`
- **THEN** the metadata SHALL include the catalog developer-guide link and version compatibility information
- **AND** unavailable diagnostics SHALL point developers to the relevant declaration, permission, entitlement, provider, schema, search, policy, publication, freshness, or boundary remediation section
