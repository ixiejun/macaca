# Tasks

## 1. 审计与影响分析

- [x] 1.1 阅读 `macaca-memory` active recall、facade、scope、governance、vector topology 实现。
- [x] 1.2 阅读 `macaca-context` provider/composer/report contract。
- [x] 1.3 查找所有直接把 memory 内容拼入 prompt 的生产路径。
- [ ] 1.4 对计划修改符号运行 GitNexus upstream impact analysis（建议在合入前本地执行 `gitnexus_impact` / `gitnexus_detect_changes`）。

## 2. Provider Contract

- [x] 2.1 定义 memory recall context provider adapter（`MemoryActiveRecallContextProvider` + `WorkspaceMemoryRecallSource`）。
- [x] 2.2 定义 recall request 到 context request 的字段映射（`MemoryRecallQuery`）。
- [x] 2.3 明确 session id、application id、agent name、scope 的路由语义（query 字段 + `workspace_memory_entry_visible_for_recall`）。
- [x] 2.4 定义 recall candidate 到 `ContextCandidate` 的转换（`memory_active_recall_provider`）。

## 3. Recall Policy Integration

- [x] 3.1 默认召回 `AgentPrivate`（`include_agent_private` 默认 true）。
- [x] 3.2 默认召回 `SessionShared`（`include_session_shared` 默认 true）。
- [ ] 3.3 接入 tombstone/governance/redaction/filter（当前为 workspace 适配器层保守过滤；完整 governance 管线待对接 `GovernedMemoryFacade` 等）。
- [x] 3.4 增加 max hits、score threshold、token/char budget、timeout（`ActiveVectorMemoryContextConfig` / `ActiveRecallPolicy`）。
- [x] 3.5 provider error/timeout fail-open 并记录 diagnostics（composer provider + `active_recall` 单测）。

## 4. Context Integration

- [x] 4.1 将 recall candidates 标记为 dynamic/request-only/fenced。
- [x] 4.2 确保 recall injection 不写回 canonical transcript（dynamic candidate 路径）。
- [x] 4.3 将 recall diagnostics 写入 `ContextReport`（`active_recall` / provider outcome）。
- [x] 4.4 默认不持久化完整 memory content（report 诊断语义与现有 `active_recall` 测试一致）。

## 5. Migration

- [x] 5.1 迁移生产代码中直接 memory prompt injection（composer 路径 + `apply_active_recall` 在 composer 激活时 no-op）。
- [x] 5.2 标记旧 direct memory context helper deprecated（文档说明 legacy `apply_active_recall` 与 composer 迁移关系）。
- [x] 5.3 保留显式 memory search/tool 能力（未移除既有 recall 工具能力）。

## 6. Tests

- [x] 6.1 单测 session/application/agent scope routing（`MemoryRecallQuery` + `workspace_memory_recall_source` 可见性单测）。
- [x] 6.2 单测 AgentPrivate 与 SessionShared 合并（默认 query 标志 + 6.1 单测覆盖分支）。
- [x] 6.3 单测 private memory 不泄漏到其他 agent（`agent_row_requires_matching_route_and_flag` / fail-closed）。
- [x] 6.4 单测 timeout/fail-open（`macaca-context` `active_recall` 测试）。
- [x] 6.5 单测 report 不泄漏完整 memory content（`active_recall_report_omits_full_content`）。

## 7. Verification

- [x] 7.1 运行 `openspec validate add-active-vector-memory-context --strict`。
- [x] 7.2 运行 `cargo test -p macaca-memory -p macaca-context`（或通过 workspace 相关 crate 测试）。
- [ ] 7.3 运行相关 framework/runtime 测试（按需全量 `cargo test` / CI）。
- [ ] 7.4 运行 `gitnexus_detect_changes()`（合入前建议执行）。
