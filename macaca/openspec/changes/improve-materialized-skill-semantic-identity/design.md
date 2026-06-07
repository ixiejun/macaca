## Context

AgentSkills are selected by model-facing name, description, and location in the
snapshot prompt. A proposal id is intentionally stable and auditable, but it is
not a useful trigger for autonomous reuse. The materialization lane already uses
a Builder for bounded `SKILL.md` content, making the identity repair local to
service-owned materialization without moving semantics into Web, CLI, kernel, or
application code.

## Goals

- Give newly materialized Skills a semantic, bounded, generic name when the
  proposal does not already specify one.
- Preserve proposal id, task id, trace id, content digest, and rollback refs as
  audit evidence.
- Improve `description` and `When To Use` so model-triggered activation can
  match future tasks without explicitly naming a UUID-like id.
- Keep all derivation provider-neutral and application-agnostic.
- Emit key-node logs that show identity derivation without leaking raw content.

## Non-Goals

- Do not introduce embedding search, ranking services, or new dependencies.
- Do not branch on application names, business domains, workflow names, model
  names, provider names, or driver names.
- Do not rewrite existing materialized packages in this slice.
- Do not claim measured downstream optimization from better naming alone.
- Do not expose full generated `SKILL.md` bodies in results, logs, or snapshots.

## Decisions

- Decision: keep the repair inside the existing materialization Builder.
  - Reason: the Builder already owns proposal-to-`SKILL.md` construction and is
    the narrowest service-owned boundary for model-facing identity.
- Decision: prefer `target_skill_name` when present, then derive a semantic slug
  from sanitized reusable procedure and bounded summary, then use a short
  proposal-id suffix only as collision-resistant fallback context.
  - Reason: user-provided or service-derived semantic names should win, while
    audit ids remain available in provenance.
- Decision: derive names with deterministic token filtering rather than LLM
  calls.
  - Reason: materialization must be reproducible, cheap, offline-capable, and
    free of provider-specific behavior.
- Decision: log the chosen semantic name, source, and fallback state.
  - Reason: operators need traceable evidence that the model-facing identity was
    intentionally derived and bounded.

## Design Patterns

- **Builder**: `SkillDraftMaterializationBuilder` composes semantic identity and
  markdown without owning file mutation.
- **Specification**: helper functions enforce bounded token count, slug length,
  generic stop-word filtering, and fallback behavior.
- **Observer**: materialization emits sanitized trace logs for identity
  derivation and final materialization result.
- **Strategy**: file writes still go through the existing content-mutation
  Strategy; naming does not bypass policy, entitlement, or rollback gates.

## Data Flow

1. Proposal processing marks a proposal `ReadyForMaterialization`.
2. Materialization command validates trace, evidence, package, entitlement, and
   policy fields.
3. The Builder derives `MaterializedSkillIdentity` from:
   - `target_skill_name`, if present;
   - otherwise sanitized reusable procedure text;
   - otherwise sanitized bounded summary;
   - otherwise a bounded proposal-id fallback.
4. The Builder renders `SKILL.md` with semantic `name`, bounded `description`,
   specific `When To Use`, reusable procedure, and immutable provenance refs.
5. Mutation Strategy writes the file and returns rollback/content-digest refs.
6. Governance promotion records the semantic name while preserving proposal id
   and trace ids.

## Risks And Mitigations

- Risk: generated names become too generic.
  Mitigation: require multiple meaningful tokens when possible and append a
  short stable proposal suffix only when needed.
- Risk: derived names leak raw task output.
  Mitigation: use existing bounded summary/procedure fields, filter punctuation,
  enforce ASCII slug output, and keep raw body out of logs/results.
- Risk: better names are mistaken for optimization proof.
  Mitigation: OpenSpec and monitoring docs keep naming/activation separate from
  token/tool/time improvement metrics.
