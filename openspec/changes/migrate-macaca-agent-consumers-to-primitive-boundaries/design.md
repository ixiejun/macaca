## Context

The previous `macaca-agent` primitive-boundary refactor introduced `AgentServices::builder()`, `AgentCapabilitySet`, and lifecycle policy primitives while keeping legacy helpers deprecated for compatibility.

The only upper-crate deprecated service construction calls currently found are in `macaca-kernel` and `macaca-sdk` tests. Framework/web already consume `AgentServices` and `AgentCapabilitySet`, while larger traced construction cleanup remains covered by `migrate-agent-construction-to-framework-primitives`.

## Goals

- Keep behavior 1:1 compatible.
- Remove upper-crate usage of deprecated `macaca-agent` helper constructors.
- Preserve deprecated helper definitions inside `macaca-agent` for temporary compatibility and grepability.
- Keep this migration independent from the broader framework construction migration.

## Non-Goals

- Do not remove deprecated helper definitions from `macaca-agent`.
- Do not migrate app manifest capability modeling in this change.
- Do not change traced agent construction, session behavior, EventLog behavior, task scheduling, or planner/worker/coordinator behavior.

## Decisions

- Use `AgentServices::builder().build()` instead of `AgentServices::default()` at migrated call sites because it explicitly exercises the new builder pattern.
- Treat `AgentServices::default()` as acceptable inside framework request defaults because it delegates to the builder and is not deprecated.
- Add a verification grep that excludes definitions inside `macaca-agent` and checks upper crates only.
