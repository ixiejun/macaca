# Change: Add Skill Proposal Processing Lane

## Why

The live self-evolution monitoring report proves that Macaca creates governed
Draft Skill experience proposals from real agent execution, but it also proves
that the system has no service-owned post-capture pressure. Draft proposals keep
growing without quality scoring, duplicate suppression, lifecycle processing,
materialization eligibility, activation evidence, or optimization metrics.

## What Changes

- Add a provider-neutral Skill proposal processing lane behind `service.skill`.
- Add deterministic quality scoring and duplicate grouping for proposal records.
- Add processing states that can suppress low-information duplicates or mark a
  proposal ready for a future materialization gate.
- Add read-only processing snapshots and backlog counters for operations
  surfaces.
- Keep this slice non-materializing: no `SKILL.md`, `_meta.json`, `_usage.json`,
  package bytes, executable scripts, or active catalog files are written.
- Keep Web, CLI, frontend, and applications as thin adapters over SDK or service
  commands.

## Impact

- Affected specs: `skill-governance-curation`
- Affected code:
  - `macaca/crates/services/macaca-skill/src/*`
  - `macaca/crates/runtime/macaca-runtime-host/src/*skill*`
  - `macaca/crates/facade/macaca-sdk/src/skill_client*`
  - `macaca/crates/shells/macaca-web/src/skill_operations_routes.rs`
- Architecture constraints:
  - Kernel remains provider-neutral and does not own Skill proposal processing.
  - Runtime-host owns the built-in processing Strategy only.
  - Skill service owns state, commands, policy envelopes, and snapshots.
  - Web and CLI do not compute quality, duplicates, suppression, or readiness.
