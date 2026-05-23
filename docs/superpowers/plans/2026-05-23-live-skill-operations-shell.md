# Live Skill Operations Shell Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make CLI and frontend Skill operations prove the same live `service.skill` self-evolution path that Web/API already exercises.

**Architecture:** CLI remains a terminal shell by using an HTTP Adapter over the Web API facade when an app id is supplied, while retaining the SDK Null Object diagnostic path when no live target is requested. Frontend preserves mutation trace visibility after automatic refresh.

**Tech Stack:** Rust, Clap, reqwest, Axum Web API, Next.js/React, OpenSpec.

---

### Task 1: Document The Shell Contract

**Files:**
- Create: `openspec/changes/fix-live-skill-operations-shell/proposal.md`
- Create: `openspec/changes/fix-live-skill-operations-shell/design.md`
- Create: `openspec/changes/fix-live-skill-operations-shell/tasks.md`
- Create: `openspec/changes/fix-live-skill-operations-shell/specs/web-cli-thin-shell-completion/spec.md`
- Create: `openspec/changes/fix-live-skill-operations-shell/specs/sdk-system-facade/spec.md`

- [ ] Add OpenSpec proposal/design/tasks and strict-validate the change.

### Task 2: Add CLI Live Web API Adapter

**Files:**
- Modify: `macaca/crates/shells/macaca-cli/Cargo.toml`
- Modify: `macaca/crates/shells/macaca-cli/src/main.rs`
- Modify: `macaca/crates/shells/macaca-cli/src/command_handlers.rs`
- Modify: `macaca/crates/shells/macaca-cli/src/skill_operations.rs`

- [ ] Add shared CLI runtime target arguments: `--app-id` and `--api-base`.
- [ ] Implement app-scoped Web API request helpers with sanitized logs and bounded JSON printing.
- [ ] Keep the existing unavailable SDK client path for commands without `--app-id`.
- [ ] Add focused unit tests for URL/payload construction and shell-boundary imports.

### Task 3: Preserve Frontend Mutation Trace

**Files:**
- Modify: `frontend/components/skills/SkillOperationsPanel.tsx`

- [ ] Split snapshot trace from last command trace.
- [ ] Keep RUN/APPLY/ROLLBACK mutation trace visible after refresh.

### Task 4: Verify

**Commands:**
- `openspec validate fix-live-skill-operations-shell --strict`
- `cargo test -p macaca-cli skill_operations`
- `cargo check -p macaca-cli`
- `cargo check -p macaca-web`
- `npm run lint` in `frontend/`
- Live backend/frontend e2e: Web API mutation, CLI app-scoped read/run, frontend RUN network trace.
- `gitnexus detect_changes()` before any commit.
