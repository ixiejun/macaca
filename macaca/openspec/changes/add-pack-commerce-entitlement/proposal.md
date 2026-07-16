# Change: Add Commerce Entitlement Pack

## Why

Macaca applications need `pack.commerce.entitlement.v1` as an industrial
capability for granting, checking, metering, suspending, revoking, syncing, and
exporting proof of access rights. Entitlement systems sit between commerce
events and application access decisions: they may be derived from subscriptions,
licenses, purchases, seats, usage credits, receipts, order records, organization
contracts, or manual administrative grants, but they must not execute payments,
refunds, invoices, or application-specific feature logic.

This proposal defines entitlement as a serviceized, provider-neutral pack. It
lets applications and other packs reason about access evidence through typed
commands while keeping provider adapters, billing systems, app feature gates,
license stores, subscription systems, and unavailable behavior behind replaceable
service providers.

## Supplier And API Baseline

The design is based on mature entitlement, subscription, licensing, and store
APIs:

- Stripe Entitlements exposes Features and active entitlements tied to customer
  products/subscriptions, including lookup, feature metadata, and lifecycle
  events.
- RevenueCat models entitlements as app-defined access rights backed by
  offerings, products, subscriptions, purchases, expiration, renewal, billing
  issue, and store provenance.
- Apple App Store Server APIs expose transaction history, subscription status,
  renewal state, refunds, revocations, and signed transaction data used to derive
  entitlement state.
- Google Play Developer and Billing APIs expose product purchases,
  subscriptions, purchase tokens, acknowledgement/consumption, expiry, auto
  renewal, cancellation, and linked purchase tokens that drive access rights.
- Microsoft Store collection/subscription APIs expose product ownership,
  consumables, subscriptions, renewals, expirations, and durable add-on
  entitlements for Microsoft accounts.
- Paddle and similar SaaS billing providers expose subscriptions, products,
  customers, transactions, adjustments, webhooks, usage/metering, and access
  evidence that must be normalized separately from billing execution.

The common denominator is an entitlement grant with subject, scope, product or
feature reference, source evidence, status, validity window, quantity or seat
allocation, usage balance, sync freshness, revocation/suspension reason,
provider attribution, and auditable proof.

Research references:

- Stripe Entitlements: https://docs.stripe.com/billing/entitlements and
  https://docs.stripe.com/api/entitlements
- RevenueCat Entitlements:
  https://www.revenuecat.com/docs/getting-started/entitlements
- Apple App Store Server API:
  https://developer.apple.com/documentation/appstoreserverapi
- Google Play Developer API subscriptions and purchases:
  https://developers.google.com/android-publisher/api-ref/rest and
  https://developer.android.com/google/play/billing
- Microsoft Store product ownership and subscriptions:
  https://learn.microsoft.com/windows/uwp/monetize/
- Paddle subscriptions and events:
  https://developer.paddle.com/api-reference/subscriptions/overview and
  https://developer.paddle.com/webhooks/overview

## Macaca Provider-Neutral Mapping

`pack.commerce.entitlement.v1` maps supplier concepts into stable Macaca
contracts:

- Provider features, durable add-ons, product access, subscription access,
  license assignments, app-store purchases, and organization seats become
  `EntitlementGrant`.
- Provider customers, users, accounts, organizations, tenants, devices, and app
  installations become `EntitlementSubject` references.
- Provider products, prices, SKUs, offerings, plans, features, seats, credits,
  and usage dimensions become `EntitlementResource` and
  `EntitlementDimension`.
- App-store purchase tokens, signed transactions, receipts, order references,
  payment references, invoices, and manual grants become
  `EntitlementSourceEvidence`.
- Active, trialing, grace-period, paused, suspended, expired, revoked, refunded,
  transferred, and consumed states become `EntitlementState`.
- Usage balances, metered dimensions, seats, consumables, and quotas become
  `EntitlementMeter` and `EntitlementUsageRecord`.
- Provider events and sync signals become `EntitlementEventReference` records
  with bounded metadata and replay pointers.

## What Changes

- Add provider-neutral `pack.commerce.entitlement.v1` under the commerce family.
- Define commands for provider inspection, schema discovery, grant planning,
  grant, check, batch check, sync source, suspend/resume, revoke, transfer, seat
  assignment, usage metering, usage balance inspection, proof export, event
  reference recording, and artifact retrieval.
- Define DTOs for entitlement scope, subject, resource, dimension, source
  evidence, grant record, state, validity, seat assignment, usage balance,
  metering record, revocation/suspension reason, sync freshness, attribution,
  proof artifacts, and redaction.
- Require policy, entitlement, approval for irreversible access changes,
  idempotency, source-evidence validation, subject/resource isolation, usage
  bounds, sanitized trace/audit, and deterministic unavailable/unsupported
  behavior.
- Require detailed developer documentation at
  `docs/developer-packs/commerce/entitlement.md`.

## Impact

- Affected specs: `pack-commerce-entitlement`,
  `developer-pack-industrial-capability-catalog`, `sdk-system-facade`,
  `service-runtime`, `unified-execution-path`.
- Affected code later: protocol DTOs, pack descriptors, admission validators,
  SDK discovery/command builders, entitlement service providers,
  mock/unavailable providers, trace/audit schemas, replay tests, redaction tests,
  and boundary gates.

## Non-Goals

- No payment authorization/capture, subscription billing execution, refund
  execution, invoice generation, receipt issuance, tax filing, settlement,
  payout, dispute handling, or application checkout UI.
- No application-specific feature gating rules, product-specific access policy,
  provider-name routing, pricing logic, subscription upgrade/downgrade business
  logic, or merchant contract interpretation in Macaca OS layers.
- No raw app-store signed payloads, purchase tokens, payment credentials,
  provider webhook bodies, license secrets, private keys, signatures, raw
  provider payloads, or unbounded exports in logs, traces, snapshots, or SDK
  diagnostics.
- No provider construction or provider-name routing in kernel, SDK, shells, or
  generic application framework.
