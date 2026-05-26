# Long-Term Memory Milvus Persistence Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make local Macaca web runs use the configured long-term memory backend and persist successful chat completion evidence into scoped Memory Service memory.

**Architecture:** `macaca-memory` owns backend construction through an Abstract Factory. `macaca-web` observes session completion and writes a bounded memory item through `SystemMemoryClient`, preserving the service boundary.

**Tech Stack:** Rust, Tokio, async-trait, Macaca Memory Service DTOs, OpenSpec.

---

### Task 1: Configured Memory Runtime

**Files:**
- Modify: `macaca/crates/services/macaca-memory/src/store.rs`
- Modify: `macaca/crates/services/macaca-memory/src/backend.rs`
- Modify: `macaca/crates/shells/macaca-web/src/lib.rs`

- [ ] Write failing tests for configured Milvus/DashScope profile selection.
- [ ] Add trait-object dispatch for `VectorStore` and `EmbeddingProvider`.
- [ ] Add configuration-driven memory manager construction.
- [ ] Wire web startup to the configured factory and sanitized logs.

### Task 2: Session Completion Capture

**Files:**
- Create: `macaca/crates/shells/macaca-web/src/session_memory_capture.rs`
- Modify: `macaca/crates/shells/macaca-web/src/chat_orchestrator.rs`
- Modify: `macaca/crates/shells/macaca-web/src/lib.rs`

- [ ] Write failing tests proving a successful chat creates one scoped `MemoryRememberCommand`.
- [ ] Implement bounded memory content and metadata generation.
- [ ] Call capture after final session persistence for successful service-agent chat.
- [ ] Log success/failure without raw prompt or model output.

### Task 3: Verification And Commit

**Files:**
- Modify: `openspec/changes/fix-long-term-memory-milvus-persistence/tasks.md`

- [ ] Run focused cargo tests.
- [ ] Run OpenSpec validation.
- [ ] Run `git diff --check`.
- [ ] Run GitNexus detect changes and record HIGH/CRITICAL as non-blocking notes.
- [ ] Commit the complete change.
