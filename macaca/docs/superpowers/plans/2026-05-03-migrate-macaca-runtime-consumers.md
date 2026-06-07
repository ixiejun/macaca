# Migrate macaca-runtime Consumers Brainstorm and Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:brainstorm before consumer migration and superpowers:write-plan before implementation. This document is a planning artifact only; do not implement consumer changes from this step without a follow-up OpenSpec proposal.

**Goal:** 迁移上层代码到本次基于设计模式重构后的 `macaca-runtime` template / observer / command primitives，确保上层不再调用 deprecated runtime execution API，并为后续减少 `macaca-web` 对 runtime pause/resume 兼容类型的耦合做好计划。

**Architecture:** 采用 additive-first 消费方迁移。`macaca-runtime` 已新增 `AgenticLoop::execute`、`AgenticLoop::execute_with_events`、`PausableAgenticLoop::execute_with_pause`，并将旧 `run`、`run_with_events`、`run_with_pause` 标记 deprecated 但保留。消费者迁移优先消除 deprecated 执行入口调用；`ResumeReason` 仍作为兼容消息类型保留，后续通过 web/framework 本地 adapter 隔离。

**Tech Stack:** Rust, Tokio, `macaca-runtime`, `macaca-web`, `macaca-integration-tests`, OpenSpec, GitNexus, cargo test/check.

---

## 1. Current Code Facts

已阅读：

- `AGENTS.md`
- `openspec/AGENTS.md`
- `macaca/docs/design-pattern-refactor-plans/README.md`
- `macaca/docs/design-pattern-refactor-plans/refactor-order.md`
- `macaca/docs/design-pattern-refactor-plans/macaca-runtime.md`
- `docs/superpowers/plans/2026-05-03-refactor-macaca-runtime.md`
- `openspec/changes/refactor-macaca-runtime-template-primitives/*`
- `macaca/crates/macaca-runtime/src/agentic_loop.rs`
- `macaca/crates/macaca-runtime/src/events.rs`
- `macaca/crates/macaca-runtime/src/execution.rs`
- `macaca/crates/macaca-runtime/src/template.rs`
- `macaca/crates/macaca-runtime/src/lib.rs`
- `macaca/crates/macaca-integration-tests/src/pipeline_dry_run.rs`
- `macaca/crates/macaca-web/src/hook_consumer.rs`
- `macaca/crates/macaca-web/src/state.rs`
- `macaca/crates/macaca-web/src/chat_orchestrator.rs`
- `macaca/crates/macaca-web/src/framework_runner.rs`
- `macaca/crates/macaca-web/src/loop_manager.rs`

当前 runtime 重构事实：

- `AgenticLoop::execute` 是 `run` 的非 deprecated replacement。
- `AgenticLoop::execute_with_events` 是 `run_with_events` 的非 deprecated replacement。
- `PausableAgenticLoop::execute_with_pause` 是 `run_with_pause` 的非 deprecated replacement。
- `run`、`run_with_events`、`run_with_pause` 已标记 deprecated 并保留为 wrapper，便于后续 grep 查找。
- `events.rs` 提供 runtime event sink wrapper。
- `execution.rs` 提供 tool command execution boundary。
- `template.rs` 提供 runtime iteration outcome。
- 本轮没有迁移 `ResumeReason`，它仍位于 `macaca_runtime::agentic_loop`。

直接 Cargo 消费方：

- `macaca-integration-tests`
- `macaca-web`

当前消费扫描结果：

- `macaca-integration-tests/src/pipeline_dry_run.rs` 已改用 `AgenticLoop::execute_with_events`。
- `pipeline_dry_run.rs` 的注释仍写 `AgenticLoop::run_with_events`，属于文档迁移遗漏。
- `macaca-web` 没有调用 deprecated runtime execution API。
- `macaca-web` 多处依赖 `macaca_runtime::agentic_loop::ResumeReason`：
  - `hook_consumer.rs`
  - `state.rs`
  - `chat_orchestrator.rs`
  - `framework_runner.rs`
  - `loop_manager.rs`
- `ResumeReason` 当前用于 web 内部 active session resume channel、hook completion notification、goal completion notification、PauseOnGoalMiddleware。
- `framework_runner.rs` 注释说明它已经替代 ad-hoc `AgenticLoop` execution；因此 web 侧当前依赖 runtime crate 的主要原因是 resume message type，而不是 execution loop。

GitNexus observations:

- `gitnexus query` 针对 runtime consumers 返回空结果，并提示 FTS index read-only 写入失败；已改用源码扫描确认消费点。
- 进入实现阶段前仍应对将要修改的具体 symbol 运行 `npx gitnexus impact ...`。

## 2. Superpowers Brainstorm

### Option A: 只清理 deprecated execution 消费点和文档

做法：

- 确认 `pipeline_dry_run.rs` 已使用 `execute_with_events`。
- 修正注释和测试命名中仍提到 `run_with_events` 的地方。
- 添加或保留 grep 验证，确保上层没有 `.run_with_events(` 或 `.run_with_pause(`。
- 不碰 `ResumeReason`。

Benefits:

- 风险最低。
- 与当前 runtime refactor 第一切片完全对齐。
- 不引入跨 crate 类型迁移。

Risks:

- `macaca-web` 仍直接依赖 `macaca_runtime::agentic_loop::ResumeReason`，runtime crate 仍被 web 用作 resume DTO 来源。
- 没有进一步降低 web/runtime 耦合。

Conclusion:

- 适合快速完成“deprecated execution API 禁止调用”的收口，但不足以作为完整的 consumer migration 计划。

### Option B: 在 macaca-web 增加本地 RuntimeResumeSignal adapter

做法：

- 在 `macaca-web` 中新增通用、应用无关的 resume signal type，例如 `CoordinatorResumeSignal` 或 `RuntimeResumeSignal`。
- `ActiveSession.resume_tx`、`PauseOnGoalMiddleware`、hook consumer、goal completion path 改用 web 本地 signal type。
- 在边界处提供转换函数：
  - `impl From<RuntimeResumeSignal> for macaca_runtime::agentic_loop::ResumeReason`
  - 或仅保留 private adapter，若不再调用 `PausableAgenticLoop`，则完全不需要转换。
- 保留 `macaca-runtime::ResumeReason` 不删除，便于未来真正 pausable runtime consumer 使用。

Benefits:

- 将 web 的 coordinator/goal resume 语义从 `agentic_loop` 模块路径中隔离。
- 与当前 framework runner 已替代 `PausableAgenticLoop` 的事实一致。
- 为未来迁移 `ResumeReason` 到更合适的 runtime state module 或删除 web 对 runtime 依赖创造条件。

Risks:

- 触碰 `macaca-web` active session、hook consumer、goal completion、framework middleware，变更面比 Option A 大。
- 需要保证 goal completion / delegate completion 的 message injection 文本不变。
- 如果命名不当，可能把 web/session 语义反向污染 runtime 或 framework。

Conclusion:

- 推荐作为本轮消费者迁移的主要切片，但必须先 OpenSpec，并保持 adapter 私有和通用。

### Option C: 将 ResumeReason 移到 macaca-runtime 顶层或新 runtime_state module

做法：

- 新增 `runtime_state.rs` 或 `resume.rs`。
- 将 `ResumeReason` 重新导出到 `macaca_runtime::ResumeReason`。
- 将 `macaca_runtime::agentic_loop::ResumeReason` 标记 deprecated。
- 上层改用顶层 re-export。

Benefits:

- 降低对 `agentic_loop` 模块路径的耦合。
- 变更比 web 本地 type 小。
- 保留 runtime 作为 resume contract 所有者。

Risks:

- 仍然让 `macaca-web` 依赖 `macaca-runtime` 作为 resume DTO 来源。
- 与当前 plan 中“本轮不移动或重命名 ResumeReason”有冲突；需要新的 OpenSpec 明确 supersede。
- 如果后续 framework runner 完全不需要 runtime pause/resume，这一步可能是中间债务。

Conclusion:

- 可作为低风险过渡方案，但不如 Option B 能真正隔离 web/runtime 耦合。

### Option D: 让 macaca-web 重新消费 PausableAgenticLoop::execute_with_pause

做法：

- 把 framework runner 的 pause/resume path 迁回 `PausableAgenticLoop::execute_with_pause`。
- 通过新 non-deprecated runtime API 执行 pause/resume。

Benefits:

- 表面上最大化使用新 runtime primitive。

Risks:

- 与 `framework_runner.rs` 当前设计相反，它明确替代 ad-hoc `AgenticLoop` execution。
- 会回退 framework primitive migration。
- 可能重新引入 loop-level pause 语义，而当前 web 采用 tool middleware-level pause。
- 高风险且没有必要。

Conclusion:

- 不推荐。

## 3. Recommendation

采用 Option B，并包含 Option A 的轻量清理。

推荐迁移边界：

- `macaca-integration-tests`：保持 `execute_with_events`，修正文档注释，作为 runtime template consumer 验收。
- `macaca-web`：新增 web-local resume signal adapter，逐步把 ActiveSession / hook consumer / framework runner middleware / goal completion 从 `macaca_runtime::agentic_loop::ResumeReason` 迁走。
- `macaca-runtime`：不删除 `ResumeReason`，不移动 public type；保留 deprecated execution wrappers。

不做：

- 不重新引入 `PausableAgenticLoop` 到 web execution path。
- 不迁移 `AgenticLoop` 构造方式，`AgenticLoop::new(RuntimeConfig)` 仍是有效 facade。
- 不新增业务/应用/workflow 特定 resume 类型。
- 不改变 goal/delegate completion 文本语义。

## 4. Proposed OpenSpec Change

Change id:

```text
migrate-runtime-consumers-to-template-primitives
```

Affected specs:

- `macaca-runtime-consumers`

Proposal should state:

- Consumers must use `execute*` runtime template entrypoints instead of deprecated `run*` entrypoints.
- `macaca-web` should not depend on deprecated execution APIs.
- `macaca-web` should isolate its framework/goal resume messages behind a local generic adapter instead of importing `agentic_loop::ResumeReason` throughout.
- Deprecated runtime compatibility APIs remain callable inside `macaca-runtime`.

## 5. Implementation Plan

## Task 1: OpenSpec Proposal and Contract

**Files:**

- Create: `openspec/changes/migrate-runtime-consumers-to-template-primitives/proposal.md`
- Create: `openspec/changes/migrate-runtime-consumers-to-template-primitives/design.md`
- Create: `openspec/changes/migrate-runtime-consumers-to-template-primitives/tasks.md`
- Create: `openspec/changes/migrate-runtime-consumers-to-template-primitives/specs/macaca-runtime-consumers/spec.md`

- [ ] **Step 1: Review OpenSpec context**

Run:

```bash
openspec list
openspec list --specs
```

Expected:

- `refactor-macaca-runtime-template-primitives` exists and is complete.
- `migrate-runtime-consumers-to-template-primitives` does not already exist.

- [ ] **Step 2: Create proposal**

Proposal must include:

- Why upper consumers need to stop calling deprecated `run*` APIs.
- Why web resume messages should be isolated from `agentic_loop::ResumeReason`.
- Compatibility: deprecated runtime APIs and `ResumeReason` remain in runtime.

- [ ] **Step 3: Create design**

Design must include:

- Pattern mapping: Adapter for web-local resume signal, Facade for runtime `execute*`, Observer for evented integration tests.
- Direct consumers: `macaca-web`, `macaca-integration-tests`.
- Non-goals: no behavior change, no reintroducing `PausableAgenticLoop` into web, no app/workflow-specific logic.

- [ ] **Step 4: Create tasks and delta spec**

Spec requirements:

- Upper consumers use non-deprecated runtime template entrypoints.
- Web resume signaling is isolated behind a local generic adapter.
- Deprecated runtime execution APIs remain searchable and callable only for compatibility.

- [ ] **Step 5: Validate OpenSpec**

Run:

```bash
openspec validate migrate-runtime-consumers-to-template-primitives --strict
```

## Task 2: Baseline and Impact Analysis

**Files:**

- Read-only: runtime, web, integration tests.

- [ ] **Step 1: Confirm direct consumers**

Run:

```bash
cargo metadata --no-deps --format-version 1 \
  | jq -r '.packages[] | select(.dependencies[]?.name=="macaca-runtime") | .name'
```

Expected:

- `macaca-web`
- `macaca-integration-tests`

- [ ] **Step 2: Scan deprecated runtime execution usage**

Run:

```bash
rg -n "AgenticLoop::run|AgenticLoop::run_with_events|PausableAgenticLoop::run_with_pause|\\.run_with_events\\(|\\.run_with_pause\\(" \
  macaca/crates/macaca-web macaca/crates/macaca-integration-tests
```

Expected:

- No executable upper calls.
- Documentation-only references may remain and should be updated.

- [ ] **Step 3: Run GitNexus impact**

Before editing, run impact for symbols that may change:

```text
run_agentic_traced
ActiveSession
start_hook_event_consumer
PauseOnGoalMiddleware
build_framework_runner
```

If GitNexus cannot resolve a symbol or returns HIGH/CRITICAL, report it before proceeding.

## Task 3: Integration Test Consumer Cleanup

**Files:**

- Modify: `macaca/crates/macaca-integration-tests/src/pipeline_dry_run.rs`

- [ ] **Step 1: Update stale doc comment**

Replace `AgenticLoop::run_with_events` wording with `AgenticLoop::execute_with_events`.

- [ ] **Step 2: Confirm execution call**

Ensure `run_agentic_traced` calls `loop_.execute_with_events(...)`.

- [ ] **Step 3: Keep dry-run behavior unchanged**

Do not change scripted LLM turns, event formatting, or tool flow.

## Task 4: Web Resume Signal Adapter

**Files:**

- Create: `macaca/crates/macaca-web/src/runtime_resume.rs`
- Modify: `macaca/crates/macaca-web/src/lib.rs` or module declarations.
- Modify: `macaca/crates/macaca-web/src/state.rs`
- Modify: `macaca/crates/macaca-web/src/hook_consumer.rs`
- Modify: `macaca/crates/macaca-web/src/chat_orchestrator.rs`
- Modify: `macaca/crates/macaca-web/src/framework_runner.rs`
- Modify: `macaca/crates/macaca-web/src/loop_manager.rs`

- [ ] **Step 1: Add local signal type**

Create a generic web-local type, for example:

```rust
pub enum RuntimeResumeSignal {
    Manual,
    DelegateCompleted { task_id: String, success: bool, output: String },
    DelegateFailed { task_id: String, error: String },
    Timeout,
}
```

Naming must stay generic; no hardcoded workflow/app names.

- [ ] **Step 2: Add adapter only if needed**

If any remaining code must call `PausableAgenticLoop`, add a private conversion to `ResumeReason`. Otherwise do not convert.

- [ ] **Step 3: Migrate web imports and channel types**

Replace web imports of `macaca_runtime::agentic_loop::ResumeReason` with `crate::runtime_resume::RuntimeResumeSignal`.

- [ ] **Step 4: Preserve matching semantics**

`PauseOnGoalMiddleware` should still extract output from completed delegate/goal signals and append the same completion text.

- [ ] **Step 5: Avoid changing active session behavior**

Do not change channel size, pause signal behavior, forwarder stop behavior, or SSE reconnection logic.

## Task 5: Deprecated Usage Verification

- [ ] Run grep to confirm no upper executable calls to deprecated runtime execution APIs.
- [ ] Run grep to confirm `macaca-web` no longer imports `macaca_runtime::agentic_loop::ResumeReason`.
- [ ] Allow deprecated wrappers only inside `macaca-runtime`.

## Task 6: Verification

- [ ] Run `cargo fmt`.
- [ ] Run `cargo test -p macaca-runtime -- --nocapture`.
- [ ] Run `cargo test -p macaca-integration-tests pipeline_dry_run -- --nocapture`.
- [ ] Run `cargo check -p macaca-runtime -p macaca-web -p macaca-integration-tests`.
- [ ] Run `openspec validate migrate-runtime-consumers-to-template-primitives --strict`.
- [ ] Run `npx gitnexus detect-changes --repo agent --scope all`.

## 6. Risk Controls

- Keep `macaca-runtime::agentic_loop::ResumeReason` public and callable.
- Do not delete deprecated runtime execution wrappers.
- Do not modify framework runner execution semantics.
- Do not change goal/delegate resume payload text beyond type names.
- Keep new web adapter file under 500 lines.
- Treat `macaca-web` as the only production upper consumer in this migration.

## 7. Completion Criteria

- No upper crate calls `AgenticLoop::run`, `AgenticLoop::run_with_events`, or `PausableAgenticLoop::run_with_pause`.
- `macaca-integration-tests` documents and calls `execute_with_events`.
- `macaca-web` no longer imports `macaca_runtime::agentic_loop::ResumeReason` directly.
- Runtime deprecated wrappers remain available for external migration searches.
- OpenSpec, tests, check, and GitNexus validation pass.
