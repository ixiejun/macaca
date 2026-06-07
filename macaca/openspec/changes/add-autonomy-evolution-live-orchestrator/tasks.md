## 1. OpenSpec And Governance

- [x] 1.1 Read governance docs, OpenSpec instructions, existing six autonomy
  evolution proposals, and the complete-self-evolution Superpowers design.
- [x] 1.2 Create `proposal.md`, `design.md`, `tasks.md`, and delta spec for
  `add-autonomy-evolution-live-orchestrator`.
- [x] 1.3 Validate `add-autonomy-evolution-live-orchestrator` with `--strict`.

## 2. Service DTOs And Strategy Interfaces

- [x] 2.1 Run GitNexus impact analysis for the autonomy evolution provider,
  SDK client, runtime-host provider, transition, benchmark, release, and ledger
  symbols before editing Rust code.
- [x] 2.2 Add failing service tests for successful live tick progression,
  duplicate idempotency, missing evidence denial, unsupported target
  unavailable, rollback-memento denial, and ledger replay resume.
- [x] 2.3 Add provider-neutral live tick and audit reconstruction DTOs with
  trace, scope, lease id, idempotency key, evidence refs, policy refs, audit
  refs, phase statuses, bounded reason codes, and sanitized checkpoints.
- [x] 2.4 Add Strategy traits for candidate discovery, workload collection,
  target dispatch, canary observation, audit reconstruction, and the live
  orchestrator.
- [x] 2.5 Export DTOs and traits through `macaca-autonomy-evolution` without
  exposing provider internals.

## 3. Local Provider And Orchestrator Execution

- [x] 3.1 Implement conservative default validation that fails closed for
  missing trace, actor, lease, idempotency key, observer evidence, policy refs,
  or required audit refs.
- [x] 3.2 Implement idempotency checkpoints so duplicate live ticks return the
  existing result without duplicating promotion, rollback, or ledger append
  side effects.
- [x] 3.3 Compose existing transition, admission, benchmark, release, ledger,
  and OS-code proposal adapter logic from the live orchestrator instead of
  duplicating their semantics.
- [x] 3.4 Ensure Skill target dispatch delegates to existing Skill service
  boundaries and unsupported targets return structured unavailable.
- [x] 3.5 Append sanitized governance ledger records for discovery, transition,
  admission, benchmark, release, target dispatch, rollback, and audit
  reconstruction checkpoints.
- [x] 3.6 Add structured logs at tick start, phase start, phase completion,
  denial, unavailable provider, release decision, target dispatch, rollback,
  ledger append, audit replay, and terminal state.

## 4. SDK And Runtime-Host

- [x] 4.1 Add SDK/SystemFacade live tick and audit reconstruction methods with
  Null Object unavailable results.
- [x] 4.2 Add runtime-host decode/forwarding for live tick and audit commands.
- [x] 4.3 Add SDK tests proving unavailable behavior does not fake success.
- [x] 4.4 Add runtime-host tests proving command decoding, trace propagation,
  and structured unavailable behavior.

## 5. Verification

- [x] 5.1 Run `openspec validate add-autonomy-evolution-live-orchestrator --strict`.
- [x] 5.2 Run `cargo test -p macaca-autonomy-evolution`.
- [x] 5.3 Run targeted SDK and runtime-host autonomy evolution tests.
- [x] 5.4 Run `git diff --check`.
- [x] 5.5 Run GitNexus change detection before commit and record any HIGH or
  CRITICAL warnings as advisory notes for this refactor.
- [x] 5.6 Commit the OpenSpec, implementation, and tests.
