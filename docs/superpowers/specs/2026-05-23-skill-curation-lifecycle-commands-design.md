# Skill Curation Lifecycle Commands Design

## Context

`docs/macaca-agent-self-evolving-skills-research.md` recommends that Macaca
move from read-only skill governance snapshots toward governed curation actions.
The current implementation already has service-owned governance telemetry,
deterministic dry-run recommendations, alias snapshots, draft experience
proposals, and a read-only operations shell.  The missing low-risk bridge is a
typed service command surface for lifecycle mutations that can later be wrapped
by approval, rollback, and durable Store/EventLog providers.

The implementation must obey `macaca-os-architecture-governance.md`,
`macaca-os-microkernel-boundaries.md`, and
`macaca-os-serviceization-allowlist.md`: the kernel does not curate skills,
Web/CLI do not own lifecycle semantics, SDK exposes a facade, and runtime-host
adapts the built-in provider.

## Design

Use the Command, Facade, State, Specification, Observer, and Memento vocabulary
already present in the Skill service:

- Add typed lifecycle mutation commands for `pin`, `unpin`, `archive`, and
  `restore`.
- Require `TraceContext`, `SkillServiceScope`, sanitized target identity,
  reason, evidence ids, and policy hints on every command.
- Keep actions metadata-only in this slice.  They update governance records and
  emit logs, but do not patch `SKILL.md`, delete files, merge instructions, or
  modify aliases.
- Protect pinned skills from archive unless a future command explicitly defines
  approval semantics.  This slice returns a structured denied/invalid argument
  error instead of silently archiving.
- Keep unavailable SDK behavior explicit: mutation commands return structured
  unavailable errors, while read commands continue to return empty safe
  snapshots.

## Scope

In scope:

- Provider-neutral DTOs in `macaca-skill`.
- Built-in runtime-host provider state transitions.
- SDK facade methods and Null Object behavior.
- Focused provider tests and OpenSpec deltas.

Out of scope:

- LLM curation review.
- Automatic apply policy.
- File patching, deletion, umbrella merge, or alias creation.
- Frontend mutation UI.
- Durable Store/EventLog persistence.

## Risk Controls

- No application-specific branches, skill names, provider names, workflow names,
  or business logic.
- Every operation requires trace and records sanitized evidence ids only.
- Logs report target ids, action, lifecycle result, and protected/denied cases
  without raw prompts, manifests, skill bodies, package bytes, or provider
  payloads.
- Tests prove pin protection, archive/restore lifecycle transitions, snapshot
  visibility rules, and missing evidence validation.
