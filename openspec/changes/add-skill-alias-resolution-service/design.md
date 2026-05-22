## Context

The previous `add-skill-governance-curation-service` slice added sanitized governance telemetry and curation dry-run reports. The research document identifies one more critical Hermes Curator behavior: after consolidation, old skill references must not break. Hermes handles this by rewriting cron references; Macaca should model identity redirection as a service-owned alias map instead.

The current runtime-host Skill provider is also at the 500-line governance limit. This slice must split reusable provider state into a focused module before adding alias behavior.

## Goals

- Represent skill redirects, supersession, and absorbed-into relationships as typed, sanitized service records.
- Allow consumers to resolve a skill id or name through a traced Skill service command.
- Expose an alias snapshot for diagnostics, audit replay, and future UI display.
- Preserve existing Skill snapshot, tool catalog, governance telemetry, and dry-run behavior.
- Keep alias writes non-destructive and provider-neutral.

## Non-Goals

- No automatic scheduler/task/context rewrite.
- No skill file patching, archive, restore, delete, or merge.
- No LLM-driven semantic consolidation.
- No application-specific alias rules or hardcoded skill names.

## Decisions

- Decision: Add aliases to `service.skill`.
  - Rationale: identity resolution belongs to the Skill capability family, not kernel, scheduler, Web, or CLI.
- Decision: Use Command, Facade, Memento, and State patterns.
  - Rationale: alias records are replayable governance state and must be accessible through SDK clients.
- Decision: Keep `SkillSystemServiceProvider` as the public provider type, but move mutable state helpers to `skill_service_provider_state.rs`.
  - Rationale: this preserves callers while making the implementation extensible and keeping files below the 500-line ceiling.

## Risks / Trade-offs

- Risk: alias resolution could hide missing skills if it silently succeeds.
  - Mitigation: resolution results include `resolved`, `target_skill_id`, and `reason`; absent aliases return `resolved = false`.
- Risk: alias records could leak task text.
  - Mitigation: records store ids, names, kind, rationale, timestamps, and evidence ids only.
- Risk: splitting provider state could accidentally change behavior.
  - Mitigation: keep constructor signatures and command names stable; rerun existing governance tests and add alias tests.

## Migration Plan

1. Add alias DTOs and command constants.
2. Split runtime-host governance state into a small provider-state module.
3. Implement alias upsert, resolve, and snapshot commands in the provider.
4. Extend SDK facade and Null Object behavior.
5. Validate OpenSpec, run focused tests, and run GitNexus change detection.
