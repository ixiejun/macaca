# Test Spec — system-audit-clarification

## Purpose

Verify that the documentation clarification work produces a usable, low-drift doc system for Macaca without mixing system identity, audit evidence, and governance responsibilities.

## Artifacts Under Test
- `macaca/README.md`
- `macaca/docs/SYSTEM_OVERVIEW.md`
- `macaca/docs/SYSTEM_AUDIT.md`
- `macaca/ARCHITECTURE-v2.md`
- `.omx/plans/prd-system-audit-clarification.md`

## Acceptance Coverage

| ID | Requirement | Verification Method | Pass Rule |
|---|---|---|---|
| AC1 | `SYSTEM_OVERVIEW.md` contains the required canonical sections | Section checklist review | All 8 required sections exist with explicit headings |
| AC2 | `SYSTEM_AUDIT.md` is execution-audit scoped | Opening-scope review | Intro explicitly says it is current-state audit / refactor-action, not primary system-definition doc |
| AC3 | README is entrypoint-only and links correctly | README review + link check | README gives a short accurate definition and links to overview + audit |
| AC4 | `ARCHITECTURE-v2.md` is non-canonical | Banner/opening-note review | File contains a clear note pointing readers to `SYSTEM_OVERVIEW.md` as canonical |
| AC5 | P0/P1 audit items map to overview principles | Audit-traceability matrix | Every P0/P1 row has at least one exact overview anchor reference |
| AC6 | Governance path is explicit | PRD review | PRD states this lane is outside OpenSpec and names the later pre-code gate |
| AC7 | New reader can answer the 5 user questions | Reader-answer matrix + reviewer check | Every question maps to exact doc anchors and reviewer confirms adequacy |
| AC8 | Current/intended/planned statements are labeled | Overview review | No unlabeled speculative target-state claims remain |

## Required Section Checklist for `SYSTEM_OVERVIEW.md`

1. What Macaca is
2. What problem it solves
3. Target system qualities
4. Core design principles
5. Module map
6. Task execution chain
7. Current vs intended vs planned boundaries
8. Links to deeper references and audit

**Pass rule:** all eight headings or clearly equivalent headings must exist.

## Reader-Answer Matrix (must be filled during execution)

| User Question | Exact Doc Section / Anchor | Why this section answers it | Reviewer Initials | Pass/Fail |
|---|---|---|---|---|
| What project is this? | `README.md#macaca`; `SYSTEM_OVERVIEW.md#what-macaca-is` | README gives the entrypoint definition; overview provides the canonical system identity | AI | PASS |
| What problem does it solve? | `SYSTEM_OVERVIEW.md#what-problem-it-solves` | This section states the system-level problem Macaca addresses | AI | PASS |
| What is the system structure? | `SYSTEM_OVERVIEW.md#module-map` | The module map defines the major subsystems and their roles | AI | PASS |
| What does each module do? | `SYSTEM_OVERVIEW.md#module-map` | The table maps each module to its system role and responsibility | AI | PASS |
| What is the full task-execution chain? | `SYSTEM_OVERVIEW.md#task-execution-chain` | This section describes current and intended execution paths end-to-end | AI | PASS |

**Pass rule:** every row contains an exact section/anchor, not just a filename.

## Audit-Traceability Matrix (must be filled during execution)

| Audit Item (P0/P1) | Audit Section / Anchor | Overview Principle / Invariant Anchor | Rationale | Pass/Fail |
|---|---|---|---|---|
| routes.rs oversized routing/orchestration surface | `SYSTEM_AUDIT.md#p0-risks` | `SYSTEM_OVERVIEW.md#principle-1-bounded-module-responsibility`; `SYSTEM_OVERVIEW.md#principle-3-observable-end-to-end-execution` | Huge routing/orchestration files violate bounded responsibilities and weaken execution observability | PASS |
| AppState God Object | `SYSTEM_AUDIT.md#p0-risks` | `SYSTEM_OVERVIEW.md#principle-1-bounded-module-responsibility` | A God Object collapses multiple boundaries into one state holder | PASS |
| hardcoded coordinator assumptions | `SYSTEM_AUDIT.md#p0-risks` | `SYSTEM_OVERVIEW.md#principle-2-config-driven-entry-and-orchestration` | Entry orchestration should come from config/manifest, not hardcoded names | PASS |
| TaskId duplicate definitions | `SYSTEM_AUDIT.md#p1-duplication-and-redundancy` | `SYSTEM_OVERVIEW.md#principle-4-shared-protocol-and-task-primitives` | Shared protocol types should not fork across crates | PASS |
| DelegatedTask duplicate definitions | `SYSTEM_AUDIT.md#p1-duplication-and-redundancy` | `SYSTEM_OVERVIEW.md#principle-4-shared-protocol-and-task-primitives` | Delegation semantics should reuse one task primitive | PASS |
| AgenticLoop repeated logic | `SYSTEM_AUDIT.md#p1-duplication-and-redundancy` | `SYSTEM_OVERVIEW.md#principle-3-observable-end-to-end-execution`; `SYSTEM_OVERVIEW.md#principle-4-shared-protocol-and-task-primitives` | Repeated loop variants increase drift in execution semantics and observability hooks | PASS |
| TaskTracker possible dead code | `SYSTEM_AUDIT.md#p1-duplication-and-redundancy` | `SYSTEM_OVERVIEW.md#principle-1-bounded-module-responsibility` | Stale task abstractions blur the task-system boundary | PASS |
| TaskQueue and ExecutionQueue overlap | `SYSTEM_AUDIT.md#p1-duplication-and-redundancy` | `SYSTEM_OVERVIEW.md#principle-4-shared-protocol-and-task-primitives` | Overlapping queue systems fragment shared execution primitives | PASS |

**Pass rule:** each P0/P1 row links to at least one exact overview anchor.

## Governance-Path Gate

This documentation clarification lane is outside OpenSpec.

Before any later code implementation under `openspec/changes/refactor-core-architecture/`, the execution owner must:
1. update the relevant OpenSpec proposal/tasks to cite the finalized docs,
2. run `openspec validate refactor-core-architecture --strict`, and
3. obtain or confirm proposal approval before code edits start.

**Pass rule:** the later implementation lane must satisfy all three conditions before editing source code.

## Verification Procedure

1. Review README against AC3.
2. Review `SYSTEM_OVERVIEW.md` against AC1 and AC8.
3. Review `SYSTEM_AUDIT.md` against AC2 and AC5.
4. Review `ARCHITECTURE-v2.md` against AC4.
5. Fill and review the Reader-Answer Matrix for AC7.
6. Fill and review the Audit-Traceability Matrix for AC5.
7. Review PRD against AC6 and governance gate requirements.

## Exit Criteria

The clarification work is only complete when:
- all AC1–AC8 pass,
- both matrices are fully filled with exact anchors,
- no doc surface violates the boundary contract, and
- the governance-path gate is explicitly preserved for the later architecture implementation lane.
