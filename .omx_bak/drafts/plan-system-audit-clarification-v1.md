# PRD Draft — Clarify Macaca System Definition and Refocus SYSTEM_AUDIT

## Requirements Summary

The repository currently has three conflicting documentation signals:
- `macaca/docs/SYSTEM_AUDIT.md` mixes architecture overview, module inventory, execution paths, technical debt, and refactoring recommendations in one artifact (`macaca/docs/SYSTEM_AUDIT.md:5-17`, `20-42`, `75-103`, `106-195`).
- `macaca/ARCHITECTURE-v2.md` already contains system-positioning and end-to-end flow material that is closer to a system-definition document than an audit (`macaca/ARCHITECTURE-v2.md:3-10`, `14-120`, `317-384`).
- `macaca/README.md` is stale and unrelated to the codebase, still describing a Go hello-world API (`macaca/README.md:1-84`), so first-time readers are misled before they reach the correct docs.

The clarified plan must produce a documentation baseline that supports the active architecture refactor stream instead of competing with it. This is especially important because an active OpenSpec change already exists for the same architecture risks (`openspec/changes/refactor-core-architecture/proposal.md:1-36`; `tasks.md:1-47`), while `openspec/project.md:1-27` is still a template and `openspec list --specs` currently returns no specs.

## Acceptance Criteria

1. A new canonical system-definition document exists under `macaca/docs/` and explains, for a first-time reader:
   - what Macaca is,
   - what problem it solves,
   - its target system qualities,
   - its core modules and responsibilities,
   - its full task-execution chain.
2. `macaca/docs/SYSTEM_AUDIT.md` is reduced to an execution-audit / refactor-action artifact and no longer carries primary responsibility for system definition.
3. Every top-tier audit recommendation in `SYSTEM_AUDIT.md` explicitly maps back to a system goal, design principle, or architecture invariant defined in the new system-definition doc.
4. `macaca/README.md` is updated from the stale Go hello-world content to a short repo entrypoint that routes readers to the right docs.
5. The documentation set makes the current implementation debt legible with concrete evidence, including at minimum:
   - oversized routing/orchestration surface (`macaca/docs/SYSTEM_AUDIT.md:31`, `112-115`; `macaca/crates/macaca-web/src/routes.rs:1-80`),
   - overloaded AppState (`macaca/docs/SYSTEM_AUDIT.md:113`; `macaca/crates/macaca-web/src/state.rs:56-114`),
   - real audit subsystem presence (`macaca/crates/macaca-kernel/src/audit.rs:1-120`),
   - current-vs-target request flow (`macaca/ARCHITECTURE-v2.md:317-384`).
6. The resulting doc structure is explicitly aligned with the active `openspec/changes/refactor-core-architecture/` workstream, either by reusing it or by documenting why doc clarification stays outside OpenSpec.

## RALPLAN-DR Summary

### Principles
1. **Separate definition from diagnosis** — system identity and current-state audit must not live in the same document.
2. **Documentation must point execution** — audit recommendations should derive from system goals, not float as isolated cleanup ideas.
3. **Prefer reuse over reinvention** — mine existing architecture material before writing new prose from scratch.
4. **First-reader clarity is a quality gate** — the top-level repo entrypoint and canonical docs must tell a coherent story.
5. **Keep refactor guidance traceable** — each major doc claim should link to code or existing design sources.

### Decision Drivers
1. The user wants `SYSTEM_AUDIT.md` to become a **refactor execution basis**, not a mixed narrative artifact.
2. The repo already contains source material for system definition (`ARCHITECTURE-v2.md`) and source material for implementation debt (`SYSTEM_AUDIT.md`), but they are not organized as a navigable doc system.
3. The current `README.md` is actively misleading, which blocks the first-reader success criteria.

### Viable Options

#### Option A — Keep a single expanded `SYSTEM_AUDIT.md`
**Approach:** Fold missing system-definition content into the existing audit document.
**Pros:** Fastest edit path; no extra filenames; low coordination overhead.
**Cons:** Preserves the mixed-responsibility problem; makes refactor recommendations harder to trace to enduring principles; contradicts the clarified user intent.

#### Option B — Two-layer docs only (`SYSTEM_OVERVIEW.md` + refocused `SYSTEM_AUDIT.md`)
**Approach:** Create a dedicated system-definition document and narrow `SYSTEM_AUDIT.md` to audit/action content.
**Pros:** Matches clarified intent; clean conceptual split; easiest handoff into refactor planning.
**Cons:** Still leaves the stale `README.md` as the first repo entrypoint unless separately updated; risks leaving `ARCHITECTURE-v2.md` in ambiguous status.

#### Option C — Three-surface doc baseline: README entrypoint + canonical system-definition doc + refocused `SYSTEM_AUDIT.md`
**Approach:**
- Rewrite `macaca/README.md` as a brief entrypoint,
- create `macaca/docs/SYSTEM_OVERVIEW.md` as the canonical “what/why/how” document,
- narrow `macaca/docs/SYSTEM_AUDIT.md` to current-state evidence, prioritized risks, and refactor actions,
- explicitly mark `ARCHITECTURE-v2.md` as source/reference material to mine or absorb.
**Pros:** Solves the first-reader problem, preserves the user's two-layer intent at the docs layer, reduces ambiguity across the full doc stack, and gives the refactor stream a stable narrative baseline.
**Cons:** Slightly broader doc touch set; requires editorial decisions about how much `ARCHITECTURE-v2.md` content to retain vs. duplicate.

### Recommendation
Recommend **Option C**. It is the only option that fixes both the clarified doc split and the repo-entrypoint mismatch without forcing readers to bypass a misleading README.

## Implementation Steps

### Step 1 — Establish the source-of-truth map
- Read and annotate the content boundaries across:
  - `macaca/docs/SYSTEM_AUDIT.md:5-17`, `20-42`, `75-103`, `106-195`
  - `macaca/ARCHITECTURE-v2.md:3-10`, `14-120`, `317-384`
  - `macaca/README.md:1-84`
  - `openspec/changes/refactor-core-architecture/proposal.md:1-36`
  - `openspec/changes/refactor-core-architecture/tasks.md:1-47`
- Produce a section-level mapping of which material belongs in:
  - README entrypoint,
  - system-definition doc,
  - execution-audit doc,
  - retained reference docs.

### Step 2 — Define the target doc architecture
Create the target structure and section contracts:
- `macaca/README.md` — short entrypoint, quick project definition, where to read next.
- `macaca/docs/SYSTEM_OVERVIEW.md` — canonical system definition, goals, design principles, module map, task-execution chain, target quality bar.
- `macaca/docs/SYSTEM_AUDIT.md` — current-state audit, prioritized issues, evidence, refactor recommendations, traceability back to overview principles.
- `macaca/ARCHITECTURE-v2.md` — either retained as deep-reference architecture appendix or marked as historical/source material after extracting canonical sections.

### Step 3 — Plan the content extraction and rewrite sequence
- Extract from `ARCHITECTURE-v2.md:3-10` the core system positioning.
- Extract from `ARCHITECTURE-v2.md:14-120` the module/system topology.
- Extract from `ARCHITECTURE-v2.md:317-384` the current-vs-target request flow and agent execution chain.
- Keep `SYSTEM_AUDIT.md:106-195` focused on prioritized debt and actions.
- Reframe `SYSTEM_AUDIT.md:5-17` and `75-103` so they become audit context or are moved into the system overview if they define the target system rather than current-state evidence.
- Replace the stale README with a minimal, accurate entrypoint that links to the new canonical documents.

### Step 4 — Align docs with refactor planning artifacts
- Update `.omx/plans/` with:
  - `prd-system-audit-clarification.md` as the approved work plan,
  - `test-spec-system-audit-clarification.md` as the verification matrix.
- Decide whether to amend `openspec/changes/refactor-core-architecture/` with documentation-oriented tasks or record an explicit note that documentation clarification is a prerequisite planning artifact outside OpenSpec.
- Default recommendation: **reuse the existing `refactor-core-architecture` change rather than creating a second overlapping architecture proposal**, unless maintainers want documentation approval separated from implementation approval.

### Step 5 — Add traceability from audit findings to system goals
For each P0/P1 recommendation in `SYSTEM_AUDIT.md:108-195`, add a reference to one of the system-definition anchors, such as:
- maintainability / bounded module responsibility,
- configurable entry agents instead of hardcoded coordinator assumptions,
- consistent execution-chain clarity,
- pluggable capability model.

### Step 6 — Verify the doc system against first-reader and executor use-cases
- Run a newcomer-read test: can a reader answer the five user-defined questions from the docs alone?
- Run an executor-read test: can a refactor implementer identify why each high-priority recommendation exists and what system goal it protects?
- Confirm README → overview → audit navigation works without needing hidden context from old docs.

## Risks and Mitigations

- **Risk:** `SYSTEM_OVERVIEW.md` duplicates too much of `ARCHITECTURE-v2.md`.
  - **Mitigation:** Define the overview as canonical, concise, and reader-oriented; treat `ARCHITECTURE-v2.md` as deep technical reference or source material.
- **Risk:** Documentation work drifts into speculative architecture redesign.
  - **Mitigation:** Keep the overview grounded in current intended architecture already evidenced in `ARCHITECTURE-v2.md` and active OpenSpec work, not new unapproved system behavior.
- **Risk:** Audit recommendations remain disconnected from goals.
  - **Mitigation:** Require explicit cross-links from each P0/P1 audit item to a system goal/principle anchor in the overview.
- **Risk:** README is updated but still too detailed or too vague.
  - **Mitigation:** Cap README to entrypoint scope; push depth into `SYSTEM_OVERVIEW.md`.
- **Risk:** Overlapping OpenSpec artifacts cause planning fragmentation.
  - **Mitigation:** Reuse `openspec/changes/refactor-core-architecture/` unless there is a strong approval-boundary reason not to.

## Verification Steps

1. **Document structure check**
   - Confirm the planned file set exists in the plan: `macaca/README.md`, `macaca/docs/SYSTEM_OVERVIEW.md`, `macaca/docs/SYSTEM_AUDIT.md`, `.omx/plans/prd-system-audit-clarification.md`, `.omx/plans/test-spec-system-audit-clarification.md`.
2. **Question-answerability check**
   - Verify the final overview/audit pair can answer:
     - What project is this?
     - What problem does it solve?
     - What is the detailed system structure?
     - What does each module do?
     - What is the full task execution chain?
3. **Traceability check**
   - Verify each top-tier audit action cites a system-goal anchor or principle.
4. **Evidence check**
   - Ensure major doc claims still point back to source materials or code locations, especially `SYSTEM_AUDIT.md`, `ARCHITECTURE-v2.md`, `state.rs`, `routes.rs`, and `audit.rs`.
5. **OpenSpec alignment check**
   - Confirm the plan explicitly states whether the documentation clarification is absorbed into `refactor-core-architecture` or intentionally left as a prerequisite artifact.

## ADR

### Decision
Adopt a **three-surface documentation baseline**: an accurate README entrypoint, a canonical system-definition doc (`macaca/docs/SYSTEM_OVERVIEW.md`), and a refocused execution-audit doc (`macaca/docs/SYSTEM_AUDIT.md`).

### Drivers
- Mixed responsibilities in `SYSTEM_AUDIT.md` prevent it from serving cleanly as a refactor execution basis.
- Existing architecture material already contains the needed system-definition content.
- The stale README blocks first-reader comprehension.
- Active architecture refactor work benefits from a clarified, non-conflicting doc baseline.

### Alternatives Considered
- Keep everything in `SYSTEM_AUDIT.md`.
- Use only two docs and ignore README.
- Create a new standalone OpenSpec change for docs.

### Why Chosen
This option preserves the user-requested split, fixes the repo entrypoint, and minimizes future drift by separating enduring system definition from per-iteration audit/refactor content.

### Consequences
- More than two files are touched, but only two carry the main semantic weight.
- `ARCHITECTURE-v2.md` needs an explicit status decision.
- Future refactor proposals can point to stable system-definition anchors instead of repeating intent.

### Follow-ups
- Decide whether `ARCHITECTURE-v2.md` becomes appendix/reference or is partially absorbed.
- Decide whether doc tasks should be added into `openspec/changes/refactor-core-architecture/tasks.md`.
- If execution proceeds, perform doc edits before or alongside code refactor planning, not after major code churn.

## Available-Agent-Types Roster
- `planner` — planning structure and sequencing
- `architect` — architecture/doc-boundary review
- `critic` — plan quality challenge
- `writer` — doc drafting and restructuring
- `explore` / `explorer` — fast repo fact gathering
- `verifier` — acceptance/traceability checks

## Follow-up Staffing Guidance

### If handed to `$ralph`
- `writer` (high): own README + `SYSTEM_OVERVIEW.md` + `SYSTEM_AUDIT.md` rewrite sequence.
- `architect` (high): validate section boundaries and avoid speculative redesign.
- `verifier` (high): run newcomer-question and traceability checks before declaring complete.

### If handed to `$team`
- Lane 1 — `explore` (low): mine exact source sections from `ARCHITECTURE-v2.md`, `SYSTEM_AUDIT.md`, and OpenSpec artifacts.
- Lane 2 — `writer` (high): draft `SYSTEM_OVERVIEW.md` and README entrypoint.
- Lane 3 — `writer` or `executor` (medium/high): refocus `SYSTEM_AUDIT.md` around prioritized actions and principle links.
- Lane 4 — `verifier` (high): validate answerability, cross-links, and OpenSpec alignment.
- Team verification path: team proves doc structure, source mapping, and cross-links; Ralph (or verifier lane) proves newcomer-question coverage and that audit actions trace to overview principles.

## Launch Hints
- Ralph path: `$ralph .omx/plans/prd-system-audit-clarification.md`
- Team path: `$team .omx/plans/prd-system-audit-clarification.md`
- OMX CLI equivalent: `omx team .omx/plans/prd-system-audit-clarification.md`
