# Change: Add Store / Entitlement Services v1

## Why

Route C S9 requires Store / Entitlement to become auditable, replaceable system services instead of runtime helper APIs. Phase 08 already introduced provider-neutral commerce contracts, entitlement persistence, runtime guard facades, app commercial guards, and encrypted skill hooks; S9 now needs to put those capabilities behind `ServiceRuntime` and `SystemFacade` so Web, CLI, Application, Skill, and future Gateway consumers stop owning commerce behavior.

Without a service boundary, paid package install/start/call decisions can keep fragmenting across app, skill, runtime-host, Web, and CLI code paths. That creates entitlement bypass risk, missing metering audit, and long-term coupling to concrete runtime helpers.

## What Changes

- Add provider-neutral Store Service DTOs for package inspect, resolve, install, status, and snapshot.
- Add provider-neutral Entitlement Service DTOs for query, upsert, revoke, install/start/call authorization, audit query, metering record, and snapshot.
- Add runtime-host Store and Entitlement service providers that adapt Phase 08 `EntitlementStore`, `EntitlementRuntimeFacade`, `CommercialPackageGuard`, encrypted package authorization, and package runtime guard seams.
- Add SDK focused clients: `SystemStoreClient`, `SystemEntitlementClient`, and a service-backed `SystemPackageClient`.
- Migrate Web/CLI package manager and entitlement-related consumers to service-first `SystemFacade` paths where those surfaces exist.
- Keep Phase 08 direct helper APIs as deprecated compatibility anchors until all consumers migrate and dependency gates prove they can be removed.
- Update Route C governance and allowlist docs with S9 ownership and remaining debt.

## Impact

- Affected specs: `store-service`, `entitlement-service`, `store-entitlement-sdk-client`, `store-entitlement-consumer-migration`
- Affected crates: `macaca-proto`, `macaca-runtime-host`, `macaca-sdk`, `macaca-app`, `macaca-skill`, `macaca-web`, `macaca-cli`, `macaca-integration-tests`
- Affected docs: `macaca/docs/route-c-architecture-governance.md`, `macaca/docs/route-c-serviceization-allowlist.md`
- Regression matrix: `RC-APP-001`, `RC-SKILL-001`, `RC-TRACE-001`

## Governance Alignment

- Store / Entitlement remains a System Service per `macaca/docs/agent-os-microkernel-boundaries.md`; kernel does not own commerce policy.
- The design follows `route-c-serviceization-allowlist.md`; new direct dependency exceptions require explicit migration debt and expiry conditions.
- The design follows `route-c-architecture-governance.md`; service calls are traced, policy-checkable, provider-neutral, and sanitized.
- S9 is additive-first: free/open package behavior, existing YAML apps, encrypted skill hook behavior, and Phase 08 entitlement guard tests must keep working.

## Non-Goals

- Do not implement payment provider settlement, A2A quote/intent/receipt, marketplace billing, or subscription charging. Those belong to S10.
- Do not implement Web3/EVM on-chain entitlement verification. That belongs to S11.
- Do not build marketplace recommendation, rating, or business-operation UI.
- Do not add a new `macaca-store` crate in this slice.
- Do not delete Phase 08 helper APIs during this change.
- Do not hardcode app names, workflow names, Store vendor names, payment provider names, driver names, gateway names, model names, chain names, or business-specific routing.
