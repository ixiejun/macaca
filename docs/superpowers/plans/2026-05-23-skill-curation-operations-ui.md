# Skill Curation Operations UI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Expose Skill governance, curation dry-run, alias, and draft proposal state through a thin application operations UI.

**Architecture:** Web remains an Adapter over `SystemSkillClient`; Skill service owns all curation semantics. Frontend renders sanitized snapshots and never mutates skill files or implements lifecycle classification.

**Tech Stack:** Rust Axum, Macaca SDK Skill facade, Next.js React frontend, OpenSpec.

---

### Task 1: Specification

**Files:**
- Create: `openspec/changes/add-skill-curation-operations-ui/proposal.md`
- Create: `openspec/changes/add-skill-curation-operations-ui/design.md`
- Create: `openspec/changes/add-skill-curation-operations-ui/tasks.md`
- Create: `openspec/changes/add-skill-curation-operations-ui/specs/skill-governance-curation/spec.md`

- [ ] Add an OpenSpec change that defines the shell-only operations surface.
- [ ] Validate with `openspec validate add-skill-curation-operations-ui --strict`.

### Task 2: Web Adapter Route

**Files:**
- Create: `macaca/crates/shells/macaca-web/src/skill_operations_routes.rs`
- Modify: `macaca/crates/shells/macaca-web/src/bootstrap.rs`
- Modify: `macaca/crates/shells/macaca-web/src/lib.rs`

- [ ] Add `GET /api/apps/{app_id}/skills/operations`.
- [ ] Call `SystemSkillClient` snapshot methods with trace context.
- [ ] Log bounded counts and return sanitized aggregate JSON.

### Task 3: Frontend Operations Panel

**Files:**
- Create: `frontend/lib/skill-operations-types.ts`
- Create: `frontend/components/skills/SkillOperationsPanel.tsx`
- Modify: `frontend/lib/autonomy.ts`
- Modify: `frontend/components/autonomy/OperationsModeTabs.tsx`
- Modify: `frontend/components/autonomy/ApplicationOperationsDialog.tsx`

- [ ] Add typed frontend fetch helper.
- [ ] Add a Skill operations tab.
- [ ] Render governance records, curation recommendations, aliases, and proposals.

### Task 4: Verification

- [ ] Run `openspec validate add-skill-curation-operations-ui --strict`.
- [ ] Run `cd macaca && cargo check -p macaca-web`.
- [ ] Run focused Skill provider tests.
- [ ] Run `cd frontend && npm run lint && npm run build`.
- [ ] Run `git diff --check` and `gitnexus_detect_changes`.
