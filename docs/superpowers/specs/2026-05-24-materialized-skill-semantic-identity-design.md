# Materialized Skill Semantic Identity Design

## Problem

Materialized Skills can currently receive proposal-id-derived names such as
`skill-exp-<task-id>-<timestamp>`. These names are stable for audit but weak for
model selection because the agent snapshot asks the model to match skills by
name and description.

## Design

Keep immutable proposal ids in provenance and governance, but derive the
model-facing `SKILL.md` name from semantic, bounded proposal evidence. The
existing materialization Builder is the right boundary because it already owns
`SKILL.md` construction while mutation, rollback, promotion, and policy remain
in their existing service-owned strategies.

The Builder should prefer `target_skill_name` when supplied. If absent, it
should deterministically derive a short ASCII slug from sanitized reusable
procedure text, falling back to bounded summary text and finally to a short
proposal-id suffix if no meaningful tokens exist. Generated descriptions and
`When To Use` sections should expose bounded trigger context without raw task
output or provider payloads.

## Architecture Notes

- Pattern: Builder for identity/rendering, Strategy for mutation, Specification
  for bounded deterministic derivation, Observer for sanitized trace logs.
- Ownership: `service.skill` provider owns this behavior. Web, CLI, kernel, and
  application code remain outside naming semantics.
- Audit: proposal id, task id, trace id, content digest, rollback ref, and
  materialization refs remain unchanged as provenance.
- Extensibility: future alias or ranking work can consume the semantic name
  without replacing this deterministic baseline.
