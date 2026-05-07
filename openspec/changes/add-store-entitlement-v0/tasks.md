## 1. Preparation

- [x] 1.1 Read `macaca/docs/agent-os-microkernel-boundaries.md`, `macaca/docs/route-c-regression-matrix.md`, `macaca/docs/route-c-phase-template.md`, `macaca/docs/route-c-architecture-governance.md`, and `docs/superpowers/plans/route-c-microkernel-ecosystem/phase-08-store-entitlement-v0.md`.
- [x] 1.2 Review current package/runtime guard and plugin runtime contracts in `macaca-proto`, `macaca-persist`, `macaca-runtime-host`, `macaca-skill`, and `macaca-app`.
- [x] 1.3 Run GitNexus impact before modifying each selected symbol; warn before editing any HIGH or CRITICAL impact symbol.

## 2. Commerce Protocol Contracts

- [x] 2.1 Add `macaca/crates/macaca-proto/src/commerce.rs` with provider-neutral commerce and entitlement contracts.
- [x] 2.2 Export commerce contracts from `macaca/crates/macaca-proto/src/lib.rs`.
- [x] 2.3 Define `LicenseType`, `EntitlementId`, `EntitlementState`, `CommerceMetadata`, `EntitlementDecision`, `SubscriptionPlan`, `MeteringEvent`, and `CommerceError`.
- [x] 2.4 Support free/open/paid/subscription/metered categories and unknown/custom values without hardcoding provider names.
- [x] 2.5 Add serde roundtrip tests for free/open/paid/subscription/metered fixtures and unknown license values.

## 3. Entitlement Persistence Contract

- [x] 3.1 Add `macaca/crates/macaca-persist/src/entitlement_store.rs` with entitlement persistence traits and in-memory/test adapter.
- [x] 3.2 Export entitlement store contract from `macaca-persist`.
- [x] 3.3 Implement deterministic precedence rules where `revoked` overrides `valid`.
- [x] 3.4 Add tests for upsert/query/revocation precedence and audit record append behavior.

## 4. Runtime Entitlement Guard Facade

- [x] 4.1 Add `macaca/crates/macaca-runtime-host/src/entitlement.rs` with `EntitlementRuntimeFacade`, validator/specification rules, and decision pipeline components.
- [x] 4.2 Export entitlement facade from `macaca-runtime-host`.
- [x] 4.3 Implement guard operations for `authorize_install`, `authorize_start`, and `authorize_capability_call`.
- [x] 4.4 Implement structured unavailable/deny states for missing, expired, revoked, region_blocked, usage_exceeded, and unknown_offline cases.
- [x] 4.5 Add structured logs for entitlement validation start/pass/reject, decision source, and metering emission.
- [x] 4.6 Add detailed English comments for public contracts, decision pipeline rules, and non-goals.

## 5. Encrypted Skill Hook Integration

- [x] 5.1 Add `macaca/crates/macaca-skill/src/encrypted_package.rs` with encrypted package detection and decrypt hook abstraction.
- [x] 5.2 Integrate entitlement authorization before decrypt/load for encrypted packages.
- [x] 5.3 Return structured errors for entitlement deny and decrypt failures; do not panic/hang.
- [x] 5.4 Add tests proving encrypted package load is denied without entitlement and enters decrypt hook with valid entitlement.

## 6. Commercial Package Guard Integration

- [x] 6.1 Add `macaca/crates/macaca-app/src/commercial_package.rs` for app-level commercial package guard wiring.
- [x] 6.2 Keep free/open package paths runnable without Store requirement.
- [x] 6.3 Integrate paid package install/start checks through `EntitlementRuntimeFacade` only.
- [x] 6.4 Mark bypassed direct entitlement decision paths as deprecated where canonical facade replaces them.

## 7. Metering and Audit Events

- [x] 7.1 Add metering event emission for paid capability calls using existing trace/event infrastructure.
- [x] 7.2 Ensure event payload includes app/package/developer/session/capability, operation, decision, status, and timestamp.
- [x] 7.3 Add tests proving metering events are emitted and trace/audit compatible.
- [x] 7.4 Run hardcode scan over new Store/Entitlement files for app/workflow/provider/driver/gateway/model/chain/business constants.

## 8. Regression and Verification

- [x] 8.1 Run `openspec validate add-store-entitlement-v0 --strict`.
- [x] 8.2 Run `cargo test -p macaca-proto commerce`.
- [x] 8.3 Run `cargo test -p macaca-persist entitlement`.
- [x] 8.4 Run `cargo test -p macaca-runtime-host entitlement`.
- [x] 8.5 Run `cargo test -p macaca-skill encrypted_package`.
- [x] 8.6 Run `cargo test -p macaca-integration-tests --test route_c_baseline`.
- [x] 8.7 Run `cargo check --workspace`.
- [x] 8.8 Run `npx gitnexus detect-changes --repo agent` before committing and verify affected flows align with Phase 08 scope.
