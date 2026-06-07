# Change: Refactor macaca-tools toward command, schema, middleware, and composite primitives

## Why

`macaca-tools` 当前把工具 schema、执行、streaming trace 和 toolset 聚合都压在 `Tool` / `ToolSet` 这两个扁平接口上。随着 `macaca-framework`、`macaca-web`、`macaca-driver`、`macaca-skill` 持续消费这些接口，继续在旧 contract 上叠加逻辑只会放大 bridge glue、trace 分散和组合重复。

这轮需要在不破坏现有行为的前提下，把 `macaca-tools` 演进到更稳定的模式化原语上，并把上层消费迁移到新入口：

- `ToolCommandContext`
- `ToolCommand`
- `ToolSchemaProvider`
- `ToolCommandMiddleware`
- `CompositeToolSet`

旧接口保留但标记为 `deprecated`，仅作为过渡兼容层存在，便于后续查找和清理。

## What Changes

- 为 `macaca-tools` 增加 command-style 执行上下文与 canonical 执行入口
- 将 schema 提供抽为独立 provider contract，并保留旧 `parameters_schema()` 兼容入口
- 在 `macaca-tools` 内新增标准 tool command middleware 链，并提供默认 trace middleware
- 将 `orchestration` / `todo` tools 保持为纯业务动作实现，trace 发射迁移到标准执行链
- 为 toolset 增加 composite group 原语，并迁移已知上层聚合实现到新原语
- 对旧 `Tool` / `ToolSet` consumer-facing 入口加 `deprecated` 标记，并迁移仓库内已知调用面
- 继续迁移上层 crate 到 canonical consumer entrypoints，消除仓库内仍在直接调用 deprecated `macaca-tools` 方法的路径
- 保留非 `macaca-tools` 所有者的同名 API，例如 `Driver::tools()`、`Toolkit::get_tool()`、`ToolHandler::execute()`

## Impact

- Affected specs:
  - `macaca-tools-core`
- Affected code:
  - `macaca/crates/macaca-tools/src/tool.rs`
  - `macaca/crates/macaca-tools/src/lib.rs`
  - `macaca/crates/macaca-tools/src/builtin.rs`
  - `macaca/crates/macaca-framework/src/adapter.rs`
  - `macaca/crates/macaca-driver/src/toolset.rs`
  - `macaca/crates/macaca-web/src/lib.rs`
  - `macaca/crates/macaca-integration-tests/src/pipeline_dry_run.rs`
  - `macaca/crates/macaca-runtime/src/agentic_loop.rs`
  - `macaca/crates/macaca-agent/src/agent.rs`
  - `macaca/crates/macaca-agent/src/basic.rs`
  - `macaca/crates/macaca-sdk/src/builder.rs`
  - `macaca/crates/macaca-kernel/src/kernel.rs`
  - `macaca/crates/macaca-kernel/src/registry.rs`
  - `macaca/crates/macaca-kernel/src/scheduler.rs`
  - `macaca/crates/macaca-skill/src/tool.rs`

## Risk

- `Tool` upstream impact: expected `HIGH`
- `ToolSet` upstream impact: expected `CRITICAL`

因此本轮只做 additive-first 迁移：

- 不删除旧接口
- 不直接替换现有运行时语义
- 不改变 `TraceEvent` 数据结构
- 不将 `macaca-tools` 反向耦合到 `macaca-framework`

## Non-Goals

- 不在本轮实现 schema cache
- 不重写 `macaca-framework::Toolkit` 自身的 middleware 模型
- 不改变任何具体 tool 的业务语义
