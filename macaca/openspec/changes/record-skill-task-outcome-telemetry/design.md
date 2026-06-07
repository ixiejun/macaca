## Context
The Skill service already owns sanitized usage telemetry counters and exposes
typed `SkillGovernanceRecordUsageCommand` commands. The Web shell already
records activation telemetry when a governed Skill is visible in an agent Skill
snapshot. The remaining live-proof gap is task outcome telemetry: after a later
task completes, the governed Skill record still has `successful_task_count = 0`.

## Goals
- Record a task success event when Agent Execution returns `Completed` and the
  session has a cached Skill snapshot containing active governed Skills.
- Use only refs, counters, agent/session ids, and trace ids.
- Keep policy, storage, aggregation, and report semantics in `service.skill`.

## Non-Goals
- Do not infer optimization from success telemetry alone.
- Do not parse prompts, task content, or Skill bodies.
- Do not branch on application names, provider names, model names, or Skill
  names.

## Design
Use the Observer and Command patterns.

The existing Agent Execution decorator remains the Observer of the generic
completion boundary. On successful completion it calls a focused Web adapter
helper. The helper loads the cached `skill_snapshot/{agent}` memento for that
session, reads active governance records through the SDK Skill client, filters
records whose names were visible in the snapshot, and sends one
`SuccessfulTask` usage command per matching governed Skill.

This keeps the shell thin: it owns only conversion from a completed service
result into bounded command metadata. The Skill service remains the semantic
owner of usage counters, lifecycle records, audit-safe aggregation, and
evaluation scoring.

## Risks
- A visible Skill snapshot can include Skills that were available but not
  semantically selected by the model. This mirrors existing activation telemetry
  semantics and is intentionally treated as "visible activated Skill state",
  not proof of quality improvement.
- Missing cached snapshots should not fail the user task. The helper logs a
  bounded skip reason and returns.
