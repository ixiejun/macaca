# Change: Serviceize Evolution Governance Ledger

## Why

The autonomy evolution control plane now has lifecycle, admission, benchmark,
and release safety commands, but its durable governance source of truth is still
only represented by bounded refs. Complete unattended self-evolution needs a
replaceable Store/EventLog ledger contract that can persist sanitized evolution
records, replay them after restart, compact old records, migrate schema versions,
and support future cross-node replay without binding the OS to local JSONL.

## What Changes

- Add provider-neutral evolution governance ledger DTOs for append, replay,
  snapshot, compaction, and migration diagnostics.
- Add a replaceable ledger Strategy trait and a local JSONL development
  provider that is explicitly not the production Store/EventLog backend.
- Ensure ledger records contain only bounded refs and sanitized metadata, never
  raw prompts, provider payloads, manifests, package bytes, credentials, or
  unbounded output.
- Add tests for replay after provider restart, malformed record skip,
  schema-version migration, concurrent append ordering, compaction, and
  sanitized snapshots.

## Impact

- Affected specs: `autonomy-evolution-control-plane`
- Affected code:
  - `macaca/crates/services/macaca-autonomy-evolution`
  - targeted ledger tests under the same crate
