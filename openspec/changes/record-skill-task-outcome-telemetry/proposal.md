# Change: Record Skill task outcome telemetry

## Why
Live self-evolution proof now shows that a materialized Skill can become
catalog-visible and record activation, but a later successful task does not
increment task outcome telemetry. Without that bounded outcome counter, the
evaluation harness can prove reuse only partially and cannot score a later
task-family run as successful Skill-backed execution.

## What Changes
- Add a generic Web adapter observer that records `SuccessfulTask` telemetry for
  active governed Skills that were visible in the completed task's cached Skill
  snapshot.
- Keep scoring and telemetry ownership inside `service.skill`; the Web shell
  only converts the Agent Execution completion boundary into a typed Skill
  usage command.
- Preserve distinct evidence layers: materialization, catalog visibility,
  activation, task outcome telemetry, and optimization evaluation remain
  separately measurable.

## Impact
- Affected specs: `skill-governance-curation`
- Affected code:
  - `macaca/crates/shells/macaca-web/src/skill_usage_telemetry.rs`
  - `macaca/crates/shells/macaca-web/src/skill_self_evolution_execution_observer.rs`
  - `macaca/crates/shells/macaca-web/src/lib.rs`
