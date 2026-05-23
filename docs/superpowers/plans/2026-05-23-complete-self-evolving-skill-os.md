# Complete Self-Evolving Skill OS Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development or superpowers:executing-plans to
> implement this plan task-by-task. Keep OpenSpec, implementation, tests, and
> audit evidence aligned at every slice.

**Goal:** Complete the unimplemented parts of
`docs/macaca-agent-self-evolving-skills-research.md` while preserving Macaca OS
microkernel and serviceization boundaries.

**Architecture:** Skill service owns evolution and curation; Store/EventLog owns
durability; Task, Memory, Knowledge, Context, Policy, Entitlement, and Scheduler
integrate through service calls; runtime-host wires providers; SDK exposes
facades; shells render and submit commands only.

---

### Task 1: OpenSpec Contract

**Files:**
- `openspec/changes/complete-self-evolving-skill-os/proposal.md`
- `openspec/changes/complete-self-evolving-skill-os/design.md`
- `openspec/changes/complete-self-evolving-skill-os/tasks.md`
- `openspec/changes/complete-self-evolving-skill-os/specs/skill-governance-curation/spec.md`

- [x] **Step 1: Define the complete remaining capability contract**

Create the umbrella proposal, detailed design, granular tasks, and delta spec
for the remaining self-evolving skill work.

- [x] **Step 2: Validate the OpenSpec change**

Run:

```bash
openspec validate complete-self-evolving-skill-os --strict
```

Expected: validation succeeds.

### Task 2: Implementation Slices

Follow the OpenSpec `tasks.md` sequence:

1. Durable Skill Governance Store.
2. Lifecycle state machine.
3. Rich provenance and telemetry.
4. Task completion experience extraction.
5. Proposal lifecycle commands.
6. Safe skill content mutation.
7. Curation status/run/snapshot/rollback.
8. Optional semantic review provider.
9. Umbrella merge and support-file demotion.
10. Alias resolution across consumers.
11. Context Composer integration.
12. Package, Store, Entitlement, and ownership policy.
13. Operations UI and CLI mutation adapters.
14. Boundary, security, audit, and sanitization gates.
15. Documentation and operator runbooks.

### Task 3: Completion Gates Per Slice

Run the relevant subset for each slice. These gates are intentionally modeled as
release checklist evidence rather than product logic, so runtime services remain
generic and shells continue to act as thin adapters.

```bash
openspec validate complete-self-evolving-skill-os --strict
cargo test -p macaca-runtime-host skill_service -- --nocapture
cargo check -p macaca-skill -p macaca-sdk -p macaca-runtime-host
npm run lint
npm run build
git diff --check
```

Use the frontend lint/build commands only when the slice touches Web or frontend
surfaces. Use the most focused cargo test target that proves the touched
service, provider, or boundary, then run the shared cargo check before commit.
Also run GitNexus `detect_changes` with staged scope before each commit and
record any HIGH/CRITICAL impact warnings before proceeding.
