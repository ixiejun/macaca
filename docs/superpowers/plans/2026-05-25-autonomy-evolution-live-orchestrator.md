# Autonomy Evolution Live Orchestrator Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Connect the existing autonomy-evolution control plane, admission, benchmark, release safety, governance ledger, and target adapters into a resumable unattended orchestration loop.

**Architecture:** Add a thin service-owned live orchestrator that owns leases, idempotency, phase ordering, and audit stitching while delegating lifecycle validation, admission, benchmark scoring, release safety, ledger persistence, and target mutation to existing services. The orchestrator is generic and application-agnostic, with replaceable Strategies for candidate discovery, workload collection, target dispatch, and audit reconstruction.

**Tech Stack:** Rust workspace under `macaca/`, `macaca-autonomy-evolution`, runtime-host service providers, SDK/SystemFacade focused clients, OpenSpec, tracing, serde DTOs, governance ledger replay, targeted Rust unit tests.

---

## Files And Ownership Map

- Create OpenSpec:
  - `openspec/changes/add-autonomy-evolution-live-orchestrator/proposal.md`
  - `openspec/changes/add-autonomy-evolution-live-orchestrator/design.md`
  - `openspec/changes/add-autonomy-evolution-live-orchestrator/tasks.md`
  - `openspec/changes/add-autonomy-evolution-live-orchestrator/specs/autonomy-evolution-control-plane/spec.md`
- Modify service crate:
  - `macaca/crates/services/macaca-autonomy-evolution/src/live_orchestrator_model.rs`
  - `macaca/crates/services/macaca-autonomy-evolution/src/live_orchestrator.rs`
  - `macaca/crates/services/macaca-autonomy-evolution/src/lib.rs`
  - `macaca/crates/services/macaca-autonomy-evolution/src/local_provider.rs`
- Modify SDK facade:
  - `macaca/crates/facade/macaca-sdk/src/autonomy_evolution_client.rs`
- Modify runtime-host adapter:
  - `macaca/crates/runtime/macaca-runtime-host/src/autonomy_evolution_service_provider.rs`
- Add tests:
  - `macaca/crates/services/macaca-autonomy-evolution/tests/evolution_live_orchestrator_tests.rs`
  - `macaca/crates/facade/macaca-sdk/tests/autonomy_evolution_live_orchestrator_client_tests.rs`
  - `macaca/crates/runtime/macaca-runtime-host/tests/autonomy_evolution_live_orchestrator_service_provider_tests.rs`

## Task 1: OpenSpec And Boundary Check

- [ ] **Step 1: Validate active OpenSpec context**

Run:

```bash
openspec list
openspec list --specs
openspec show add-autonomy-evolution-control-plane --json --deltas-only
```

Expected: the six previous autonomy evolution changes are complete or visible, and no active change already owns live orchestrator behavior.

- [ ] **Step 2: Validate the new proposal**

Run:

```bash
openspec validate add-autonomy-evolution-live-orchestrator --strict
```

Expected: validation passes before Rust implementation begins.

## Task 2: Live Orchestrator DTOs And Tests

- [ ] **Step 1: Run GitNexus impact analysis**

Run impact analysis for these symbols before editing:

```text
LocalAutonomyEvolutionProvider
AutonomyEvolutionClient
AutonomyEvolutionServiceProvider
EvolutionTransitionCommand
EvolutionGovernanceLedger
EvolutionBenchmarkCommand
EvolutionReleaseCommand
```

Expected: record the affected direct callers, processes, and risk level. Treat HIGH or CRITICAL as advisory for this refactor, but document them in the implementation summary.

- [ ] **Step 2: Write failing service tests**

Create `macaca/crates/services/macaca-autonomy-evolution/tests/evolution_live_orchestrator_tests.rs` with tests for:

- one bounded tick advances observation evidence into an admitted candidate path;
- duplicate tick idempotency returns the existing checkpoint instead of duplicating records;
- missing admission evidence fails closed;
- unsupported target adapter returns unavailable;
- rollback-required release without memento is denied;
- replay cursor resumes after provider restart.

- [ ] **Step 3: Add DTOs**

Create `live_orchestrator_model.rs` with provider-neutral DTOs:

- `AUTONOMY_EVOLUTION_LIVE_TICK_COMMAND`
- `AUTONOMY_EVOLUTION_LIVE_AUDIT_COMMAND`
- `EvolutionLiveTickCommand`
- `EvolutionLiveTickResult`
- `EvolutionLivePhase`
- `EvolutionLivePhaseStatus`
- `EvolutionLiveCandidateDiscovery`
- `EvolutionLiveWorkloadPlan`
- `EvolutionLiveAdapterDispatch`
- `EvolutionLiveAuditCommand`
- `EvolutionLiveAuditResult`

All DTOs must carry `TraceContext`, `EvolutionScope`, bounded evidence refs, policy decision refs, audit refs, idempotency key, lease id, and sanitized reason codes.

- [ ] **Step 4: Export DTOs**

Modify `lib.rs` to expose the model module without exposing provider internals.

- [ ] **Step 5: Run targeted tests**

Run:

```bash
cargo test -p macaca-autonomy-evolution evolution_live_orchestrator -- --nocapture
```

Expected before implementation: compile or behavior failure that proves the tests are meaningful.

## Task 3: Orchestrator Strategy And Local Provider

- [ ] **Step 1: Implement Strategy traits**

Create `live_orchestrator.rs` with focused traits:

- `EvolutionCandidateDiscoveryStrategy`
- `EvolutionBenchmarkWorkloadRunner`
- `EvolutionTargetDispatchStrategy`
- `EvolutionAuditReconstructionStrategy`
- `EvolutionLiveOrchestrator`

Each trait must be provider-neutral, use command/result DTOs, and log key execution nodes with trace id, run id, lease id, phase, decision, and bounded reason codes.

- [ ] **Step 2: Implement conservative default orchestrator**

The default orchestrator must:

- validate trace, lease id, actor id, idempotency key, policy refs, and evidence refs;
- append a governance ledger record for each accepted phase;
- call existing admission, benchmark, release, and OS-code proposal adapter functions through their service-owned interfaces;
- return `Inconclusive`, `Denied`, `Rejected`, `RolledBack`, or `Unavailable` instead of fake success;
- never mutate Skill files or source files directly.

- [ ] **Step 3: Wire local provider**

Modify `local_provider.rs` to expose live tick and audit commands. Keep the provider as a built-in development provider; do not add application-specific branches.

- [ ] **Step 4: Run service tests**

Run:

```bash
cargo test -p macaca-autonomy-evolution
```

Expected: all autonomy evolution tests pass.

## Task 4: SDK And Runtime-Host Boundaries

- [ ] **Step 1: Add SDK methods**

Modify `autonomy_evolution_client.rs` to add:

- `run_live_tick(command: EvolutionLiveTickCommand)`
- `audit_live_run(command: EvolutionLiveAuditCommand)`

The unavailable client must return structured unavailable results with service id, command name, trace id, and bounded reason.

- [ ] **Step 2: Add runtime-host command decode**

Modify `autonomy_evolution_service_provider.rs` to decode the live tick and audit commands and forward them to the provider.

- [ ] **Step 3: Add SDK/runtime-host tests**

Add tests proving unavailable behavior and command decode do not fake success.

- [ ] **Step 4: Run targeted tests**

Run:

```bash
cargo test -p macaca-sdk autonomy_evolution_live
cargo test -p macaca-runtime-host autonomy_evolution_live
```

Expected: SDK and runtime-host live orchestrator tests pass.

## Task 5: Verification And Commit

- [ ] **Step 1: Validate OpenSpec**

Run:

```bash
openspec validate add-autonomy-evolution-live-orchestrator --strict
```

Expected: validation passes.

- [ ] **Step 2: Run Rust verification**

Run:

```bash
cargo test -p macaca-autonomy-evolution
cargo test -p macaca-sdk autonomy_evolution
cargo test -p macaca-runtime-host autonomy_evolution
```

Expected: targeted tests pass. Existing unrelated warnings may remain advisory.

- [ ] **Step 3: Run change detection**

Run:

```bash
npx gitnexus detect-changes
git diff --check
git status --short
```

Expected: changed scope is limited to the live orchestrator OpenSpec, autonomy evolution service, SDK facade, runtime-host adapter, and tests.

- [ ] **Step 4: Commit**

Run:

```bash
git add docs/superpowers/specs/2026-05-25-autonomy-evolution-live-orchestrator-design.md \
  docs/superpowers/plans/2026-05-25-autonomy-evolution-live-orchestrator.md \
  openspec/changes/add-autonomy-evolution-live-orchestrator \
  macaca/crates/services/macaca-autonomy-evolution \
  macaca/crates/facade/macaca-sdk \
  macaca/crates/runtime/macaca-runtime-host
git commit -m "feat: add autonomy evolution live orchestrator"
```

Expected: commit contains the OpenSpec, implementation, and tests for the live orchestrator slice.
