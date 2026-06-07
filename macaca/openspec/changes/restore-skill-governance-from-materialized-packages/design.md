## Context
The local Skill service provider currently keeps governance records in memory.
Materialized packages are durable enough to survive restart, and registry
projection can rediscover them, but the governance snapshot does not replay an
`Active` record unless a usage or lifecycle event is observed in the new
process.

## Goals
- Recover missing governance records for materialized agent-created Skill
  packages after provider restart.
- Use only bounded package metadata: YAML frontmatter and the explicit
  provenance refs already written by the materialization Builder.
- Preserve ownership: recovery lives in the Skill service/runtime-host provider,
  not Web, CLI, frontend, or application code.
- Emit key logs for discovered, skipped, and recovered packages.

## Non-Goals
- Do not restore historical counters from task logs in this slice.
- Do not infer semantic quality or optimization from package presence.
- Do not branch on application names, workflow names, model names, provider
  names, or specific Skill names.
- Do not read executable scripts or arbitrary bundled resources.

## Design
Use Memento, Strategy, and Specification patterns.

The materialized `SKILL.md` acts as a bounded Memento for identity recovery. A
small recovery Strategy scans configured package roots for direct child
`SKILL.md` files, validates each candidate with a Specification requiring
frontmatter `name`, `description`, and a provenance `Proposal ref`, and then
creates a `Created` usage event through the existing governance event path.

The provider runs recovery at `start()` and may also allow snapshot-time
recovery for tests or future composition roots. The scan remains generic: roots
come from provider-neutral workspace configuration, and the recovered record
stores source/scope/evidence refs rather than raw bodies.

## Risks
- Package presence alone cannot recover historical telemetry counters. The
  recovered record must be `Active` with zero counters until later observations
  occur.
- Malformed or hand-authored Skills may lack provenance. The recovery Strategy
  logs a bounded skip reason and leaves them unmanaged rather than inventing
  governance state.
