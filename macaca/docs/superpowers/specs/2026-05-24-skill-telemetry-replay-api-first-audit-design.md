# Skill Telemetry Replay And API-First Audit Design

## Context

Run 49 proved that governed Skill usage telemetry increments during a live `/api/chat/v2` task, but those counters reset after process restart. The root cause is that the built-in Skill provider appends governance events to an in-memory vector only. Restart package recovery can rebuild the `Active` identity from materialized `SKILL.md` provenance, but it cannot replay prior `Activated` or `SuccessfulTask` observations.

Run 49 also proved a second failure mode: an agent-written audit artifact can contradict canonical API state when it inspects filesystem artifacts first. Self-evolution proof needs an API-first evidence surface that reads Skill operations, registry/load-path projection, and session observer events before using filesystem details as supporting evidence.

## Design

Use a `Memento + Strategy` slice for durable telemetry replay. The local Skill provider writes every sanitized `SkillGovernanceEventRecord` to an append-only JSONL journal configured by the web composition root. On provider start, the same provider replays the journal into `SkillGovernanceReadModel` before materialized package recovery runs. This keeps replay semantics inside the Skill service boundary and makes the journal replaceable by a future Store/EventLog strategy.

Use a `Facade/Adapter + Specification` slice for API-first audit. Web exposes a thin diagnostic route that aggregates canonical Skill operations, registry/load-path visibility, and bounded session observer evidence through existing service and session APIs. The route returns explicit statuses and missing-evidence reasons; it does not read full Skill bodies, raw prompts, provider payloads, or application-specific task content.

## Constraints

- Keep Skill governance semantics in `macaca-runtime-host` / `macaca-skill`, not in Web.
- Keep Web as a composition root and diagnostic adapter only.
- Persist sanitized governance events only; never persist raw prompts, full task output, package bytes, credentials, or full `SKILL.md` bodies.
- Use generic workspace/service paths and typed DTOs; do not branch on app names, provider names, driver names, workflow names, or business domains.

## Verification

- Unit test durable replay by recording `Created`, `Activated`, and `SuccessfulTask`, restarting a provider with the same journal, and asserting counters survive.
- Unit test API-first audit DTO construction so missing registry, operations, or observer evidence returns explicit failed status.
- Validate OpenSpec strictly.
- Run targeted cargo tests for runtime-host Skill provider and web audit helpers.
