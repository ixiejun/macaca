## Context

The research document `docs/macaca-agent-self-evolving-skills-research.md`
places skill lifecycle governance in the Skill service, not in the kernel,
application code, or Web shell.  Prior changes implemented telemetry,
non-destructive dry-run recommendations, alias resolution, draft proposal
snapshots, and a read-only operations panel.  This change adds the next
metadata-only lifecycle command layer.

## Goals / Non-Goals

- Goal: expose generic, trace-required lifecycle mutation commands through
  `service.skill`.
- Goal: keep mutation limited to governance metadata in this slice.
- Goal: keep pinned-skill protection executable and tested.
- Non-goal: mutate skill files, merge skill bodies, delete directories, create
  aliases, or run LLM review.
- Non-goal: add frontend buttons or user approval UI.

## Decisions

- Decision: use separate command names for pin, unpin, archive, and restore.
  - Reason: each action is auditable and policy-addressable without a generic
    free-form mutation command.
- Decision: require reason and evidence ids.
  - Reason: lifecycle operations must be traceable and replayable, even while
    the built-in provider remains in-memory.
- Decision: deny archive when a record is pinned.
  - Reason: pinned records represent an explicit protection state; overriding it
    needs a future approval and policy contract.

## Risks / Trade-offs

- Risk: lifecycle commands could be mistaken for full curation.
  - Mitigation: commands update governance metadata only and return `mutated`
    for metadata mutation, not file mutation.
- Risk: missing durable persistence loses lifecycle state after restart.
  - Mitigation: this is explicitly the built-in provider Strategy; future Store
    providers can implement the same typed contract.
- Risk: shells could grow semantics around these actions.
  - Mitigation: this change does not add UI mutation; Web remains an adapter.
