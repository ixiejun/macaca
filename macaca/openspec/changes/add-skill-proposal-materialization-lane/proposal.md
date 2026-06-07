# Change: Add Skill Proposal Materialization Lane

## Why

The platform now captures real Skill experience proposals and processes them
into `ReadyForMaterialization`, but it still does not write an auditable
`SKILL.md` draft. This leaves self-evolution stuck between proposal readiness
and actual reusable Skill creation.

## What Changes

- Add a provider-neutral Skill proposal materialization command behind
  `service.skill`.
- Require a `ReadyForMaterialization` processing record before any write.
- Generate bounded AgentSkills-compatible `SKILL.md` content through a Builder.
- Reuse the existing content-mutation Strategy for filesystem writes, path
  admission, rollback mementos, and sanitized mutation results.
- Promote the draft proposal into active governance metadata only after
  successful materialization.
- Keep Web, CLI, frontend, SDK consumers, and applications from owning
  materialization semantics.

## Impact

- Affected specs: `skill-governance-curation`
- Affected code:
  - `macaca/crates/services/macaca-skill/src/*`
  - `macaca/crates/runtime/macaca-runtime-host/src/*skill*`
- Architecture constraints:
  - Kernel remains provider-neutral.
  - Runtime-host owns the built-in local Strategy only.
  - Skill service owns the command, validation, policy envelope, state gate, and
    sanitized result DTOs.
  - Materialization does not hardcode application, workflow, provider, driver,
    or business-domain names.
