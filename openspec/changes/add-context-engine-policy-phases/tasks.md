# Tasks

## 1. 审计与影响分析

- [x] 1.1 审计 `macaca-context` 当前 `ContextEngine`、`PromptComposer`、`ContextReport`、`ContextBudget` 的已实现边界。
- [x] 1.2 审计所有可能进入上下文的大 source：tool result、trace event、file read、command output、search result、skill index、memory recall、workspace file。
- [x] 1.3 审计 session/event/report 持久化路径，确认 compaction lineage 可挂载的位置。
- [x] 1.4 对每个计划编辑的 Rust symbol 先运行 GitNexus upstream impact analysis，并记录风险等级、直接调用者、受影响流程。
- [x] 1.5 确认本变更不与 `add-pluggable-context-engine-foundation` 的 legacy 行为兼容要求冲突。

## 2. Source rendering 和 pruning contract

- [x] 2.1 在 `macaca-context` 中定义 `ContextRenderable`、`ContextSnippet`、source reference、render decision 等最小值对象。
- [x] 2.2 定义 `PruningPolicy` 和 `BudgetPolicy`，默认实现必须只依据 source kind、大小、预算和配置。
- [x] 2.3 为 tool result、trace event、file read、command output、search result 提供默认 renderer。
- [x] 2.4 确保 renderer 输出 summary、bounded excerpt、source/artifact reference 和 token estimate。
- [x] 2.5 单元测试覆盖大输出裁剪、JSON/tool call 参数保持合法、默认策略不检查 app/workflow/agent 名称。

## 3. Non-destructive pruning 接入

- [x] 3.1 在 context facade/engine assembly 中接入 source renderer chain。
- [x] 3.2 保证 canonical transcript、event store、artifact store 中原文不被 pruning 改写。
- [x] 3.3 将 included/summarized/dropped/pruned token 决策写入 `ContextReport`。
- [x] 3.4 在 Web/trace context report 中展示 pruning source breakdown 和原文引用。
- [x] 3.5 增加集成测试验证大 stdout/file read 不再完整进入模型上下文，但原文仍可读取。

## 4. Compaction 和 session lineage

- [x] 4.1 定义 `CompactionPolicy`、compaction trigger、manual compact input 和 focused topic。
- [x] 4.2 定义 fixed compaction summary envelope，包含 reference-only、untrusted、active task、open questions、IDs/paths。
- [x] 4.3 实现 `before_compaction` 和 `after_compaction` lifecycle hooks，供 memory/source provider 做 bounded flush。
- [x] 4.4 在 session/event 存储中表示 successor transcript segment 或 child session lineage。
- [x] 4.5 更新 logical session 查询，使默认展示 lineage tip，debug/审计可展开 root-to-tip。
- [x] 4.6 增加测试验证压缩不删除原始历史、resume 落到最新 successor、摘要不会被当成新用户指令。

## 5. Memory/wiki source provider

- [x] 5.1 定义 memory source provider contract，并保持 `macaca-memory` 只是 source provider，不成为 context engine。
- [x] 5.2 将 durable memory recall 与 wiki/digest source 分离建模。
- [x] 5.3 recall 结果必须带 provenance、confidence、privacy tier 和 source id。
- [x] 5.4 recall 注入必须进入 dynamic/untrusted/request-only section，不写回 canonical transcript。
- [x] 5.5 增加测试验证 memory 不默认全量进入 prompt，prompt stable hash 不受 recall 变化影响。

## 6. Opt-in preflight recall

- [x] 6.1 定义 `ContextPreflightRecall` pipeline step 和启用配置。
- [x] 6.2 限制 preflight recall 只能使用 read-only recall/search/get 工具 allowlist。
- [x] 6.3 增加 timeout、max chars/tokens、failure fallback 和 warning report。
- [x] 6.4 确保 preflight recall 默认关闭，只能通过 config/manifest/profile opt-in。
- [x] 6.5 增加测试验证 recall 超时不会阻塞主模型调用，失败时降级为空 recall。

## 7. 用户自定义与外部 adapter 路径

- [x] 7.1 发布 `ContextEngine`、source provider、policy 的 conformance test suite。
- [x] 7.2 支持本地 in-process custom context engine/provider 注册和配置选择。
- [x] 7.3 设计 `ExternalContextAdapter` 安全边界，但暂不冻结远程 wire protocol。
- [x] 7.4 为后续 process/RPC/WASM adapter 定义 safety policy：schema validation、budget validation、timeout、payload limit、circuit breaker、fallback。
- [x] 7.5 增加测试验证自定义 engine/provider 可替换默认实现且上层不依赖具体类型。

## 8. 验证与归档准备

- [x] 8.1 运行 `openspec validate add-context-engine-policy-phases --strict`。
- [x] 8.2 分阶段运行 `cargo check` 和相关 crate 测试。
- [x] 8.3 对 context report API/UI 做回归验证，确认不泄漏完整 prompt/tool output/memory。
- [ ] 8.4 运行 `gitnexus_detect_changes()`，确认影响范围符合预期。
- [ ] 8.5 所有任务完成后更新 checklist，并确保代码和规范对齐后再归档。
