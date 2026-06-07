# Change: 完成 Context Engineering 未闭环 Phases

## Why

`2026-05-05-macaca-context-engineering-brainstorm-plan.md` 定义了 `Phase 0-10` 的完整路线，但当前 OpenSpec 与代码状态仍分散在多个 changes 中：

- `add-pluggable-context-engine-foundation` 主要完成 foundation / contract
- `complete-context-engine-runtime-phases` 主要推进 `Phase 0-5`
- `add-context-composer-foundation`、`add-memory-active-recall-integration`、`add-knowledge-digest-context-provider` 等 changes 又分别补了局部能力

审计表明，研究报告原始 `Phase 0-5` 中 `Phase 0`、`Phase 1`、`Phase 5` 已基本闭环，`Phase 2`、`Phase 3`、`Phase 4` 已有基础能力但仍缺产品级闭环。后续 OpenSpec 已把这些缺口扩展为 `Phase 6-10` 收尾项。缺少一个“以所有未闭环 Phases 最终完成为目标”的总收口 OpenSpec。

本 change 根据 `docs/superpowers/plans/2026-05-06-complete-context-engine-unfinished-phases-plan.md` 更新，重点不是重写已有 context engine，而是补齐以下闭环：

- non-destructive pruning 的 canonical 原文可追溯。
- compaction/session lineage 的 logical-session UX。
- memory/wiki/digest recall 的 bounded、dynamic、untrusted、request-only runtime path。
- custom engine/provider 与 external adapter 的可插拔边界。
- legacy prompt/context API 的迁移、deprecated 保留与归档门禁。

## What Changes

- 使用 `complete-context-engine-all-phases` 作为总收口 change，明确 `Phase 0-10` 的最终完成标准与剩余实现切片。
- 新增 `phase-status.md` 作为总收口状态矩阵，要求每个 Phase 同时满足 contract、runtime、diagnostics、verification 才能标记 complete。
- 对 `Phase 0-5`：
  - 校正 Phase 状态矩阵，使之反映当前真实实现。
  - 明确 `Phase 2/3/4` 的剩余闭环分别由 `Phase 6/7/8` 承接。
- 对 `Phase 6`：
  - 要求所有 pruning source kind 的原始 payload 在 canonical storage 中保持可追溯。
  - 引入 repository/adapter 风格的 source artifact retrieval 边界。
  - 要求 API/UI 能通过 source ref 访问原始数据、bounded preview 或明确不可取回原因。
- 对 `Phase 7`：
  - 补齐 lineage tip/root-to-tip 的前端展示与交互，不只停留在 API 和事件文案。
  - 明确 compaction 的自动/手动流、resume 语义、summary fence 与可视化要求。
- 对 `Phase 8`：
  - 完成 wiki/digest runtime recall 入口，优先通过 context source provider path 接入。
  - 明确 recall 注入必须是 dynamic/untrusted/request-only，并带 provenance、confidence、privacy tier。
  - 对齐 active recall、preflight recall、explicit memory tools、wiki/digest provider 的 diagnostics。
- 对 `Phase 9`：
  - 完成 config-driven custom engine/provider registration 的系统级接入路径。
  - 定义 local in-process custom engine 的 conformance、注册、选择与 fallback。
  - 定义 process/RPC/WASM external adapter 的安全边界、预算、超时、schema validation、circuit breaker。
- 对 `Phase 10`：
  - 完成所有 legacy prompt/context 入口的迁移纪律与 searchable deprecated 兼容层。
  - 在归档前强制 `rg`、OpenSpec、GitNexus、测试矩阵全部通过。

## Impact

- Affected specs:
  - `context-engine-runtime`
- Status artifact:
  - `openspec/changes/complete-context-engine-all-phases/phase-status.md`
- Related changes:
  - `add-pluggable-context-engine-foundation`
  - `complete-context-engine-runtime-phases`
  - `add-context-composer-foundation`
  - `add-memory-active-recall-integration`
  - `add-knowledge-digest-context-provider`
  - `add-context-governance-provider-runtime`
- Affected code:
  - `macaca/crates/macaca-context`
  - `macaca/crates/macaca-web`
  - `macaca/crates/macaca-runtime`
  - `macaca/crates/macaca-framework`
  - `macaca/crates/macaca-persist`
  - `frontend/`
- Compatibility:
  - 默认行为仍必须保持 `legacy` 可回退。
  - 所有新能力必须由 config / manifest / profile 驱动，不得按 app/workflow/agent 名硬编码分支。
  - pruning、recall、compaction、external adapter 都不得破坏 canonical transcript / event / artifact store。
  - legacy API 不删除，只标记 deprecated 或保留 rustdoc replacement 指引，便于后续迁移查找。

## Design Pattern Commitments

- Facade: production callers use context facade/runtime facade rather than prompt string builders.
- Adapter: source artifact retrieval, lineage persistence, memory/wiki recall, and external context providers are bridged through adapters.
- Strategy: pruning, recall, lineage display, provider selection, fallback, and external safety policies remain replaceable.
- Decorator: redaction, tombstone, timeout, trust fencing, circuit breaker, and schema validation wrap untrusted or external inputs.
- Repository: canonical EventLog/session/artifact payload access is isolated behind retrieval repositories.
- Chain of Responsibility: profile, memory, wiki/digest, skills, MCP, and tool schema context continue to enter through provider stages.
- Memento: compaction summaries and successor lineage nodes remain audit snapshots, not destructive transcript rewrites.
- Ports and Adapters / Bridge: custom engines/providers and external adapters implement Macaca-owned ports without coupling application logic to concrete implementations.
