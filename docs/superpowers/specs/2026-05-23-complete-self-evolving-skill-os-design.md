# Complete Self-Evolving Skill OS Planning Design

## Brainstorm Summary

The research leaves three possible implementation strategies.

1. Land more UI and command buttons first.
   - Benefit: operators can act on the current in-memory lifecycle commands.
   - Risk: the shell would gain mutation surfaces before durable audit,
     rollback, and ownership policy exist.
2. Land an LLM curator provider first.
   - Benefit: moves directly toward merge and skill-quality improvements.
   - Risk: semantic review would be a prompt-driven black box without typed
     proposal, durable governance, rollback, and policy boundaries.
3. Land the durable Skill Governance Store and state machine first, then layer
   task extraction, proposal lifecycle, safe mutation, curation runs, semantic
   review, context integration, and shell approvals.
   - Benefit: every later side effect is traceable, auditable, replayable, and
     governed through service boundaries.
   - Risk: slower path to visible automation, but the risk is bounded because
     each slice remains independently testable.

## Selected Approach

Use option 3. The next work must start with Store/EventLog-backed governance and
explicit lifecycle state. The existing in-memory provider is useful as a
compatibility adapter, but it is not sufficient for 24/7 autonomous operation.

## Design Pattern Mapping

- Command: typed service commands for evolution, curation, lifecycle, alias,
  mutation, rollback, and approval.
- Facade: SDK clients expose the stable boundary to Web, CLI, frontend, Task,
  Context, and applications.
- Strategy: governance store, semantic review, similarity, archive policy,
  merge eligibility, and mutation applier remain replaceable.
- Decorator: trace, policy, package guard, resource, entitlement, budget,
  metering, and sanitization wrap side-effecting commands.
- State: lifecycle transitions are explicit and enforced.
- Observer: usage, activation, mutation, curation, alias, and rollback events
  flow to audit/event streams.
- Memento: curation runs and mutations create before/after snapshots and
  rollback refs.
- Specification: package ownership, merge eligibility, sensitive-content,
  executable script, context visibility, and admission gates are executable
  rules.
- Abstract Factory: runtime-host owns built-in, plugin, remote, mock,
  unavailable, and optional semantic provider construction.

## Boundary Decisions

- Kernel only routes typed calls and keeps trace/audit/policy primitives.
- Skill service owns self-evolution and curation semantics.
- Store/EventLog owns durable records and replay.
- Task/Autonomy provides verified success events, not skill-writing logic.
- Memory/Knowledge receive non-procedural classified knowledge through their
  own service facades.
- Context Composer consumes Skill service snapshots and alias resolution only.
- Web/CLI/frontend submit commands and render reports only.

## Risk Controls

- Add durable governance before active file mutation.
- Keep draft and dry-run paths non-mutating until rollback is proven.
- Treat LLM/similarity providers as optional typed proposal producers.
- Deny protected package mutation by default.
- Require trace, policy, entitlement, and package guard before side effects.
- Sanitize every report, log, snapshot, and shell payload.
