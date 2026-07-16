# Commerce Receipt Pack

`pack.commerce.receipt.v1` describes provider-neutral receipt evidence,
issue/reissue, read/search, source sync, verification, delivery state,
correction references, event references, audit export, and artifact handles.
The descriptor is discoverable through SDK catalogs, but commands remain
unavailable until a receipt provider is installed through the runtime
composition root.

## Manifest Declaration

```toml
[service_contract]
optional_packs = ["pack.commerce.receipt.v1"]
```

## Permissions

Use the narrowest scope: `commerce.receipt.read`,
`commerce.receipt.issue`, `commerce.receipt.reissue`,
`commerce.receipt.verify`, `commerce.receipt.deliver`,
`commerce.receipt.correction_reference`, and
`commerce.receipt.audit_export`.

## Capability Model

Macaca models receipt records as source references, receipt numbers, audiences,
variants, issue states, lines, adjustments, totals, delivery requests, delivery
states, verification results, correction references, event references,
freshness, attribution, redaction policies, and artifact handles. Buyer PII,
payment credentials, provider webhook bodies, receipt HTML, printable blobs,
signatures, private keys, raw provider payloads, and unbounded exports stay
behind provider adapters.

## Commands And Results

`receipt.inspect_provider`, `receipt.describe_schema`, `receipt.plan_issue`,
`receipt.issue_receipt`, `receipt.plan_reissue`,
`receipt.reissue_receipt`, `receipt.read_receipt`,
`receipt.search_receipts`, `receipt.sync_source`,
`receipt.verify_receipt`, `receipt.plan_delivery`,
`receipt.delivery_request`, `receipt.get_delivery_status`,
`receipt.link_correction_reference`,
`receipt.list_correction_references`,
`receipt.record_event_reference`, `receipt.plan_audit_export`,
`receipt.audit_export_request`, and `receipt.get_artifact_handle` are
descriptor-owned schema names.

Every command uses a `CommerceCommandEnvelope`. Results use
`ReceiptResultEnvelope<T>` with success, paged, partial, accepted, denied,
unavailable, unsupported, conflict, quota-exceeded, stale-data,
approval-required, verification-failed, artifact-redacted, and failure states.

## App-Facing Examples

- Plan issuing or reissuing receipts from declared source references.
- Verify receipt evidence and artifact checksums before showing trust state.
- Request delivery through reference-only destinations and check delivery
  status without owning communication workflow semantics.
- Read or search receipts with bounded cursors and redacted audience refs.
- Sync source metadata, link correction references, and request audit exports
  through explicit plans.
- Handle delivery conflicts, verification failures, artifact redaction,
  stale-data, quota, unsupported delivery, and unavailable-provider diagnostics
  as typed results.

## Trace And Audit

Traces should record receipt refs, source refs, audience, variant, delivery
channel, verification state, correction refs, event refs, descriptor hash,
provider class, freshness class, result status, idempotency hash, artifact id,
and redaction profile. They must not record buyer PII, payment credentials,
webhook bodies, receipt bodies, printable blobs, signatures, or unbounded
exports.

## Boundaries

Receipt does not authorize or capture payments, execute refunds, generate
invoices, reconcile settlement, provision entitlements, file taxes, fulfill
shipments, or own customer communication workflows.
