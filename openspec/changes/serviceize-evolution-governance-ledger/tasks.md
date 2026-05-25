## 1. OpenSpec

- [x] 1.1 Create proposal, design, tasks, and delta spec.
- [x] 1.2 Validate `serviceize-evolution-governance-ledger` with `--strict`.

## 2. Ledger Contract And Provider

- [x] 2.1 Add versioned governance ledger DTOs for records, append, replay,
  snapshot, and compaction.
- [x] 2.2 Add replaceable `EvolutionGovernanceLedger` Strategy trait.
- [x] 2.3 Add local JSONL development provider with sanitized append, replay,
  migration, compaction, and structured logs.

## 3. Verification

- [x] 3.1 Add tests for replay after restart, malformed record skip,
  schema-version migration, concurrent append ordering, compaction, and
  sanitized snapshots.
- [x] 3.2 Run targeted Rust tests.
- [x] 3.3 Run `openspec validate serviceize-evolution-governance-ledger --strict`.
- [x] 3.4 Run `git diff --check`.
