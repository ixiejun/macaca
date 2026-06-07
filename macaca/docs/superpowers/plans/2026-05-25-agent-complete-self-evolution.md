# Agent Complete Self-Evolution Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Upgrade Macaca from governed Skill-level self-evolution to a generic, auditable, rollback-capable agent self-evolution platform.

**Architecture:** Add a service-owned Autonomy Evolution Control Plane that orchestrates discovery, proposal generation, admission, evaluation, quarantine, canary, promotion, monitoring, and rollback through typed service commands. Existing Skill proposal/materialization/telemetry surfaces remain the first target adapter; OS-code evolution is introduced later as OpenSpec/Superpowers/GitNexus-governed proposals, not direct source mutation.

**Tech Stack:** Rust workspace under `macaca/`, OpenSpec, Superpowers planning, GitNexus impact analysis, Store/EventLog service contracts, Skill service, Task/Autonomy services, SDK/SystemFacade, Web/CLI thin adapters.

---

## Current Baseline

The current platform has proven:

- Real `/api/chat/v2` task completion can trigger `skill_self_evolution_observer`
  and create proposal evidence.
- Proposal-linked Skill packages can be materialized and made visible through
  registry/load-path.
- Skill usage telemetry can record activation, use, and successful task counts.
- Durable telemetry replay survives restart through a governance event journal.
- API-first audit can validate operations, registry/load-path, and observer
  evidence after restart.

The current platform has not yet proven:

- A fully autonomous policy loop for discover -> generate -> evaluate ->
  promote -> monitor -> roll back.
- Standard normalized metrics for quality and efficiency.
- Production Store/EventLog governance ledger semantics beyond local JSONL.
- Automatic paired benchmarks or canary release.
- Strong Skill Creator admission gates.
- OS-code self-evolution with OpenSpec/Superpowers/GitNexus release governance.

## Plan Decomposition

This is too large for one implementation change. Execute it as six OpenSpec
changes. Each change must keep files small, add English comments for non-obvious
logic, log key execution nodes, and avoid application-specific code.

Recommended change sequence:

1. `add-autonomy-evolution-control-plane`
2. `add-evolution-admission-quality-gates`
3. `add-normalized-evolution-benchmarking`
4. `add-evolution-release-safety-chain`
5. `serviceize-evolution-governance-ledger`
6. `add-os-code-evolution-proposal-adapter`

## Files And Ownership Map

Expected areas. Exact symbols must be confirmed by code reading and GitNexus
before edits.

- OpenSpec:
  - Create `openspec/changes/add-autonomy-evolution-control-plane/`
  - Create later change directories listed above.
- Proto/contracts:
  - Extend provider-neutral command/result DTOs in the appropriate
    `macaca/crates/foundation/` or service contract crate.
- Runtime host:
  - Add built-in provider/adapters under the runtime-host service provider
    composition area.
- Services:
  - Add or extend Autonomy, Skill, Evaluation, Store/EventLog, Policy,
    Entitlement, and Package Guard service contracts only through typed
    boundaries.
- SDK/SystemFacade:
  - Add focused clients and unavailable/null-object behavior.
- Web/CLI:
  - Add diagnostic or trigger adapters only; no classification, scoring,
    promotion, rollback, or semantic ownership.
- Tests:
  - Add unit tests for state machines, admission gates, metric scoring, replay,
    and rollback mementos.
  - Add boundary tests proving kernel/shell/service dependency rules.
  - Add live-chain runbooks and API-first audit checks.

## Task 1: Create OpenSpec For The Control Plane

**Files:**

- Create: `openspec/changes/add-autonomy-evolution-control-plane/proposal.md`
- Create: `openspec/changes/add-autonomy-evolution-control-plane/design.md`
- Create: `openspec/changes/add-autonomy-evolution-control-plane/tasks.md`
- Create: `openspec/changes/add-autonomy-evolution-control-plane/specs/autonomy-evolution-control-plane/spec.md`

- [ ] **Step 1: Read current code and specs**

Run:

```bash
openspec list
openspec list --specs
openspec show skill-governance-curation --type spec || true
openspec show autonomous-runtime --type spec || true
```

Expected: active changes and existing specs are visible. Missing spec ids are
acceptable because this repo currently keeps many capability specs under active
changes.

- [ ] **Step 2: Run GitNexus exploration before editing symbols**

Run GitNexus queries for:

```text
self evolution proposal materialization operator
skill governance usage telemetry replay
agent execution observer completion boundary
autonomy scheduler heartbeat service
Store EventLog governance journal
```

Expected: identify the concrete symbols that own current observer, proposal,
materialization, telemetry, and audit paths.

- [ ] **Step 3: Write the OpenSpec proposal**

`proposal.md` must state:

- Why Skill-level self-evolution is not enough.
- What changes: service-owned evolution run lifecycle, target adapter Strategy,
  typed commands, trace/policy/audit requirements, SDK unavailable behavior,
  thin diagnostics.
- Impacted areas: autonomy services, Skill service adapter, Store/EventLog,
  Evaluation service, Policy/Entitlement, SDK/SystemFacade, Web/CLI adapters.

- [ ] **Step 4: Write the OpenSpec design**

`design.md` must include:

- Ownership table matching the design doc.
- Lifecycle:

```text
Observed -> CandidateQueued -> CandidateClassified -> ProposalGenerated
  -> AdmissionReview -> Quarantined -> BenchmarkPrepared
  -> BaselineMeasured -> CandidateMeasured -> CanaryRunning
  -> Promoted -> ActiveMonitoring -> Superseded | RolledBack | Rejected | Inconclusive
```

- Design patterns: Command, Facade, Strategy, Decorator, State, Observer,
  Memento, Specification, Abstract Factory.
- Explicit non-goals: no kernel ownership, no Web/CLI semantics, no app-specific
  hardcoding, no raw payload observability.

- [ ] **Step 5: Write the delta spec**

The spec must add requirements for:

- Evolution run lifecycle state machine.
- Typed control-plane commands/results.
- Target adapter Strategy contract.
- Trace/policy/audit required before side effects.
- Replayable evidence refs and sanitized diagnostics.
- Structured unavailable behavior.

- [ ] **Step 6: Validate**

Run:

```bash
openspec validate add-autonomy-evolution-control-plane --strict
```

Expected: validation passes before implementation starts.

## Task 2: Implement The Minimal Control Plane Skeleton

**Files:** determine exact paths after Task 1 and GitNexus impact analysis.

- [ ] **Step 1: Write failing tests for lifecycle transitions**

Tests must cover:

- Valid forward transitions.
- Rejection from admission.
- Rollback from canary or active monitoring.
- Invalid transition denial.
- Every transition requiring trace id and policy decision id when it can mutate
  target state.

- [ ] **Step 2: Implement typed DTOs and state machine**

Implementation rules:

- Keep DTOs provider-neutral.
- Use explicit enums, not free-form strings.
- Add English comments explaining state ownership and why mutation is delegated
  to target adapters.
- Emit structured logs for transition start, policy decision, adapter call,
  evidence append, and terminal state.

- [ ] **Step 3: Add unavailable provider behavior**

SDK/SystemFacade must return structured unavailable results when the control
plane service is absent. Shells must render that result without fake success.

- [ ] **Step 4: Verify**

Run targeted unit tests, dependency-boundary tests, and:

```bash
openspec validate add-autonomy-evolution-control-plane --strict
git diff --check
```

## Task 3: Add Skill Target Adapter As The First Target Type

**Depends on:** Task 2.

- [ ] **Step 1: Run GitNexus impact analysis**

Targets must include current proposal processing, materialization operator,
semantic Skill identity, telemetry replay, and API-first audit symbols.

- [ ] **Step 2: Write adapter tests**

Tests must prove:

- The adapter invokes existing Skill service commands rather than rewriting
  Skill files directly.
- App/agent scope is preserved.
- Already-promoted or non-Draft proposals are not selected.
- Registry/load-path and usage telemetry evidence are read through service APIs.

- [ ] **Step 3: Implement adapter**

Use Strategy. The control plane sees a generic target adapter interface; the
Skill adapter delegates to existing service-owned Skill operations.

- [ ] **Step 4: Verify live chain**

Run a controlled live task through `/api/chat/v2`, then API-first audit. The
claim is only control-plane orchestration over the existing Skill path, not
normalized optimization yet.

## Task 4: Add Admission And Skill Creator Quality Gates

**OpenSpec change:** `add-evolution-admission-quality-gates`

- [ ] **Step 1: Create OpenSpec**

Requirements must cover:

- Semantic package naming gate.
- Trigger/frontmatter quality.
- `SKILL.md` body size and focus.
- Required resource structure checks.
- Quick validation command refs.
- Forward-test evidence refs.
- Duplicate/suppression decisions.
- Stale metadata regeneration.

- [ ] **Step 2: Implement executable Specifications**

Use the Specification pattern. Gates return structured `Accepted`, `Denied`,
`NeedsEvidence`, or `Quarantined` results with bounded reasons.

- [ ] **Step 3: Verify**

Add tests for meaningless `skill-exp-*` style candidates, good semantic names,
missing trigger quality, duplicate candidates, and sanitized denial reasons.

## Task 5: Add Normalized Evolution Benchmarking

**OpenSpec change:** `add-normalized-evolution-benchmarking`

- [ ] **Step 1: Define metric schema**

Metrics must include:

- token counts
- elapsed time
- tool calls and tool results
- retry counts
- failure recovery
- quality score
- human intervention rate
- policy decisions
- activation/use/success counters
- artifact refs
- regression reasons

- [ ] **Step 2: Add paired benchmark command**

The service must create baseline and candidate runs from provider-neutral task
family definitions. It must not hardcode application workflows.

- [ ] **Step 3: Add scoring Strategy**

The default Strategy must require quality preservation before efficiency gains
count. Inconclusive is a first-class result.

- [ ] **Step 4: Verify**

Re-run a Run 47-style paired workload and record whether the result is passed,
failed, or inconclusive. Do not use Run 51 as a normalized efficiency claim.

## Task 6: Add Quarantine, Canary, Promotion, And Rollback

**OpenSpec change:** `add-evolution-release-safety-chain`

- [ ] **Step 1: Define release states**

States must include `Quarantined`, `CanaryRunning`, `Promoted`,
`ActiveMonitoring`, `RolledBack`, `Superseded`, `Rejected`, and `Inconclusive`.

- [ ] **Step 2: Add policy gates**

Policy must evaluate capability diff, package ownership, tenant/app scope,
trust level, resource permissions, executable changes, and blast-radius score.

- [ ] **Step 3: Add rollback mementos**

Rollback must restore governance state and target state from replayable refs.
Tests must prove rollback after canary failure.

- [ ] **Step 4: Verify**

Run dry-run, canary-pass, canary-fail, rollback, and post-restart audit tests.

## Task 7: Serviceize The Governance Ledger

**OpenSpec change:** `serviceize-evolution-governance-ledger`

- [ ] **Step 1: Define Store/EventLog requirements**

Requirements must cover versioning, compaction, concurrency, migration,
cross-node replay, bounded snapshots, sanitized records, and read-model rebuild.

- [ ] **Step 2: Implement Store/EventLog Strategy**

Keep local JSONL as a development provider only. Add provider boundaries so a
production Store/EventLog backend can replace it without touching Skill,
Autonomy, Web, or CLI semantics.

- [ ] **Step 3: Verify**

Tests must cover replay after restart, malformed record skip, compaction,
schema-version migration, concurrent append ordering, and sanitized snapshots.

## Task 8: Add OS-Code Evolution Proposal Adapter

**OpenSpec change:** `add-os-code-evolution-proposal-adapter`

- [ ] **Step 1: Define non-mutating adapter contract**

The first adapter must produce governed code-change proposals only. It must not
edit source code until OpenSpec approval, GitNexus impact analysis, tests, and
release policy gates are satisfied.

- [ ] **Step 2: Integrate Superpowers/OpenSpec/GitNexus gates**

The adapter creates or updates OpenSpec artifacts, records Superpowers design
and plan refs, runs GitNexus impact analysis, and attaches blast-radius results.

- [ ] **Step 3: Add release path**

Only after approval may a separate executor implement code changes in small
slices with tests, boundary gates, commits, canary, and rollback.

- [ ] **Step 4: Verify**

Run an end-to-end non-mutating proposal for a low-risk internal improvement and
prove the output contains proposal/design/tasks, impact evidence, expected tests,
and a release-gate decision.

## Global Verification Gates

Every slice must pass:

```bash
git diff --check
openspec validate <change-id> --strict
```

Rust slices must add targeted `cargo test` commands from `macaca/` and include
the exact package/test names in the slice-specific plan.

Before committing any implementation slice:

- Run GitNexus impact analysis before editing symbols.
- Run `gitnexus_detect_changes()` before commit.
- Verify no kernel dependency on concrete providers.
- Verify no Web/CLI/frontend semantic ownership.
- Verify optional service absence returns structured unavailable behavior.
- Verify logs and snapshots are sanitized.

## First Milestone Definition Of Done

The first milestone is complete when:

- A real `/api/chat/v2` task creates an evolution run automatically.
- The control plane records lifecycle transitions with trace, policy, audit,
  app/tenant/session/task scope, and bounded evidence refs.
- The Skill target adapter routes through existing Skill service commands.
- API-first audit reconstructs observer, proposal, target-adapter, registry,
  telemetry, and control-plane state after restart.
- The verdict remains conservative: materialization and telemetry continuity can
  be claimed; normalized optimization waits for paired benchmarking.

## Final Definition Of Done

Macaca can claim complete agent self-evolution when:

- The autonomous loop discovers, proposes, evaluates, quarantines, canaries,
  promotes, monitors, and rolls back candidate improvements without manual
  operator invocation.
- Admission gates prevent low-quality or meaningless Skills from becoming
  active candidates.
- Paired benchmarks produce normalized pass/fail/inconclusive results.
- Store/EventLog is the durable governance source of truth.
- API-first audit reconstructs the full chain after restart.
- OS-code evolution is gated by OpenSpec, Superpowers, GitNexus impact analysis,
  tests, canary, and rollback before any source mutation is released.
