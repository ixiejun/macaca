# PRD Draft — Clarify Macaca System Definition and Refocus SYSTEM_AUDIT

## Requirements Summary

The repository currently presents inconsistent documentation about what Macaca is and how its refactor work should be understood:
- `macaca/docs/SYSTEM_AUDIT.md` mixes system overview, execution-path description, module inventory, technical debt, and refactor actions in one artifact (`macaca/docs/SYSTEM_AUDIT.md:5-17`, `20-42`, `75-103`, `106-195`).
- `macaca/ARCHITECTURE-v2.md` already contains useful system-definition material, but it also blends stable description, target-state prescriptions, planned features, and fix guidance (`macaca/ARCHITECTURE-v2.md:3-10`, `14-108`, `317-357`, `406-489`, `555-597`).
- `macaca/README.md` is stale and actively misleading, still describing an unrelated Go hello-world API (`macaca/README.md:1-84`).
- The active OpenSpec workstream `openspec/changes/refactor-core-architecture/` depends on the audit’s architecture findings (`proposal.md:1-36`, `tasks.md:1-47`), but this clarification task itself is still a documentation/planning baseline rather than a behavior-changing implementation.

The plan therefore needs to create a canonical doc stack that makes `SYSTEM_AUDIT.md` usable as a refactor execution basis while giving first-time readers an accurate, stable definition of the system.

## Acceptance Criteria

1. A new canonical system-definition document exists at `macaca/docs/SYSTEM_OVERVIEW.md` and contains clearly labeled sections for:
   - project identity,
   - problem statement,
   - target system qualities,
   - core design principles,
   - module map and responsibilities,
   - task execution chain,
   - current vs intended vs planned boundaries.
2. `macaca/docs/SYSTEM_AUDIT.md` is explicitly scoped to current-state evidence, prioritized risks, and refactor actions, and no longer serves as the primary source of system identity.
3. `macaca/README.md` is reduced to repo-entrypoint scope and links readers to `macaca/docs/SYSTEM_OVERVIEW.md` and `macaca/docs/SYSTEM_AUDIT.md`.
4. `macaca/ARCHITECTURE-v2.md` is given an explicit non-canonical status (deep reference / source draft), so it no longer competes with the overview as a canonical system-definition surface.
5. Every P0/P1 audit recommendation in `macaca/docs/SYSTEM_AUDIT.md:108-195` maps to at least one goal, principle, or invariant defined in `macaca/docs/SYSTEM_OVERVIEW.md`.
6. The governance decision is explicit and singular: **this clarification task remains outside OpenSpec** as a prerequisite planning/documentation artifact; no OpenSpec files are modified in this lane.
7. The deliverables include two reproducible matrices:
   - **Reader-answer matrix:** the 5 required user questions → exact README/overview/audit sections or anchors.
   - **Audit-traceability matrix:** each P0/P1 audit item → exact overview principle/invariant anchor.
8. A first-time reader can answer the user’s five required questions using the final docs alone:
   - What project is this?
   - What problem does it solve?
   - What is the system structure?
   - What does each module do?
   - What is the full task-execution chain?

## RALPLAN-DR Summary

### Principles
1. **Separate stable intent from current-state diagnosis.**
2. **Keep one canonical owner per doc surface.**
3. **Ground every refactor action in a system goal or invariant.**
4. **Treat first-reader clarity as a release-quality concern.**
5. **Minimize drift by reusing proven source material and labeling non-canonical references.**

### Decision Drivers
1. The user explicitly wants `SYSTEM_AUDIT.md` to become a refactor execution basis, not a mixed-purpose narrative artifact.
2. Existing docs already contain much of the needed system-definition material, but it is fragmented and semantically mixed.
3. The stale README blocks the requested “new reader can explain the system” success criterion.

### Viable Options

#### Option A — Canonicalize a trimmed `ARCHITECTURE-v2.md`
**Approach:** Fix README, heavily rewrite `ARCHITECTURE-v2.md` into the canonical system-definition doc, and narrow `SYSTEM_AUDIT.md`.
**Pros:** Lowest doc-count increase; reduces one potential duplication surface.
**Cons:** `ARCHITECTURE-v2.md` currently mixes overview, target-state prescriptions, planned features, and repair guidance (`macaca/ARCHITECTURE-v2.md:317-357`, `406-489`, `555-597`), so making it canonical requires a major semantic rewrite before it can safely anchor execution.

#### Option B — Two-layer docs only (`SYSTEM_OVERVIEW.md` + `SYSTEM_AUDIT.md`)
**Approach:** Create a new canonical overview doc and narrow the audit, but leave README and governance alignment for later.
**Pros:** Matches the clarified two-layer requirement; clean conceptual split.
**Cons:** Leaves the misleading repo entrypoint unresolved and keeps governance alignment implicit.

#### Option C — Canonical doc stack: README entrypoint + `SYSTEM_OVERVIEW.md` + refocused `SYSTEM_AUDIT.md`
**Approach:**
- convert `macaca/README.md` into a short, accurate entrypoint,
- create `macaca/docs/SYSTEM_OVERVIEW.md` as the canonical concise contract,
- refocus `macaca/docs/SYSTEM_AUDIT.md` on current-state evidence and refactor actions,
- explicitly demote `macaca/ARCHITECTURE-v2.md` to non-canonical deep reference/source draft,
- keep this clarification lane outside OpenSpec, but require future implementation work under `refactor-core-architecture` to cite the finalized docs before code changes begin.
**Pros:** Solves first-reader clarity, preserves the user’s requested two-layer docs, locks doc ownership boundaries, and removes governance ambiguity from this lane.
**Cons:** Requires touching one extra surface (README) and enforcing stronger anti-drift rules across all docs.

### Recommendation
Recommend **Option C**.

### Invalidation Rationale for Rejected Alternatives
- **Option A invalidated** because `ARCHITECTURE-v2.md` is currently too semantically mixed to serve as a stable canonical contract without a more disruptive rewrite than this clarification task needs.
- **Option B invalidated** because it leaves the stale README unresolved and still leaves executors guessing about governance alignment.

## Document Boundary Contract

| Surface | Primary Audience | Canonical Scope | Forbidden Content | Update Trigger |
|---|---|---|---|---|
| `macaca/README.md` | First-time repo visitor | One-screen project definition, quick navigation, where to read next | Deep architecture detail, full audit findings, speculative roadmap | Any change to canonical doc names or project one-line identity |
| `macaca/docs/SYSTEM_OVERVIEW.md` | Maintainers, contributors, reviewers | What Macaca is, what it solves, target qualities, module map, task-execution chain, current/intended/planned boundaries | Issue backlog, fix checklist, implementation-level debt ranking | Any material change to system goals, module responsibilities, or execution model |
| `macaca/docs/SYSTEM_AUDIT.md` | Refactor implementers | Current-state evidence, prioritized risks, rationale, action recommendations linked to overview principles | Primary system identity narrative, roadmap prose detached from evidence, unlinked cleanup ideas | New audit findings, reprioritization, or accepted refactor recommendations |
| `macaca/ARCHITECTURE-v2.md` | Deep technical readers | Supporting reference and source material mined from earlier architecture thinking | Competing canonical definition of project identity | Only when preserving or archiving useful detail not promoted into canonical docs |

## Explicit Status Decision for `ARCHITECTURE-v2.md`

`macaca/ARCHITECTURE-v2.md` should remain in the repository as a **non-canonical deep reference / source draft**, not as the primary system-definition document. The final docs pass should add a short banner or opening note clarifying that `SYSTEM_OVERVIEW.md` is canonical for system identity and execution-chain understanding.

## Governance Decision

This clarification task is **outside OpenSpec**.

Rationale:
- The work in this lane is a documentation/planning baseline, not a direct behavior-changing implementation.
- The active `openspec/changes/refactor-core-architecture/` change is code-refactor-oriented and should consume the clarified docs as input rather than be edited speculatively in the same lane.

Implication:
- **Pass condition for this lane:** no OpenSpec files are modified; the plan and docs land under `.omx/plans/` and `macaca/docs/`.
- **Required future handoff condition before architecture implementation:** any later implementation run under `refactor-core-architecture` must (1) update the relevant OpenSpec proposal/tasks to cite the finalized docs, (2) run `openspec validate refactor-core-architecture --strict`, and (3) obtain or confirm proposal approval before any code changes start.

## Anti-Drift Rules

1. `README.md` must stay under entrypoint scope; if content exceeds one-screen orientation, move it to `SYSTEM_OVERVIEW.md`.
2. `SYSTEM_OVERVIEW.md` may state **current**, **intended**, and **planned** behavior only when each statement is explicitly labeled.
3. `SYSTEM_AUDIT.md` may reference goals from the overview, but it must not redefine the project mission or architecture contract.
4. `ARCHITECTURE-v2.md` must not be updated as if it were canonical; any new canonical statements belong in `SYSTEM_OVERVIEW.md` first.
5. Every newly added top-tier audit action must include a link or anchor reference to the overview principle it protects.

## Implementation Steps

### Step 1 — Build the source map and extraction ledger
Use these materials as the evidence base:
- `macaca/docs/SYSTEM_AUDIT.md:5-17`, `20-42`, `75-103`, `106-195`
- `macaca/ARCHITECTURE-v2.md:3-10`, `14-108`, `317-357`, `406-489`, `555-597`
- `macaca/README.md:1-84`
- `macaca/crates/macaca-web/src/state.rs:56-114`
- `macaca/crates/macaca-web/src/routes.rs:1-80`
- `macaca/crates/macaca-kernel/src/audit.rs:1-120`

Create a section ledger that marks each source segment as one of:
- README seed,
- overview seed,
- audit seed,
- retained deep-reference only.

### Step 2 — Draft the canonical system-definition contract
Plan `macaca/docs/SYSTEM_OVERVIEW.md` with required sections:
1. What Macaca is
2. What problem it solves
3. Target system qualities
4. Core design principles
5. Module map
6. Task execution chain
7. Current vs intended vs planned boundaries
8. Links to deeper references and audit

Source the content primarily from `ARCHITECTURE-v2.md:3-10`, `14-108`, and `317-357`, but rewrite it into stable, reader-oriented language.

### Step 3 — Refocus `SYSTEM_AUDIT.md`
Retain and reorganize the current audit around:
- current-state evidence,
- prioritized risks,
- actionable refactor recommendations,
- traceability back to overview goals.

Specifically:
- move or rewrite overview-like content from `macaca/docs/SYSTEM_AUDIT.md:5-17` and `75-103` if it belongs to the system-definition layer,
- keep debt/action content centered on `macaca/docs/SYSTEM_AUDIT.md:106-195`,
- ensure each P0/P1 item points to a named principle or invariant in `SYSTEM_OVERVIEW.md`.

### Step 4 — Repair the repo entrypoint
Replace the stale `macaca/README.md:1-84` Go hello-world narrative with:
- a one-paragraph definition of Macaca,
- links to `docs/SYSTEM_OVERVIEW.md` and `docs/SYSTEM_AUDIT.md`,
- a short note on where deeper architecture detail lives.

### Step 5 — Encode verification into the artifact set
Write:
- `.omx/plans/prd-system-audit-clarification.md` as the approved plan,
- `.omx/plans/test-spec-system-audit-clarification.md` as the concrete verification matrix.

The test-spec must include two required appendices:
1. **Reader-answer matrix** — 5 user questions → exact doc sections/anchors.
2. **Audit-traceability matrix** — each P0/P1 audit item → exact overview principle/invariant anchor.

## Risks and Mitigations

- **Risk:** `SYSTEM_OVERVIEW.md` duplicates `ARCHITECTURE-v2.md` and creates competing canon.
  - **Mitigation:** explicitly demote `ARCHITECTURE-v2.md`; keep the overview concise and canonical; add a banner in `ARCHITECTURE-v2.md`.
- **Risk:** overview text becomes speculative rather than grounded.
  - **Mitigation:** separate current/intended/planned statements and source them from existing docs or code-backed audit evidence.
- **Risk:** audit recommendations remain contextless cleanup bullets.
  - **Mitigation:** require a principle/invariant link for every P0/P1 recommendation.
- **Risk:** README regresses into a second overview.
  - **Mitigation:** enforce README entrypoint-only rule and verify length/scope during review.
- **Risk:** future implementation starts without aligning the existing OpenSpec change to the clarified docs.
  - **Mitigation:** make OpenSpec update + strict validation + proposal approval confirmation a pre-implementation gate for later architecture execution lanes, not for this clarification lane.

## Verification Steps

1. **Required sections check**
   - Verify `SYSTEM_OVERVIEW.md` contains all eight required sections from Step 2.
   - Verify `SYSTEM_AUDIT.md` opens with an explicit scope statement that it is an execution-audit/refactor-action document.
   - Verify `README.md` links to the overview and audit.
   - Verify `ARCHITECTURE-v2.md` has a non-canonical banner or opening note.
2. **Reader-answer matrix check**
   - Verify the test-spec contains a written matrix mapping all five required reader questions to exact README/overview/audit sections or anchors.
3. **Audit-traceability matrix check**
   - Verify the test-spec contains a written matrix mapping every P0/P1 audit item to at least one overview principle/invariant anchor.
4. **Current/intended/planned labeling check**
   - Verify every target-state or planned statement in the overview is explicitly labeled, and no unlabeled speculative claims remain.
5. **Evidence check**
   - Verify key claims about current implementation still trace to source docs or code (`state.rs`, `routes.rs`, `audit.rs`) where applicable.
6. **Governance-path check**
   - Verify the PRD explicitly states this lane is outside OpenSpec.
   - Verify the PRD explicitly states that any later implementation under `refactor-core-architecture` must update relevant OpenSpec artifacts, run `openspec validate refactor-core-architecture --strict`, and obtain/confirm proposal approval before code edits.

## ADR

### Decision
Adopt a canonical doc stack consisting of:
- `macaca/README.md` as entrypoint,
- `macaca/docs/SYSTEM_OVERVIEW.md` as canonical system-definition contract,
- `macaca/docs/SYSTEM_AUDIT.md` as execution-audit/refactor-action artifact,
- `macaca/ARCHITECTURE-v2.md` as non-canonical deep reference.

### Drivers
- The user-requested two-layer doc split.
- The need to make `SYSTEM_AUDIT.md` usable as a refactor execution basis.
- The stale README problem.
- The need to remove governance ambiguity from the clarification lane.

### Alternatives Considered
- Canonicalize a heavily rewritten `ARCHITECTURE-v2.md`.
- Split only overview/audit and defer README/governance work.
- Edit the active OpenSpec change directly during this documentation clarification lane.

### Why Chosen
This option is the smallest change set that resolves reader clarity, doc ownership boundaries, execution traceability, and governance ambiguity at the same time.

### Consequences
- A new canonical doc is introduced.
- `ARCHITECTURE-v2.md` must carry explicit non-canonical status.
- Future refactor execution must explicitly absorb the clarified docs into the existing OpenSpec change, pass strict validation, and confirm proposal approval before code edits begin.

### Follow-ups
- Use the finalized docs as prerequisites for later `refactor-core-architecture` implementation planning.
- Decide whether to trim or archive sections of `ARCHITECTURE-v2.md` after canonical docs land.
- Use the overview anchors when future refactor plans justify changes.

## Available-Agent-Types Roster
- `planner`
- `architect`
- `critic`
- `writer`
- `explore` / `explorer`
- `verifier`

## Follow-up Staffing Guidance

### Ralph path
- `writer` (high): write README, `SYSTEM_OVERVIEW.md`, and audit restructuring.
- `architect` (high): enforce doc-boundary contract and current/intended/planned labeling.
- `verifier` (high): run reader-answer, traceability, anti-drift, and governance-path checks.

### Team path
- Lane 1 — `explore` (low): produce extraction ledger from source docs/code.
- Lane 2 — `writer` (high): draft `SYSTEM_OVERVIEW.md` and README entrypoint.
- Lane 3 — `writer` or `executor` (medium/high): refocus `SYSTEM_AUDIT.md` and add overview-principle links.
- Lane 4 — `verifier` (high): verify sections, labels, matrices, and governance-path statements.
- Team verification path: team proves file contents and matrix completeness; verifier/Ralph proves the five reader questions and audit-to-principle traceability.

## Launch Hints
- Ralph path: `$ralph .omx/plans/prd-system-audit-clarification.md`
- Team path: `$team .omx/plans/prd-system-audit-clarification.md`
- OMX CLI path: `omx team .omx/plans/prd-system-audit-clarification.md`
