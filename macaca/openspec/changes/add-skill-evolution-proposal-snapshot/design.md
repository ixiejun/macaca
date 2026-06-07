## Context

Self-evolving skills require an auditable draft lifecycle. The previous slice added proposal creation but no read model, which would force future shells or review providers to infer state from provider internals.

## Goals / Non-Goals

- Goal: expose draft proposal state through a traced Skill service command.
- Goal: keep the command non-mutating and sanitized.
- Goal: keep the built-in provider replaceable by future Store/EventLog-backed strategies.
- Non-goal: promote, reject, patch, archive, or activate any skill.
- Non-goal: add LLM review or auto-apply policy.

## Decisions

- Decision: extend `service.skill` instead of adding a separate service for this read-only slice.
  - Reason: the current Skill service already owns governance and draft proposal state.
- Decision: return records sorted by proposal id.
  - Reason: deterministic output improves tests, audit replay, and future snapshot comparison.
- Decision: include an `include_discarded` flag even though current records do not yet model discarded state.
  - Reason: this is a provider-neutral extension point for later lifecycle states without requiring consumers to change method shape.

## Risks / Trade-offs

- Risk: the Skill service descriptor is shared by web startup and service registration.
  - Mitigation: do not change service id, existing permissions, or existing commands; add only the new command constant and route.
- Risk: proposal records could become an unbounded observability surface.
  - Mitigation: reuse sanitized proposal DTOs and avoid raw prompts, raw task output, provider payloads, manifests, package bytes, or secrets.

## Migration Plan

Existing proposal creation remains compatible. Consumers that do not call `skill.evolution.snapshot` are unaffected.

## Open Questions

- Future slices must decide whether proposal promotion/rejection belongs in the same service command family or a separate approval-gated curation provider.
