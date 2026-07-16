# Commerce Entitlement Pack

`pack.commerce.entitlement.v1` describes provider-neutral entitlement grants,
checks, batch checks, source sync, state transitions, suspension, resume,
revocation, transfer, seat assignment, usage metering, event references, proof
export, and artifact handles. The descriptor is discoverable through SDK
catalogs, but commands remain unavailable until an entitlement provider is
installed through the runtime composition root.

## Manifest Declaration

```toml
[service_contract]
optional_packs = ["pack.commerce.entitlement.v1"]
```

## Permissions

Use the narrowest scope: `commerce.entitlement.read`,
`commerce.entitlement.grant`, `commerce.entitlement.revoke`,
`commerce.entitlement.suspend`, `commerce.entitlement.transfer`,
`commerce.entitlement.seat`, `commerce.entitlement.meter`, and
`commerce.entitlement.proof_export`.

## Capability Model

Macaca models entitlements as subject references, resource references,
dimensions, source evidence, grants, state mappings, validity windows, usage
balances, seat assignments, usage records, event references, proof export
plans, freshness, attribution, redaction policies, and artifact handles. Raw
purchase tokens, app-store signed payloads, payment credentials, webhooks,
license secrets, private keys, signatures, provider payloads, and unbounded
exports stay behind provider adapters.

## Commands And Results

`entitlement.inspect_provider`, `entitlement.describe_schema`,
`entitlement.plan_grant`, `entitlement.grant`, `entitlement.check`,
`entitlement.batch_check`, `entitlement.sync_source`,
`entitlement.plan_suspend`, `entitlement.suspend`,
`entitlement.plan_resume`, `entitlement.resume`,
`entitlement.plan_revoke`, `entitlement.revoke`,
`entitlement.plan_transfer`, `entitlement.transfer`,
`entitlement.assign_seat`, `entitlement.release_seat`,
`entitlement.record_usage`, `entitlement.get_usage_balance`,
`entitlement.record_event_reference`, `entitlement.plan_proof_export`,
`entitlement.proof_export_request`, and
`entitlement.get_artifact_handle` are descriptor-owned schema names.

Every command uses a `CommerceCommandEnvelope`. Results use
`EntitlementResultEnvelope<T>` with success, paged, partial, accepted, denied,
unavailable, unsupported, conflict, quota-exceeded, stale-data,
approval-required, source-authority-denied, proof-redacted, and failure states.

## App-Facing Examples

- Check or batch-check subject/resource access with bounded result sets.
- Plan grants, revocations, transfers, seat changes, usage records, and proof
  exports before mutating provider state.
- Use source evidence references instead of storing raw app-store, billing, or
  webhook payloads.
- Sync source evidence, suspend and resume grants, read usage balances, and
  record usage through idempotent command envelopes.
- Handle conflicts, source-authority denial, stale source data, seat
  exhaustion, quota failures, proof redaction, unsupported transfer, and
  unavailable-provider diagnostics as typed results.

## Trace And Audit

Traces should record subject refs, resource refs, grant refs, state, source
evidence refs, usage dimension, event refs, proof artifact ids, provider class,
descriptor hash, freshness class, result status, idempotency hash, and
redaction profile. They must not record purchase tokens, signed payloads,
payment credentials, license secrets, provider webhook bodies, signatures, or
unbounded proof exports.

## Boundaries

Entitlement does not execute payments, subscription billing, refunds, invoices,
receipts, settlement, pricing changes, checkout flows, tax filing, or
application-specific feature gating logic.
