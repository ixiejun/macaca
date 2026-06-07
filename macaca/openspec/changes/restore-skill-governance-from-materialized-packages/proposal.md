# Change: Restore Skill governance from materialized packages

## Why
Live restart verification shows that materialized Skill packages remain visible
through the registry/load path after backend restart, but the in-memory
governance read model starts empty. A 24/7 Agent OS must be able to recover the
bounded governance identity for agent-created Skills from already materialized
packages instead of losing active governance state on process restart.

## What Changes
- Add a service-owned recovery path that scans materialized Skill packages for
  bounded `SKILL.md` frontmatter and provenance refs.
- Rebuild missing `Active` governance records through the Skill service read
  model without parsing raw prompts, full task output, package bytes, or
  application-specific content.
- Keep shell adapters unchanged: Web and CLI continue to request governance
  snapshots through `service.skill`.

## Impact
- Affected specs: `skill-governance-curation`
- Affected code:
  - `macaca/crates/runtime/macaca-runtime-host/src/skill_service_provider_state.rs`
  - `macaca/crates/runtime/macaca-runtime-host/src/skill_service_provider.rs`
  - focused runtime-host tests
