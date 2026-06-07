## Context

The previous evolution changes created a service-owned control plane, admission
gates, normalized benchmarks, and release safety chain. Those commands carry
evidence refs, but durable governance remains a future Store/EventLog concern.
This change introduces the provider boundary and a local JSONL development
provider so the control plane has a replayable ledger contract without making
local files the long-term architecture.

## Goals

- Define a Store/EventLog Strategy boundary for evolution governance records.
- Keep records versioned, bounded, sanitized, and replayable.
- Support restart replay, malformed-record skip, schema migration, compaction,
  concurrent append ordering, and cross-node replay cursors.
- Keep Skill, Autonomy, Web, CLI, and SDK semantics independent from the local
  JSONL provider.

## Non-Goals

- This change does not implement a production distributed Store/EventLog
  backend.
- This change does not add Web/CLI/frontend diagnostics.
- This change does not persist raw target state, raw manifests, package bytes,
  raw prompts, provider payloads, private keys, credentials, or raw signatures.

## Decisions

- **Strategy:** `EvolutionGovernanceLedger` is the replaceable interface. The
  local JSONL provider implements it for development and tests.
- **Memento:** replay cursors, compaction refs, and record refs are stable,
  bounded mementos.
- **Command:** append/replay/compact/snapshot inputs are typed DTOs even when
  used directly by providers in this slice.
- **Observer:** records represent observed control-plane events without owning
  target mutation semantics.
- **Specification:** schema version and sanitization checks are executable in
  provider code before records are written or exposed.

## Risks And Mitigations

- **Risk:** Local JSONL becomes treated as production storage.
  **Mitigation:** Name and document it as a development provider and keep all
  consumers on the `EvolutionGovernanceLedger` trait.
- **Risk:** Ledger snapshots leak raw payloads.
  **Mitigation:** append sanitization truncates refs and scrubs forbidden
  tokens; snapshot output exposes only sanitized records and counters.
- **Risk:** Compaction loses replay ordering.
  **Mitigation:** compact writes ordered retained records to a replacement file
  and preserves sequence numbers.

## Verification

- Service tests cover restart replay, malformed skip, schema migration,
  concurrent append ordering, compaction, and sanitized snapshots.
- `openspec validate serviceize-evolution-governance-ledger --strict`,
  targeted Rust tests, `git diff --check`, file-size checks, and GitNexus
  detect-changes run before commit.
