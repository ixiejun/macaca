# Change: 增加可插拔上下文工程基础设施

## Why

Macaca 作为 7x24 小时运行的 Agent OS，需要长期承载多 application、多 agent、多 session、多 runtime 的模型调用。当前上下文组装分散在 Web agent 构造、framework ReAct loop、runtime loop、skill 注入、memory/working memory 和 tool schema 注入等路径中，缺少统一的可观测边界和可替换策略。

本变更建立最小的上下文工程基础设施：先保留现有行为，通过可插拔 `ContextEngine` 抽象、`ContextReport` 诊断和 `PromptComposer` stable/dynamic 分层，为后续 pruning、compaction、memory recall、外部上下文系统接入打基础。核心目标是避免把 Macaca Agent OS 与任何具体上下文管理系统强耦合，允许用户未来替换为自己的上下文管理系统。

## What Changes

- 新增 `context-engine` 能力规范，定义 Macaca 上下文工程的基础契约。
- 增加可插拔 `ContextEngine` 抽象，默认实现为 `LegacyContextEngine`，保持现有 LLM 请求语义兼容。
- 增加 `ContextReport`，为每次模型请求记录上下文来源、预算、估算 token、prompt hash、裁剪/降级决策等摘要信息。
- 增加 `PromptComposer` 设计约束，将 prompt 构建拆分为 typed sections，并显式区分 stable 与 dynamic 区域。
- 增加配置/manifest 驱动的上下文引擎选择原则，禁止通过 app name、workflow name、agent name 等硬编码分支选择上下文行为。
- 定义后续扩展边界：source provider、policy、external adapter 均通过接口接入，不能让上层业务代码依赖具体实现。
- 本提案只做基础设施和默认 legacy 包装，不实现 LLM-based compaction、active memory recall sub-agent、外部 RPC/WASM 协议或大规模 UI 改造。

## Impact

- Affected specs: `context-engine`
- Affected code:
  - `macaca/crates/macaca-context`：推荐新增 crate，用于承载上下文工程契约和默认通用组件。
  - `macaca/crates/macaca-framework`：后续接入 `ReActAgent` 请求前上下文组装与报告。
  - `macaca/crates/macaca-runtime`：后续接入直接 agentic loop 的上下文组装与报告。
  - `macaca/crates/macaca-web`：后续通过 facade/API 读取上下文报告，不直接实现上下文策略。
  - `macaca/crates/macaca-skill`：后续作为 skill context source，不拥有上下文引擎。
  - `macaca/crates/macaca-memory`：后续作为 memory context source，不拥有上下文引擎。
  - `macaca/crates/macaca-llm`：保持 provider-facing，不引入 Macaca-specific 上下文策略。
- Compatibility:
  - 默认 `legacy` 引擎必须保持现有 LLM messages/options 的行为兼容。
  - 旧 prompt 构造入口在迁移完成前保留；迁移后标记 deprecated，但不得删除，便于后续查找。
- Non-impact:
  - 不修改现有业务 application 语义。
  - 不删除现有 memory/session/event store。
  - 不引入 application-specific 或 workflow-specific 的上下文逻辑。
