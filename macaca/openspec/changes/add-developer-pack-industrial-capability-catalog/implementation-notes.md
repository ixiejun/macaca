# Implementation Notes

## GitNexus Memo

- `DomainPackDefinition` upstream impact: LOW. Directly affects finance catalog construction and proto tests.
- `expand_service_capabilities` upstream impact: CRITICAL. Direct callers include application runtime bootstrap, service projection, WASM policy sync, SDK resolution, web UI context, and integration tests. Per user instruction, this warning was recorded as memo-only and did not block the implementation.
- `reference_domain_pack_definitions` and `DomainPackDefinitionSpec` were not found by the GitNexus index under those exact targets; static search and targeted tests were used to cover the affected paths.

## Implementation Evidence

- Added provider-neutral descriptor fields for availability, result schema refs, source attribution, migration notes, and descriptor callability.
- Added executable specification validation for callable descriptors, including service-to-command and service-to-result schema mappings.
- Added deterministic catalog snapshots with stable descriptor hashes and replay schema refs.
- Added the 74 required industrial sub-pack descriptors as `PreviewUnavailable` catalog entries. These entries are discoverable but never expand into callable service capabilities until a serviceized optional package or plugin overrides them with available descriptors.
- Extended effective capability mementos with command/result schema maps and unavailable reasons.
- Extended SDK invocation helpers to reject undeclared commands before building canonical traced `ServiceCallCommand` values.
- Kept base kernel, SDK, shells, and runtime-host free of optional provider imports and business-domain routing branches. The finance package remains an optional-module provider.

## Verification

- `cargo test -p macaca-proto domain_pack_contract --lib`
- `cargo test -p macaca-sdk domain_pack_client --lib`
- `cargo test -p macaca-domain-pack-finance`
- `openspec validate add-developer-pack-industrial-capability-catalog --strict`
- Strict validation loop over all 74 `add-pack-*` child proposals.
- File-size gate for edited domain-pack files: all checked files are below the 500-line project limit.

## Residual Notes

- `macaca-proto` still emits pre-existing unused import warnings in unrelated modules during targeted tests.
- The 74 child proposals are implementation-grade OpenSpec plans and currently mark their runtime scope as preview/unavailable unless a concrete provider is implemented by that child proposal.
