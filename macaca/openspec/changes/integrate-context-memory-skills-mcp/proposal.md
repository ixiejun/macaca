# Proposal: integrate-context-memory-skills-mcp (umbrella)

## Why

The 2026-05-05 Superpowers integration plan enumerated context composition, profiles, proactive vector recall, skills/MCP capability context, governance digesting, façade wiring, and an external-validation boundary.

Implementation landed incrementally via multiple focused OpenSpec changes. This umbrella aggregates **verification requirements** plus the remaining **glue** (fingerprints, profile hardening, recall tombstones, merged tombstone snapshots, inventories) without rewinding prior merges.

## What changes

Add this change directory with **`integration-acceptance` spec deltas** documenting cross-cutting MUST statements.

Companion code deltas close known gaps noted in [`docs/superpowers/plans/2026-05-06-complete-2026-05-05-context-integration-phases-brainstorm-and-plan.md`](../../../docs/superpowers/plans/2026-05-06-complete-2026-05-05-context-integration-phases-brainstorm-and-plan.md).

## Risks / mitigations

- **Fingerprints persisted in reports:** extra bytes on `context_report`; keep SHA-256 hex only.
- **`profile_max_content_lines`:** defaults to disabled (`0`) to avoid surprising truncation.
- **Tombstone fail-open:** operator warnings + OpenSpec Scenario documents behavior.
