# Change: 渐进式重构 macaca-app 核心装配与 workflow prompt 抽象

## Why

`macaca-app` 承担 application manifest 解析、runtime 装配、workflow prompt 生成和应用级配置解释，是“应用如何被 Agent OS 运行”的声明式边界。当前 crate 已经具备可运行能力，但从长期演进看，仍存在几类扩展压力：

- `AppRuntime::start_app*` 同时承担 parse / validate / assemble / register 责任，后续继续增加 manifest 解释逻辑时，边界会越来越混杂。
- `WorkflowEngine` 的默认 prompt 仍然包含工具名和执行行为的硬编码文本，例如 `claude_code_execute`，不利于 driver/tool policy/agent capability 的持续解耦。
- application startup 相关职责分散在 loader、runtime、workflow、registry 之间，缺少稳定的工厂式装配边界。
- agent capability 的上游来源未来会同时来自 manifest、skills、drivers、tool policy、framework primitive，仅靠分散字段解释会降低可追踪性。

本 change 依据：

- `macaca/docs/design-pattern-refactor-plans/README.md` 的全局渐进式重构约束
- `macaca/docs/design-pattern-refactor-plans/macaca-app.md` 的 crate 级渐进计划

目标是在行为 1:1 还原前提下，为 `macaca-app` 建立更稳定的 Builder / Template Method / Strategy / Abstract Factory / Composite 结构，支撑后续跨应用统一演进。

## What Changes

- 增加 `AppRuntimeBuilder`，把 manifest 到 runtime 的装配拆成 parse / validate / assemble 三段，但保持外部启动行为不变。
- 抽出 `WorkflowPromptParts`，将当前 workflow prompt 拆成 role、constraints、tools、handoff 等稳定片段。
- 引入 `WorkflowPromptStrategy`，默认实现保持当前 prompt 文本和行为兼容。
- 将 driver/tool 选择规则从字符串硬编码 prompt 迁移到 capability/provider 输入层，不再在默认 prompt 中写死单个 driver 名称。
- 为 application-level capability 增加 composite 表达，先保持 legacy 对外视图兼容。
- 增加 runtime / prompt snapshot 测试，锁定 `FULLSTACK-AUTODEV` 与 `NEWSROOM-AUTOWRITER` 的运行时输出兼容性。

## Non-Goals

- 不改变 application manifest schema。
- 不改变 `entry_agent`、`allowed_tools`、skills、drivers、MCP 的现有配置语义。
- 不在本 change 中迁移 `macaca-web`、`macaca-framework`、`macaca-task` 的 agent 构建逻辑。
- 不在本 change 中改变 coordinator / planner / worker 的运行链路。
- 不在本 change 中修改 task 分解、claim、review、resume 的业务语义。
- 不一次性删除现有 `AppRuntime` / `WorkflowEngine` API；旧入口只允许委托到新抽象。
- 不以单个 application 写特化逻辑，应用差异必须通过 manifest、capability、tool policy 或 prompt strategy 表达。

## Impact

- Affected specs: `macaca-app-core`
- Affected code:
  - `macaca/crates/macaca-app/src/loader.rs`
  - `macaca/crates/macaca-app/src/runtime.rs`
  - `macaca/crates/macaca-app/src/workflow.rs`
  - `macaca/crates/macaca-app/src/model.rs`
  - 相关测试与 fixture
- Expected risk: Medium
- Risk reason:
  - `macaca-app` 是所有 application 启动和 prompt 默认值的基础入口。
  - 但本 change 采用 additive-first 路径，优先加 builder/strategy/factory/composite，再让旧 API 委托，避免一次性破坏调用侧。
- Behavioral compatibility:
  - `AppRuntime::start_app*`、`stop_app`、`remove_app`、`list_apps` 的外部行为必须保持一致。
  - `WorkflowEngine::build_system_prompt` 默认输出在未切换 strategy 时必须保持兼容。
  - 对 application 的 capability / tool / driver 可见性不能降低。
  - 不得削弱 trace、session、resume、task todo、driver、MCP、skill 的可观测性。

## Rollout Strategy

本 change 必须按文档定义的小切片推进：

1. 先补测试，锁定当前 runtime 和 prompt 行为。
2. 再增加 `AppRuntimeBuilder`，旧启动入口先委托。
3. 抽出 `WorkflowPromptParts` 和 `WorkflowPromptStrategy`，默认实现保持输出兼容。
4. 将 driver/tool 规则迁移到 capability/provider 层，避免 prompt 中持续硬编码具体 driver。
5. 最后引入 application capability composite，并用快照测试确认 `FULLSTACK-AUTODEV`、`NEWSROOM-AUTOWRITER` 的 runtime 结果不变。

每个切片都必须可以单独编译、单独回滚。
