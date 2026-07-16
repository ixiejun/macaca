# Commerce Entitlement Pack Research

## Purpose

This note records supplier/API research, Macaca provider-neutral mapping,
explicit non-goals, existing platform inventory, and GitNexus memo evidence for
`pack.commerce.entitlement.v1`. The entitlement pack must expose grants,
checks, batch checks, source sync, state transitions, suspension/resume,
revocation, transfer, seat assignment, usage metering, proof export, event
references, artifacts, freshness, attribution, and redaction through typed
service commands. It must not execute billing, payment, refund, invoice,
receipt, tax, settlement, or application-specific feature-gating workflows.

## Source Baseline

- Stripe Entitlements:
  <https://docs.stripe.com/billing/entitlements> and
  <https://docs.stripe.com/api/entitlements>
- RevenueCat Entitlements:
  <https://www.revenuecat.com/docs/getting-started/entitlements>
- Apple App Store Server API:
  <https://developer.apple.com/documentation/appstoreserverapi>
- Google Play Billing and Android Publisher APIs:
  <https://developer.android.com/google/play/billing> and
  <https://developers.google.com/android-publisher/api-ref/rest>
- Microsoft Store product ownership and subscriptions:
  <https://learn.microsoft.com/windows/uwp/monetize/>
- Paddle subscriptions and webhooks:
  <https://developer.paddle.com/api-reference/subscriptions/overview> and
  <https://developer.paddle.com/webhooks/overview>

## Supplier API Notes

- Stripe contributes features and active entitlements tied to customer products
  and subscriptions. Macaca should treat billing events as source evidence and
  keep subscription billing execution outside entitlement checks.
- RevenueCat contributes app-defined entitlements backed by offerings, products,
  subscriptions, purchases, expiration, grace/billing issue state, and store
  provenance. Macaca should normalize app-store source authority and freshness.
- Apple and Google contribute transaction histories, subscription status,
  signed or tokenized purchase evidence, revocation/refund state, expiry,
  acknowledgement, consumption, and linked purchases. Macaca should redact raw
  signed payloads and purchase tokens while preserving proof references.
- Microsoft Store contributes durable ownership, consumables, subscriptions,
  renewals, expiration, and account-scoped collection behavior. Macaca should
  model durable versus consumable rights and subject isolation.
- Paddle-style SaaS providers contribute subscription events, products,
  customers, transactions, adjustments, usage/metering, and webhooks. Macaca
  should treat webhook data as bounded event references, not raw payload storage.

## Macaca-Owned Abstractions

`pack.commerce.entitlement.v1` should define `EntitlementScope`,
`EntitlementProviderCapability`, `EntitlementSubject`,
`EntitlementResource`, `EntitlementDimension`,
`EntitlementSourceEvidence`, `EntitlementGrant`, `EntitlementState`,
`EntitlementSeatAssignment`, `EntitlementUsageRecord`,
`EntitlementUsageBalance`, `EntitlementEventReference`,
`EntitlementProofExportPlan`, `EntitlementArtifactHandle`,
`EntitlementFreshness`, `EntitlementAttribution`, and
`EntitlementRedactionPolicy`.

The DTOs must carry subject/resource isolation, source-evidence visibility,
validity windows, quantity, seat pools, usage dimensions, idempotency, source
authority, state-transition evidence, event freshness, capability hashes,
redaction classes, bounded provider reason codes, and replay pointers. Raw
signed transactions, purchase tokens, license secrets, payment credentials, raw
webhook bodies, private keys, signatures, and unbounded proof exports are
rejected.

## Explicit Non-Goals

- Do not implement concrete Stripe, RevenueCat, Apple, Google, Microsoft,
  Paddle, license-server, billing, app-store-validation, or webhook adapters in
  this research phase.
- Do not define payment authorization/capture, subscription billing execution,
  refund execution, invoice generation, receipt issuance, settlement, payouts,
  disputes, tax filing, application-specific feature gating, upgrade/downgrade
  business logic, or checkout UI semantics inside this pack.
- Do not expose provider-native entitlement payloads, raw store tokens, raw
  webhooks, app feature flags, merchant contract rules, pricing rules, or
  provider-specific billing workflows as stable SDK contracts.

## Existing Macaca Platform Inventory

- `crates/foundation/macaca-proto/src/domain_pack_contract/` already provides
  descriptor metadata, lifecycle/availability, policy templates, SDK metadata,
  diagnostics, provider snapshots, unavailable diagnostics, and effective
  capability expansion concepts that entitlement descriptors can reuse.
- `crates/facade/macaca-sdk/src/system_facade.rs` and focused SDK clients
  provide the Facade pattern expected for app-facing discovery and command
  construction; entitlement SDK helpers should only build canonical traced
  service calls.
- `crates/runtime/macaca-host-composition/src/runtime_host.rs` and
  `crates/kernel/macaca-kernel/src/domain_pack_registration.rs` provide generic
  provider registration/composition mechanics for optional domain-pack
  providers.
- Existing payment and entitlement-adjacent specs establish that Store,
  Entitlement, Payment, Web3, and EVM providers belong behind service/runtime
  boundaries, not kernel, SDK, or shell logic.
- Current evidence does not prove commerce-entitlement-specific DTOs,
  descriptors, command schemas, providers, SDK helpers, WASM ABI metadata, trace
  schemas, replay tests, redaction tests, dependency gates, or developer
  documentation.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
