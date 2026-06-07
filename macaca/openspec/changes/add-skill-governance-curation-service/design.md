## Context

`docs/macaca-agent-self-evolving-skills-research.md` concluded that self-evolving agents need Skill Evolution and Skill Curation capabilities, but those capabilities must be governed like OS services. Hermes Curator provides useful mechanics such as usage metadata, pinned skills, archive-first cleanup, snapshots, reports, and umbrella-skill consolidation. Macaca must implement the same class of behavior through service contracts, trace, policy, audit, and replaceable providers.

The current Macaca Skill service already exposes traced commands for knowledge skill snapshots, executable skill loading, tool catalogs, invocation, status, cleanup, and service snapshots. This change extends that existing service boundary instead of introducing kernel logic or shell-owned semantics.

## Goals

- Model skill lifecycle state, provenance, usage telemetry, and curation reports as typed service data.
- Let agents and framework tools record usage/view/update observations through a traced Skill service command.
- Provide a deterministic curation dry-run report that classifies stale, overused, narrow, duplicate, and pinned candidates without modifying files.
- Make future destructive operations possible only after policy, approval, memento, and audit support are added in later changes.
- Preserve compatibility for existing snapshot, executable skill, and tool invocation behavior.

## Non-Goals

- No LLM-driven skill rewriting in this slice.
- No file patching, merge, archive, delete, or restore operation in this slice.
- No marketplace, remote store, entitlement sale, or package publication workflow.
- No application-specific skill taxonomy or business workflow rules.

## Decisions

- Decision: Extend `service.skill` instead of creating a kernel primitive.
  - Rationale: Skill evolution is a replaceable capability family and the three Macaca constitutions require it to live behind a service boundary.
- Decision: Use Command and Facade patterns for every new operation.
  - Rationale: SDK consumers and shells need typed calls while runtime-host remains the provider adapter.
- Decision: Use State, Observer, Memento, and Specification vocabulary in the data model.
  - Rationale: lifecycle state, usage events, pre-run snapshots, and curation eligibility rules must be explicit and auditable.
- Decision: Make curation dry-run deterministic and non-destructive.
  - Rationale: this is the smallest safe implementation slice and avoids shipping prompt-only automated mutation before policy and rollback are complete.

## Risks / Trade-offs

- Risk: Adding commands to the Skill descriptor affects service registration consumers.
  - Mitigation: only append capabilities and permissions; existing command names and results remain unchanged.
- Risk: Usage telemetry could leak task content.
  - Mitigation: the command stores counts, lifecycle hints, source labels, and evidence ids, not raw prompts or raw task outputs.
- Risk: Dry-run heuristics could be mistaken for authoritative cleanup.
  - Mitigation: report actions are recommendations only and result fields use explicit `would_*` action names.

## Migration Plan

1. Add typed governance and curation data models in `macaca-skill`.
2. Extend the Skill service provider and SDK client with governance snapshot, usage record, and curation dry-run calls.
3. Add focused unit tests for descriptor shape, Null Object behavior, and provider dry-run behavior.
4. Validate OpenSpec and run targeted Rust tests.

## Open Questions

- Later changes must decide whether governance state persists in Store, EventLog, or a dedicated Skill Governance Store provider.
- Later changes must decide how human approval and autonomous policy thresholds gate destructive curation actions.
