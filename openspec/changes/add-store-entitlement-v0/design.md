# Design: Store / Entitlement Runtime v0

## Context

Route C Phase 07 established plugin manifest/lifecycle boundaries. Phase 08 now needs a commerce and entitlement layer that can gate package installation, runtime activation, encrypted skill loading, and paid capability calls without coupling Macaca OS to a concrete Store provider or payment network.

This phase must preserve existing YAML app execution and skill/MCP baseline behavior while introducing auditable allow/deny decisions and metering events. The implementation must remain additive and forward-compatible with later A2A payment and optional Web3/EVM modules.

## Goals

- Define provider-neutral commerce contracts in `macaca-proto`.
- Add entitlement persistence contract in `macaca-persist`.
- Add runtime-host entitlement guard facade for install/start/call checks.
- Add encrypted skill loading authorization hook in `macaca-skill`.
- Add app commercial package runtime guard integration in `macaca-app`.
- Emit structured metering events into existing trace/audit/event flows.
- Preserve `RC-APP-001`, `RC-SKILL-001`, and `RC-TRACE-001`.

## Non-Goals

- No real payment processing, subscription settlement, or external billing provider integration.
- No cryptographic DRM claims beyond structured hooks and policy decisions.
- No forced migration of all legacy package paths in one step.
- No kernel-owned commerce business logic.

## Superpowers Brainstorm Summary

### Problem

Paid capability governance is currently fragmented and cannot guarantee that install/start/call paths are consistently protected by entitlement policy and traceable metering.

### Why This Phase

Phase 09 A2A payment and future store ecosystem depend on deterministic entitlement semantics and metering trace contracts defined in Phase 08.

### Options Considered

1. **Central entitlement facade + contracts (recommended)**
   - Pros: clear policy boundary, auditable decisions, provider-neutral, easy to extend.
   - Cons: requires additive wiring across several crates.
2. **Embed entitlement checks in each consumer module**
   - Pros: fast local patches.
   - Cons: duplicated logic, drift, bypass risk, poor audit consistency.
3. **Kernel-owned entitlement logic**
   - Pros: central authority.
   - Cons: violates microkernel boundaries; entitlement rules are replaceable service behavior.

### Recommended

Use Facade + Strategy + Specification + Chain of Responsibility with additive decorators around package/runtime/capability call paths.

## Architecture Decisions

### 1. Protocol Contracts (`macaca-proto/src/commerce.rs`)

Introduce typed contracts:

- `LicenseType`, `EntitlementId`, `SubscriptionPlanId`, `MeteringEventId`
- `EntitlementState` (`valid`, `expired`, `missing`, `revoked`, `region_blocked`, `usage_exceeded`, `unknown_offline`)
- `CommerceMetadata` (license/store/developer/signature/plan/metering/offline grace/revocation fields)
- `EntitlementDecision` and structured `CommerceError`

Pattern: **Value Object + Specification Inputs**

### 2. Entitlement Store Contract (`macaca-persist/src/entitlement_store.rs`)

Define trait-based persistence facade:

- upsert entitlement snapshot
- query by entitlement/package/developer keys
- deterministic precedence resolution
- append decision/metering audit record

Pattern: **Repository + Strategy**

### 3. Runtime Entitlement Facade (`macaca-runtime-host/src/entitlement.rs`)

Single entry point for runtime checks:

- `authorize_install`
- `authorize_start`
- `authorize_capability_call`
- `record_metering`

Decision pipeline:

`signature metadata check -> entitlement state check -> policy check -> optional metering decorator`

Pattern: **Facade + Chain of Responsibility + Decorator**

### 4. Encrypted Skill Hook (`macaca-skill/src/encrypted_package.rs`)

Add structured hook boundary:

- detect encrypted package metadata
- require entitlement authorization prior to decrypt
- abstract decrypt interface (no fake security claims)
- return structured deny/decrypt-failed errors

Pattern: **Proxy + Adapter**

### 5. Commercial Package Guard (`macaca-app/src/commercial_package.rs`)

Integrate entitlement decisions into app package install/start path while preserving free/open flows.

Pattern: **Guard Facade**

### 6. Trace & Audit

Every deny/allow/metering action emits structured logs and events:

- entitlement id
- package id/version
- developer id
- app id/session id when available
- operation (`install`, `start`, `call`, `decrypt`, `meter`)
- decision state
- error/status code
- timestamp

Pattern: **Observer**

## Data and State Rules

- `revoked` MUST override `valid`.
- `usage_exceeded` MUST deny paid capability call unless explicit grace policy allows.
- `unknown_offline` MAY allow temporary start only if offline grace metadata exists and has not expired.
- Free/open license types MUST remain runnable without store requirement.

## Compatibility and Regression

- Keep legacy YAML app boot path available (`RC-APP-001`).
- Keep skill/MCP smoke path available, adding only additive entitlement hooks where required (`RC-SKILL-001`).
- Preserve existing trace infra compatibility; metering and entitlement records are additive (`RC-TRACE-001`).

## Risk and Mitigation

- **Risk:** accidental deny of free/open packages.
  - Mitigation: explicit free/open fast-path tests.
- **Risk:** entitlement bypass in alternate call path.
  - Mitigation: central facade integration and deprecated direct path markers where canonical guard replaces old route.
- **Risk:** over-coupling to one store provider.
  - Mitigation: provider-neutral contracts; no provider constants in control flow.
- **Risk:** noisy logs with sensitive data.
  - Mitigation: structured bounded fields only; no credentials/secrets/private keys/raw encrypted payloads.

## Verification Plan

- `cargo test -p macaca-proto commerce`
- `cargo test -p macaca-persist entitlement`
- `cargo test -p macaca-runtime-host entitlement`
- `cargo test -p macaca-skill encrypted_package`
- `cargo test -p macaca-integration-tests --test route_c_baseline`
- `cargo check --workspace`
- `npx gitnexus detect-changes --repo agent`
