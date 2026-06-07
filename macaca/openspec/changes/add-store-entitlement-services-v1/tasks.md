## 1. Preparation

- [x] 1.1 Read `macaca/docs/agent-os-microkernel-boundaries.md`, `macaca/docs/route-c-serviceization-allowlist.md`, `macaca/docs/route-c-architecture-governance.md`, `macaca/docs/route-c-regression-matrix.md`, `docs/superpowers/plans/2026-05-08-route-c-serviceize-and-modularize-non-kernel-capabilities.md`, and `docs/superpowers/plans/2026-05-10-s9-store-entitlement-serviceization-plan.md`.
- [x] 1.2 Review Phase 08 implementation in `macaca-proto`, `macaca-persist`, `macaca-runtime-host`, `macaca-app`, and `macaca-skill`.
- [x] 1.3 Run GitNexus impact before modifying every existing symbol; warn before editing HIGH or CRITICAL impact symbols.
- [x] 1.4 Confirm touched Rust files remain under 500 lines, splitting modules before adding large DTO/provider/client logic.

## 2. Store / Entitlement Proto DTOs

- [x] 2.1 Add `macaca/crates/macaca-proto/src/store_service.rs` with Store Service ids, command names, commands, results, sanitized views, unavailable state, and English comments.
- [x] 2.2 Add `macaca/crates/macaca-proto/src/entitlement_service.rs` with Entitlement Service ids, command names, commands, results, audit page, snapshot, unavailable state, and English comments.
- [x] 2.3 Export both modules from `macaca/crates/macaca-proto/src/lib.rs`.
- [x] 2.4 Add serde roundtrip and validation tests for service DTOs.
- [x] 2.5 Verify DTOs reuse existing commerce value objects and do not duplicate license/state semantics.

## 3. Runtime-Host Admission And Providers

- [x] 3.1 Add `macaca/crates/macaca-runtime-host/src/store_entitlement_admission.rs` with trace, package scope, operation, metering, redaction, and free/open fast-path Specifications.
- [x] 3.2 Add `macaca/crates/macaca-runtime-host/src/entitlement_service_provider.rs`.
- [x] 3.3 Implement entitlement query/upsert/revoke/authorize install/authorize start/authorize call/audit query/metering record/snapshot service commands.
- [x] 3.4 Add `macaca/crates/macaca-runtime-host/src/store_service_provider.rs`.
- [x] 3.5 Implement package inspect/resolve/install/status/snapshot service commands.
- [x] 3.6 Register provider exports in `macaca/crates/macaca-runtime-host/src/lib.rs`.
- [x] 3.7 Add structured logs for service start/stop/call/failure/allow/deny/metering/audit nodes without sensitive payloads.

## 4. SDK Clients

- [x] 4.1 Add `macaca/crates/macaca-sdk/src/store_client.rs` with `SystemStoreClient`, service-backed client, unavailable client, and English comments.
- [x] 4.2 Add `macaca/crates/macaca-sdk/src/entitlement_client.rs` with `SystemEntitlementClient`, service-backed client, unavailable client, and English comments.
- [x] 4.3 Update `macaca/crates/macaca-sdk/src/package_client.rs` so package inspection/install/status can delegate to Store Service when available.
- [x] 4.4 Export new clients from `macaca/crates/macaca-sdk/src/lib.rs`.
- [x] 4.5 Add `SystemFacade::store_client()` and `SystemFacade::entitlement_client()` accessors.
- [x] 4.6 Ensure SDK does not depend on runtime-host, app, skill, Web, CLI, or provider concrete implementations for Store/Entitlement behavior.

## 5. App And Skill Consumer Migration

- [x] 5.1 Add a service-backed application entitlement authorizer adapter while keeping `ApplicationEntitlementAuthorizer` as the dependency-inversion seam.
- [x] 5.2 Mark direct `EntitlementRuntimeFacade` app guard usage as deprecated where a service-backed authorizer is available.
- [x] 5.3 Add a service-backed encrypted package authorizer adapter while keeping `EncryptedPackageAuthorizer` as the dependency-inversion seam.
- [x] 5.4 Ensure encrypted package decrypt hooks never execute when paid/encrypted entitlement service authorization denies or is unavailable.
- [x] 5.5 Preserve free/open package behavior and existing Phase 08 tests.

## 6. Web / CLI Consumer Migration

- [x] 6.1 Register and start Store/Entitlement services during Web startup when `ServiceRuntime` is available.
- [x] 6.2 Migrate Web package inspect/status/install surfaces, if present, to `SystemStoreClient`.
- [x] 6.3 Migrate Web entitlement/audit surfaces, if present, to `SystemEntitlementClient`.
- [x] 6.4 Migrate CLI package inspect/install/status and entitlement inspection commands, if present, to `SystemFacade` clients.
- [x] 6.5 Keep old direct fallbacks as deprecated compatibility anchors until S12 thin shell can remove them.

## 7. Governance

- [x] 7.1 Add S9 Store / Entitlement Service Ownership section to `macaca/docs/route-c-architecture-governance.md`.
- [x] 7.2 Update `macaca/docs/route-c-serviceization-allowlist.md` with S9 migration status and remaining debt.
- [x] 7.3 Update dependency boundary allowlist tests only if direct dependency edges change.
- [x] 7.4 Run hardcode scans over new Store/Entitlement code for app/workflow/provider/driver/gateway/model/chain/business constants.

## 8. Verification

- [x] 8.1 Run `openspec validate add-store-entitlement-services-v1 --strict`.
- [x] 8.2 Run `cargo fmt --all --check`.
- [x] 8.3 Run `cargo test -p macaca-proto store_service`, `cargo test -p macaca-proto entitlement_service`, and `cargo test -p macaca-proto commerce`.
- [x] 8.4 Run `cargo test -p macaca-persist entitlement`.
- [x] 8.5 Run `cargo test -p macaca-runtime-host entitlement` and `cargo test -p macaca-runtime-host service_runtime`.
- [x] 8.6 Run `cargo test -p macaca-app commercial_package`.
- [x] 8.7 Run `cargo test -p macaca-skill encrypted_package`.
- [x] 8.8 Run `cargo test -p macaca-sdk store_client` and `cargo test -p macaca-sdk entitlement_client`.
- [x] 8.9 Run `cargo test -p macaca-integration-tests route_c_dependency_boundaries`.
- [x] 8.10 Run `cargo test -p macaca-integration-tests --test route_c_baseline`.
- [x] 8.11 Run `cargo check --workspace`.
- [x] 8.12 Run GitNexus detect changes before commit.
