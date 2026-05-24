# Change: Improve materialized Skill semantic identity

## Why

Autonomously materialized Skills currently fall back to proposal-id-derived
names such as `skill-exp-<task-id>-<timestamp>` when a proposal does not carry a
target skill name. These identifiers are useful for audit, but they are poor
model-facing trigger names and make later Skill activation depend on explicit
ID mention rather than semantic task matching.

## What Changes

- Materialized Skill packages get a bounded semantic display name when no target
  skill name is already present.
- Proposal ids remain preserved as immutable provenance, audit, rollback, and
  governance identifiers, but they are not the preferred model-facing `name`.
- Generated `SKILL.md` content includes a more specific `When To Use` section
  derived from sanitized proposal summary/procedure evidence.
- Materialization logs expose bounded identity derivation facts without raw
  prompts, provider payloads, package bytes, or full generated Skill bodies.

## Impact

- Affected specs: `skill-governance-curation`
- Affected code:
  - `macaca/crates/runtime/macaca-runtime-host/src/skill_service_provider_proposal_materialization.rs`
  - `macaca/crates/runtime/macaca-runtime-host/src/skill_proposal_materialization_tests.rs`
- Design patterns:
  - Builder: isolates semantic identity and `SKILL.md` rendering.
  - Specification: validates bounded, provider-neutral identity derivation.
  - Observer: logs traceable, sanitized identity decisions.
