## Context

`macaca-tools` 位于 `task` 之后、`framework` 之前，是 Agent OS 的执行层 contract。当前它暴露的核心接口是：

- `Tool`
- `ToolSet`
- `TraceEvent`

主要上层消费方是：

- `macaca-framework` 的 `ToolSetBridge` / `SingleToolAdapter`
- `macaca-web` 的 `CompositeToolSet`
- `macaca-driver` 的 `DriverToolSet`
- `macaca-integration-tests` 的 `LocalToolSet`

风险集中在两个点：

1. `Tool` 同时承担 schema、execute、streaming
2. `ToolSet` 只有扁平 `Vec<Box<dyn Tool>>` 视图，组合逻辑散落在上层

## Goals / Non-Goals

### Goals

- 提供 command-style canonical tool execution contract
- 提供独立 schema provider contract
- 提供标准 tool middleware chain 和默认 trace middleware
- 提供 crate-level composite toolset 原语
- 将仓库内已知上层消费方迁移到新入口
- 将旧入口标记为 `deprecated` 但继续兼容

### Non-Goals

- 不引入 `macaca-framework` 反向依赖
- 不实现新的跨进程 trace 协议
- 不改变现有具体 tool 的输入输出 JSON 行为

## Decision

### 1. Additive-first command abstraction

新增以下原语：

- `ToolCommandContext`
- `ToolCommand`
- `ToolSchemaProvider`
- `ToolCommandExecutor`

其中 `ToolCommandExecutor` 对现有 `Tool` 做 blanket adapter，这样：

- 旧实现不必当场全部重写
- 新消费方可以直接改用 canonical contract

### 2. Keep Tool as compatibility shell

`Tool` trait 继续存在，但其旧 consumer-facing 方法将被标记 `deprecated`：

- `parameters_schema()`
- `execute()`
- `execute_streaming()`

新 canonical 使用方式改为：

- `ToolSchemaProvider::tool_schema()`
- `ToolCommandExecutor::execute_command()`

### 3. Standard middleware in macaca-tools, not in business tools

新增：

- `ToolCommandMiddleware`
- `ToolCommandPipeline`
- `TraceToolCommandMiddleware`

它们只依赖 `TraceEvent` 和 `ToolCommandContext`，不依赖 `macaca-framework`。`framework` 和其他上层桥接只负责把事件 channel 放进 context。

### 4. Business tools stay business-only

`orchestration` / `todo` tools 继续只表达业务动作：

- delegate / get_result / report_result
- claim / start / progress / review / create_goal

标准 trace 发射交给 pipeline/middleware 处理，不再要求业务 tool 感知 streaming 细节。

### 5. Composite toolset at the producer crate

新增 `CompositeToolSet` 到 `macaca-tools`，作为标准 group 聚合原语。上层自定义聚合实现迁到它：

- `macaca-web::CompositeToolSet`
- `macaca-driver::DriverToolSet`
- `macaca-integration-tests::LocalToolSet`

旧 `ToolSet::tools()` 和 `to_definitions()` 保留兼容但标记 `deprecated`，新 canonical 入口改为：

- `ToolCatalog::all_tools()`
- `ToolCatalog::definitions()`

## Trade-offs

### Pros

- 不破坏现有 trait object 使用方式
- 给后续 schema cache / permission middleware / MCP tool wrapping 留下稳定挂点
- 将上层重复聚合逻辑收回 `macaca-tools`

### Cons

- 一段时间内会同时存在旧 contract 和新 contract
- 需要在 `framework` / `web` / `driver` 做小规模迁移

## Migration Strategy

1. 先在 `macaca-tools` 增加新原语和 blanket adapter
2. 立即迁移 `framework` adapter 到 command/schema 新入口
3. 再迁移 `web` / `driver` / `integration-tests` 的 toolset 聚合到 `CompositeToolSet`
4. 最后给旧入口打 `deprecated`

### 6. Upper-layer migration boundary

上层 crate 消费 `macaca-tools` 时，必须优先使用 canonical 入口：

- schema: `ToolSchemaProvider::tool_schema()`
- lookup / definitions: `ToolCatalog::find_tool()` / `ToolCatalog::definitions()`
- execution: `ToolCommandExecutor::execute_command()`
- composition: `CompositeToolSet`

本次迁移不要求立刻删除所有公开签名中的 `ToolSet`，但如果某个上层模块只是读取 catalog 或执行 command，它应该迁移到 `ToolCatalog` 或 canonical helper。`ToolSet` 仅作为 producer-side deprecated compatibility shell 保留。

以下同名 API 不属于本次 deprecated `macaca-tools` 调用，允许保留：

- `macaca-driver::Driver::tools()`：driver 自身的工具枚举 API
- `macaca-framework::Toolkit::get_tool()`：framework toolkit 的注册表 API
- `macaca-framework::ToolHandler::execute()`：framework handler contract

剩余直接调用 deprecated `macaca-tools::Tool` / `ToolSet` 方法的路径，必须迁移或明确限制在兼容适配层内部。

## Verification

- `cargo test -p macaca-tools -- --nocapture`
- `cargo test -p macaca-skill -- --nocapture`
- `cargo check -p macaca-tools`
- `cargo check -p macaca-framework -p macaca-web -p macaca-driver -p macaca-skill -p macaca-runtime -p macaca-agent -p macaca-sdk -p macaca-kernel -p macaca-integration-tests`
- deprecated-call containment grep:
  `rg -n "\.parameters_schema\(|\.execute\(|\.execute_streaming\(|\.to_definitions\(|\.get_tool\(|\.tools\(" macaca/crates --glob '*.rs'`
- workspace `cargo check`
- `gitnexus_detect_changes(scope: "all")`
