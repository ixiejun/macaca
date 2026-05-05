# Change: 完整接入上下文工程运行时阶段

## Why

`add-pluggable-context-engine-foundation` 和 `add-context-engine-policy-phases` 已经定义了 `ContextEngine`、`PromptComposer`、`ContextReport`、pruning、compaction lineage、memory/wiki source、preflight recall 和 external adapter 等基础契约。但审计显示，报告里的 Phase 0-5 只有部分 contract 和单元测试落地，很多验收项还没有接入真实模型调用路径、配置、持久化事件、API/UI 和运行时回退。

本变更把“已定义 contract”和“已接入运行时行为”明确拆开，并以报告 `docs/context-engineering-openclaw-hermes-research.md` 的 Phase 0-5 验收为准，补齐所有运行时闭环。

## What Changes

- 新增 `context-engine-runtime` 能力规范，专门描述运行时落地行为，不再把 contract 定义等同于 Phase 完成。
- Phase 0：统一所有 LLM 调用入口的 `ContextReport` 持久化，不只 framework path 有 EventLog report。
- Phase 1：将真实 system prompt/application/persona/skills/workspace 注入迁移到 `PromptComposer` typed sections，并保持 stable/dynamic hash 可测。
- Phase 2：让 pruning engine 通过 config/manifest/profile 可选并进入 framework/runtime 模型请求路径，确保大 tool result/stdout/file read 不完整进入模型上下文。
- Phase 3：补齐 compaction 运行时：自动触发、manual compact API、successor session/segment、resume 到 tip、compaction event 和 lineage UI。
- Phase 4：补齐 memory recall/wiki：提供 `memory_search`/`memory_get` 类只读 recall 工具、可选 preflight recall、request-only untrusted 注入，并在 `ContextReport` 中可见。
- Phase 5：补齐插件化 context engine：内置 `windowed`、`summary` engine，支持 config/manifest/profile 选择，失败 fallback 并产生可观测事件。
- 修正 OpenSpec 任务语义：`add-context-engine-policy-phases` 中已完成的 contract 项保留为 contract 完成，新 change 的任务才代表 runtime 完成。

## Impact

- Affected specs: `context-engine-runtime`
- Related specs/changes:
  - `add-pluggable-context-engine-foundation`
  - `add-context-engine-policy-phases`
- Affected code:
  - `macaca/crates/macaca-context`：新增/完善 runtime-selectable engines、source providers、policy composition、fallback reporting。
  - `macaca/crates/macaca-framework`：ReAct 模型请求改为通过 selected context engine 组装，而不是只使用 legacy report wrapper。
  - `macaca/crates/macaca-runtime`：direct agentic loop 持久化 context report，并支持 selected engine/pruning/windowed/summary。
  - `macaca/crates/macaca-web`：增加 context engine 配置解析、manual compact API、lineage/context report UI 展示。
  - `macaca/crates/macaca-memory`：提供 memory/wiki source provider 和只读 recall 工具接入点。
  - `macaca/crates/macaca-persist`：使用 lineage store 记录 successor/tip 并支持 UI/API 查询。
- Compatibility:
  - 默认仍为 `legacy`，除非配置显式选择其他 engine。
  - `legacy` 模式必须保持现有 messages/options 行为兼容。
  - Pruning、recall、compaction 都不得改写 canonical transcript/event store。
  - 所有新增行为必须是 application-generic，不能按 app/workflow/agent 名称硬编码。
