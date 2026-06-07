## Context

The first `macaca-agent` refactor is complete. The next risk is that upper crates consume these primitives while they are still shaped like implementation details. This change makes the primitives explicit module boundaries without changing behavior.

## Goals

- Preserve `Agent` trait behavior and `BasicAgent` behavior 1:1.
- Preserve no-op service side effects exactly: no memory writes, no IPC side effects, no persist writes.
- Preserve lifecycle transition matrix exactly.
- Preserve legacy flattened capability output exactly.
- Provide canonical additive APIs for future consumer migration.
- Mark legacy direct construction paths as deprecated without deleting them.

## Non-Goals

- Do not migrate framework/web/sdk/kernel consumers in this change.
- Do not introduce AgentSpec or traced construction contracts.
- Do not make `AgentServices` fields private yet.
- Do not change trace, EventLog, SSE, task, planner, worker, or coordinator behavior.

## Decisions

- Module extraction is allowed only if `lib.rs` re-exports keep existing imports compiling.
- `AgentServicesBuilder` is additive; existing direct struct construction remains possible during migration.
- Legacy constructors remain callable but are marked `#[deprecated]` to make follow-up migration grepable.
- Capability source inspection is read-only; mutation still happens through explicit constructors and `push_group`.
- Lifecycle preflight returns boolean/result based on the same `transition_reason` semantics as `transition`.
