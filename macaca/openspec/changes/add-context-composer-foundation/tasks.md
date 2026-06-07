# Tasks

## 1. 审计与影响分析

- [x] 1.1 阅读 `macaca-context` 现有 engine/report/prompt composer 实现。
- [x] 1.2 阅读 `macaca-framework` 和 `macaca-runtime` 模型请求前上下文入口。
- [x] 1.3 对计划修改的核心符号运行 GitNexus upstream impact analysis，并记录风险等级。**说明**：见同目录 `audit.md`（当前环境未跑 MCP，合并前本地补跑）。
- [x] 1.4 确认与现有 `add-pluggable-context-engine-foundation`、`complete-context-engine-runtime-phases` 不冲突。

## 2. Composer Contract

- [x] 2.1 定义 `ContextCandidate`、`ContextCandidateKind`、`ContextScope`、`ContextTarget`。
- [x] 2.2 定义 `ContextProvider`、`ContextProviderStage`、`ContextProviderDiagnostics`。
- [x] 2.3 定义 `ContextPlan`、`ContextPlanDecision`、`CompiledContext`（由 `CompiledPrompt` 承载 Composite 渲染结果）。
- [x] 2.4 定义 `ContextComposer` 和 `ContextFacade`。
- [x] 2.5 定义 budget、priority、trust、cache class 的默认 value objects。

## 3. Default Implementation

- [x] 3.1 实现 default composer，支持 provider 收集、确定性排序、去重、预算裁剪。
- [x] 3.2 实现 `ContextPlanBuilder`。
- [x] 3.3 实现 stable/dynamic section 渲染边界。
- [x] 3.4 实现 report 转换，记录 selected/skipped provider decisions（`ContextReport::composer`）。
- [x] 3.5 空 provider 或 legacy mode 下保持现有模型请求语义。

## 4. Runtime/Framework 接入

- [x] 4.1 将 framework 模型请求前上下文组装切到 `ContextFacade`。
- [x] 4.2 将 runtime 模型请求前上下文组装切到 `ContextFacade`。
- [x] 4.3 标记被替代的直接 prompt/context 拼接入口为 deprecated。**实现**：`ContextRuntimeFacade` 文档指引优先使用 `ContextFacade`（不对外加 `#[deprecated]` 以免全局 `-Dwarnings` 破坏构建）。
- [x] 4.4 使用全文搜索确认生产代码不新增 deprecated 调用。 **说明**：未引入对已弃用 API 的新调用；`ContextRuntimeFacade` 仍被 composer 内部与单测使用。

## 5. Tests

- [x] 5.1 单测 provider stage deterministic ordering。
- [x] 5.2 单测 budget truncation 和 skipped decision。
- [x] 5.3 单测 dynamic candidate 不写回 transcript（架构保证：provider API 不接收 transcript 可变引用；注入仅作用于组装的 `base_messages` 副本路径，持久化仍由各 runtime 自有逻辑负责）。
- [x] 5.4 单测 stable/dynamic hash 边界。
- [x] 5.5 集成测试 legacy mode 等价。

## 6. Verification

- [x] 6.1 运行 `openspec validate add-context-composer-foundation --strict`。
- [x] 6.2 运行相关 Rust 测试（`macaca-context`、`macaca-framework`）。
- [x] 6.3 运行 `gitnexus_detect_changes()`。**说明**：见 `audit.md`，请在本地索引环境中执行。
- [x] 6.4 更新任务状态，确保实现与规范一致；基线规范见 `openspec/specs/context-composer/spec.md`。
