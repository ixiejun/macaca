# WASM 阶段 6：Lifecycle / State / Checkpoint 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to execute this plan task-by-task. 本阶段补齐长期运行 WASM application 所需生命周期和状态能力。

## Goal

为 WASM application/ability 建立完整生命周期：validate、compile、instantiate、init、start、handle event、render、pause、resume、drain、shutdown、checkpoint、restore、upgrade、rollback。生命周期必须可 trace、可审计、可 fail-closed，并支持 7x24 长期运行。

## Scope

本阶段覆盖：

- lifecycle state machine。
- transition command/result。
- checkpoint memento。
- restore/upgrade/rollback contract。
- drain/shutdown semantics。
- lifecycle audit event。

本阶段不覆盖：

- 完整 persistent storage backend。
- 跨机器 live migration。
- out-of-process provider 的真实进程迁移。

## Required Governance Inputs

- `2026-05-12-wasm-default-in-process-runtime-provider-plan.md`
- `2026-05-12-wasm-host-import-service-portal-plan.md`
- `macaca/docs/agent-os-microkernel-boundaries.md`
- `macaca/docs/design_patterns.md`

## Architecture Decision

采用 State + Command + Memento + Observer + Specification：

- State：所有 lifecycle transition 集中在状态机中表达。
- Command：每个 transition 都是 typed command。
- Memento：checkpoint、restore point、upgrade report、rollback report 都是 sanitized snapshot。
- Observer：transition requested/completed/failed/drained 记录 trace/log。
- Specification：非法 transition、missing trace、policy denied、resource exhausted 都 fail-closed。

## Proposed OpenSpec Change

建议 change id：

- `add-wasm-lifecycle-state-checkpoint`

建议 specs：

- `wasm-application-lifecycle`
- `wasm-checkpoint-restore`
- `wasm-upgrade-rollback`
- `wasm-lifecycle-audit`

提案必须声明：

- lifecycle belongs to Application Runtime execution plane; Kernel 只提供 session/task/resource guard。
- checkpoint 不得包含 raw guest memory dump，除非未来有加密、bounded、policy-approved artifact path。
- upgrade/rollback 必须使用 artifact id/hash/ABI compatibility，不使用 app name 特判。

## Implementation Slices

### Slice 6.1：Impact 与生命周期审计

- [ ] 审计 Application lifecycle service、WASM provider session、ApplicationHostCommand。
- [ ] 对 lifecycle symbols 运行 GitNexus impact。
- [ ] 分类现有 unavailable host lifecycle 行为，确保兼容保留。

### Slice 6.2：OpenSpec lifecycle

- [ ] 定义 lifecycle states 和 allowed transitions。
- [ ] 定义 checkpoint/restore memento contract。
- [ ] 定义 upgrade/rollback decision rules。
- [ ] 定义 transition audit event。

### Slice 6.3：State machine

- [ ] 实现 `WasmLifecycleState` 和 `WasmLifecycleTransition`。
- [ ] 实现 transition validator。
- [ ] 实现 fail-closed reason code。
- [ ] 新增 Rust 代码写详细英文注释。

### Slice 6.4：Runtime lifecycle integration

- [ ] default provider 支持 init/start/event/render/shutdown。
- [ ] pause/resume/drain 没有真实 engine support 时返回 structured unsupported，而不是 silent success。
- [ ] 所有 transition 记录 sanitized logs。

### Slice 6.5：Checkpoint/restore/upgrade

- [ ] 定义 checkpoint metadata，不记录 raw guest memory。
- [ ] 定义 restore request 和 compatibility check。
- [ ] 定义 upgrade/rollback report。
- [ ] 支持 unavailable-safe fallback。

### Slice 6.6：测试

- [ ] valid/invalid transition 测试。
- [ ] checkpoint sanitized 测试。
- [ ] restore ABI mismatch 测试。
- [ ] shutdown/drain audit 测试。

## Validation

- `cargo test -p macaca-app wasm_lifecycle`
- `cargo test -p macaca-runtime-host wasm_lifecycle`
- `cargo test -p macaca-integration-tests application_platform_contracts`
- `openspec validate add-wasm-lifecycle-state-checkpoint --strict`
- `npx gitnexus detect-changes -r agent`

## Risks

- 风险：状态机过度复杂。缓解：先实现稳定核心 transition，unsupported 状态显式返回。
- 风险：checkpoint 泄露敏感数据。缓解：只记录 metadata/memento，不记录 raw memory。
- 风险：upgrade 破坏 ABI。缓解：强制 ABI compatibility spec。
