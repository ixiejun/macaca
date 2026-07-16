# Commerce Entitlement Pack Design

## Context

`pack.commerce.entitlement.v1` is Macaca's provider-neutral access-rights
capability. It owns entitlement grants, status checks, source synchronization,
seat assignment, usage metering, suspension/resume, revocation, transfer,
proof export, and entitlement audit evidence. It does not own billing,
payments, refunds, invoices, receipts, tax, settlement, app-store validation
internals, or application-specific feature gating logic.

Entitlement providers differ substantially. Some expose product features and
active entitlements, some derive access from app-store transactions, some expose
license collections, and some require subscription events to be folded into a
local access ledger. Macaca normalizes the entitlement evidence boundary while
leaving provider-specific billing and application behavior outside OS layers.

## Supplier Capability Matrix

| Supplier family | Relevant capabilities | Constraints Macaca must model |
| --- | --- | --- |
| Stripe Entitlements | Features, active entitlements, customer/product linkage, subscription lifecycle events | Feature lookup vs product catalog, active-state freshness, webhook-derived updates, billing separate from grant checks |
| RevenueCat | Entitlements, offerings, products, app-store purchases, expiration, grace period, billing issues, store provenance | App-defined entitlement identifiers, multi-store sources, grace/billing issue states, customer identity mapping |
| Apple App Store Server | Signed transactions, transaction history, subscription status, renewal info, refunds, revocations | Signed payload redaction, transaction freshness, revocation/refund state, app/account scoping |
| Google Play Billing/Developer APIs | Product/subscription purchases, purchase tokens, acknowledgement/consumption, expiry, cancellation, linked tokens | Token sensitivity, consumables vs subscriptions, acknowledgement state, linked purchase migration |
| Microsoft Store | Product ownership, durable add-ons, subscriptions, consumables, renewals, expirations | User/account collection scope, durable vs consumable distinction, subscription renewal state |
| Paddle/SaaS billing | Subscriptions, products, customers, transactions, adjustments, webhooks, usage/metering | Billing events as source evidence, usage dimensions, cancellation/paused states, provider webhook authority |

## Goals

- Provide provider inspection, schema discovery, grant planning, grant,
  entitlement check, batch check, source sync, suspension/resume, revocation,
  transfer, seat assignment, usage metering, usage balance inspection, proof
  export, event reference recording, and artifact retrieval.
- Preserve subject/resource scoping, source evidence, validity windows, trial and
  grace states, revocation/suspension reasons, usage balances, seat quantities,
  idempotency, event freshness, and proof evidence.
- Keep billing, payment, refund, invoice, receipt, settlement, pricing,
  provider-specific subscription management, and application feature gating as
  separate boundaries.
- Route every command through the canonical service runtime path with trace,
  policy, entitlement, resource, approval when required, health, snapshot, and
  structured errors.

## Non-Goals

- Payment authorization/capture, subscription billing execution, invoice
  generation, receipt issuance, refund execution, settlement, payout, tax
  filing, dispute handling, app-store purchase UI, or checkout UI.
- Application-specific feature flags, product-specific authorization logic,
  merchant contract interpretation, pricing/package upgrade rules, or provider
  routing in OS layers.
- Raw app-store signed payloads, purchase tokens, payment credentials, provider
  webhook bodies, license secrets, private keys, raw signatures, or unbounded
  entitlement exports in observability.

## Ownership And Boundaries

- Pack id: `pack.commerce.entitlement.v1`.
- Family: `commerce`.
- Backing service owner: entitlement service provider family.
- SDK surface: `sdk.packs.commerce.entitlement`.
- Command namespace: `entitlement.*`.
- Kernel ownership: identity, service-call evidence, policy facade, trace/audit
  primitives, and resource primitives only.
- Application framework ownership: manifest declaration, app-scoped permission
  declarations, lifecycle projection, and effective-capability mementos.
- Runtime-host ownership: provider registration, decorators, mock/unavailable
  providers, and adapter composition through approved composition roots.
- Service ownership: capability discovery, grant state validation, source
  evidence normalization, provider Strategy dispatch, usage metering,
  redaction, and sanitized audit.

## Command Surface

| Command | Purpose | Side-effect class |
| --- | --- | --- |
| `entitlement.inspect_provider` | Return provider support for sources, states, usage, seats, transfer, proof export, freshness, and attribution | Read-only |
| `entitlement.describe_schema` | Return subject, resource, grant, source, state, usage, seat, proof, event, and artifact schema | Read-only |
| `entitlement.plan_grant` | Validate subject, resource, source evidence, state, validity, quantity, approval, and provider constraints | Planning |
| `entitlement.grant` | Create or update entitlement grant through approved side-effect path | Mutating |
| `entitlement.check` | Check one entitlement for a subject/resource/dimension at a point in time | Read-only |
| `entitlement.batch_check` | Check multiple entitlements with bounded result set and shared freshness policy | Read-only |
| `entitlement.sync_source` | Refresh source evidence from subscription, purchase, receipt, order, license, or manual source | Read-only or provider sync |
| `entitlement.plan_suspend` | Validate suspension reason, state, source authority, and approval requirement | Planning |
| `entitlement.suspend` | Suspend an entitlement without deleting proof history | Mutating |
| `entitlement.plan_resume` | Validate resume eligibility and source authority | Planning |
| `entitlement.resume` | Resume a suspended entitlement when provider/source state allows | Mutating |
| `entitlement.plan_revoke` | Validate revocation reason, irreversible effect, source authority, and approval | Planning |
| `entitlement.revoke` | Revoke or expire a grant through approved path | Mutating |
| `entitlement.plan_transfer` | Validate transfer between subjects, devices, organizations, or seats | Planning |
| `entitlement.transfer` | Transfer entitlement ownership or assignment when provider supports it | Mutating |
| `entitlement.assign_seat` | Assign a seat or named license to a subject | Mutating |
| `entitlement.release_seat` | Release a seat or named license from a subject | Mutating |
| `entitlement.record_usage` | Meter usage against a dimension, quantity, idempotency key, and source evidence | Mutating |
| `entitlement.get_usage_balance` | Read metered balance, limit, reset window, and freshness | Read-only |
| `entitlement.record_event_reference` | Normalize provider event metadata without raw webhook body | Mutating metadata |
| `entitlement.plan_proof_export` | Plan proof export scope, format, redaction, retention, and artifact bounds | Planning |
| `entitlement.proof_export_request` | Produce entitlement proof artifact handle | Mutating/export |
| `entitlement.get_artifact_handle` | Retrieve proof artifact metadata without raw payload leakage | Read-only |

Every command must define typed command DTOs, success DTOs, partial/async shapes,
denied/unavailable/unsupported/conflict/quota/stale-data/failure results,
idempotency for side effects, redaction policy, and replay metadata.

## Provider-Neutral DTO Model

- `EntitlementScope`: application, tenant, session, task, provider scope,
  merchant/store/channel, entitlement handle, subject reference, resource
  reference, and permission scope.
- `EntitlementSubject`: account, profile, organization, tenant, device,
  installation, agent, service account, or external customer reference with
  redaction metadata.
- `EntitlementResource`: product, SKU, feature, plan, offering, license,
  subscription, seat pool, usage credit, content item, capability, or external
  resource reference.
- `EntitlementDimension`: seat, request, token, storage, time window, durable
  ownership, subscription period, consumable credit, or provider custom
  dimension.
- `EntitlementSourceEvidence`: order, payment, receipt, invoice, subscription,
  app-store transaction, purchase token, license, manual grant, support
  override, provider event, and migration references with source authority and
  redaction.
- `EntitlementGrant`: handle, subject, resource, dimensions, state, validity
  window, quantity, usage balance, source evidence, grant reason, suspension or
  revocation reason, transfer history, freshness, attribution, and redaction.
- `EntitlementState`: planned, active, trial, grace, pending_payment,
  pending_acknowledgement, paused, suspended, expired, revoked, refunded,
  transferred, consumed, unknown, and provider custom states with mapping
  metadata.
- `EntitlementSeatAssignment`: seat pool, assignee reference, quantity, role,
  assignment state, effective time, release state, and audit evidence.
- `EntitlementUsageRecord` and `EntitlementUsageBalance`: dimension, quantity,
  unit, idempotency key, usage window, reset policy, balance, limit, source
  evidence, freshness, and conflict metadata.
- `EntitlementEventReference`: provider class, event type, event timestamp,
  delivery id hash, webhook freshness, replay pointer, and bounded result code.
- `EntitlementProofExportPlan`, `EntitlementArtifactHandle`: export scope,
  proof type, checksum, expiry, retention, redaction profile, access policy, and
  replay pointer.

## Permission, Policy, Resource, Entitlement, And Approval

Initial scopes:

- `commerce.entitlement.read`
- `commerce.entitlement.grant`
- `commerce.entitlement.revoke`
- `commerce.entitlement.suspend`
- `commerce.entitlement.transfer`
- `commerce.entitlement.seat`
- `commerce.entitlement.meter`
- `commerce.entitlement.proof_export`

Policy defaults:

- Scope every call to application id, tenant id, session id, task id, trace id,
  provider scope, subject, resource, entitlement handle, and source evidence.
- Require approval for manual grants, suspensions, revocations, transfers, seat
  assignment changes, usage corrections, and retained proof exports.
- Require idempotency keys for mutating commands, usage recording, and export
  requests.
- Validate subject/resource isolation, source authority, state transitions,
  validity windows, usage dimensions, seat quantity, transfer eligibility, and
  freshness before provider calls when detectable.
- Return typed `denied`, `unavailable`, `unsupported`, `conflict`,
  `quota_exceeded`, `stale_data`, or `failure` before provider calls when
  preconditions fail.
- Enforce resource budgets for batch checks, source sync, usage recording, proof
  export size, provider quotas, storage, and snapshots.

## Trace, Audit, Health, Snapshot, And Replay

Required event families:

- `entitlement_pack_declared`
- `entitlement_pack_admission_validated`
- `entitlement_pack_policy_decision`
- `entitlement_pack_provider_inspected`
- `entitlement_pack_service_call_requested`
- `entitlement_pack_service_call_succeeded`
- `entitlement_pack_service_call_failed`
- `entitlement_pack_grant_planned`
- `entitlement_pack_state_transition_planned`
- `entitlement_pack_usage_recorded`
- `entitlement_pack_proof_export_planned`
- `entitlement_pack_unavailable`
- `entitlement_pack_snapshot_recorded`

Events include pack id, command name, trace id, application/session/task/tenant
identifiers, subject/resource handles, entitlement state, state transition,
usage dimension, policy decision, provider class, descriptor hash, latency,
freshness, idempotency hash, bounded resource counters, result code, and
sanitized artifact references. Events must exclude raw purchase tokens,
app-store signed payloads, payment credentials, provider webhook bodies, license
secrets, private keys, raw signatures, raw provider payloads, and unbounded
exports.

Snapshots include descriptor version, provider health, command availability,
source/state/usage/seat/transfer/proof support, policy-template hash, redaction
profile, freshness, resource counters, and replay pointers.

## SDK And Developer Documentation

SDK discovery must return pack metadata, lifecycle, service mapping, command
schemas, permission scopes, policy templates, examples, availability, health,
provider class, compatibility, diagnostics, and documentation links.

The required developer guide at
`docs/developer-packs/commerce/entitlement.md` must cover:

- Manifest declaration and permission scopes.
- Provider/schema discovery and unavailable diagnostics.
- DTO reference for scopes, subjects, resources, dimensions, source evidence,
  grants, states, validity windows, seats, usage records, usage balances, event
  references, proof exports, and artifacts.
- Examples for planning/granting entitlements, checking access, batch checks,
  syncing sources, suspending/resuming, revoking, transferring, assigning seats,
  recording usage, reading balances, exporting proof, and handling conflicts.
- Provider replacement, mock/unavailable provider behavior, trace/audit
  interpretation, redaction guarantees, idempotency, source freshness, and
  boundaries with payment intent, order, invoice, receipt, subscription billing,
  identity, workflow approval, and application feature gating.

## Design Patterns

- **Facade**: SDK exposes discovery and command builders while hiding providers.
- **Command**: every entitlement operation is a typed command/result DTO.
- **Strategy**: Stripe-like, RevenueCat-like, Apple-like, Google-like,
  Microsoft-like, Paddle-like, and other providers adapt behind one contract.
- **Decorator**: trace, policy, entitlement, approval, resource, idempotency,
  metering, proof bounds, and redaction wrap every call.
- **State**: grant, active/trial/grace, suspension, revocation, transfer, seat,
  usage, proof-export, source-sync, and provider-health lifecycles are explicit.
- **Specification**: admission validates declarations, scopes, source
  authority, subject/resource isolation, state transitions, usage dimensions,
  seat limits, and resource bounds.
- **Observer**: trace, audit, provider, grant, usage, event-reference, proof, and
  snapshot events are subscribable.
- **Memento**: effective capability reports, grant evidence, source evidence,
  state transitions, usage records, event references, and proof handles are
  replayable bounded records.
- **Abstract Factory**: providers register only through approved runtime-host or
  plugin composition roots.

## Risks And Mitigations

- Risk: entitlement becomes app-specific feature authorization. Mitigation:
  entitlement returns provider-neutral evidence; applications decide their own
  feature behavior through declared capabilities and app code.
- Risk: entitlement becomes a billing/payment/subscription engine. Mitigation:
  billing, payment, refund, invoice, receipt, pricing, and checkout operations
  are explicitly unsupported and represented only as source references.
- Risk: source tokens or signed store payloads leak. Mitigation: DTOs carry
  handles/hashes and redaction metadata; redaction tests cover logs, traces,
  snapshots, and SDK diagnostics.
- Risk: stale entitlement state causes incorrect access. Mitigation: freshness,
  source authority, sync commands, stale-data errors, and replayable event
  references are first-class DTO fields.
- Risk: SDK helpers become a second execution path. Mitigation: helpers only
  build canonical service-call commands and no-direct-provider-call gates cover
  every command.
