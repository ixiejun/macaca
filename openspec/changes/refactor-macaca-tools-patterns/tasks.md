## 1. Spec And Baseline

- [x] 1.1 创建 `refactor-macaca-tools-patterns` proposal / design / tasks / delta spec
- [x] 1.2 盘点 `macaca-tools` 当前 contract 与上层桥接调用面
- [x] 1.3 对 `Tool`、`ToolSet`、`TraceEvent` 等核心 symbol 运行 GitNexus impact，并记录风险

## 2. Command And Schema Primitives

- [x] 2.1 新增 `ToolCommandContext` 与 `ToolCommand`
- [x] 2.2 新增 `ToolSchemaProvider` 与 `ToolCommandExecutor`
- [x] 2.3 为现有 `Tool` 提供到新原语的 blanket adapter
- [x] 2.4 将旧 `parameters_schema` / `execute` / `execute_streaming` 标记 `deprecated`，但继续兼容

## 3. Middleware And Trace

- [x] 3.1 新增 `ToolCommandMiddleware` 与 `ToolCommandPipeline`
- [x] 3.2 新增默认 `TraceToolCommandMiddleware`，保持 `tool_call -> tool_result` 顺序
- [x] 3.3 迁移 `macaca-framework` bridge 到 canonical command + schema 入口
- [x] 3.4 确保 concrete tools 不需要手写标准 trace 发射逻辑

## 4. Business Tool Boundary

- [x] 4.1 收口 `orchestration` tools 到业务动作实现，不让其承担新的横切职责
- [x] 4.2 收口 `todo` tools 到业务动作实现，不让其承担新的横切职责
- [x] 4.3 为旧业务工具 consumer-facing 辅助入口添加 `deprecated` 标记（如有必要）

## 5. Composite ToolSet

- [x] 5.1 新增 `CompositeToolSet` / `ToolCatalog` 原语
- [x] 5.2 将旧 `ToolSet::tools` / `to_definitions` 标记 `deprecated`，并提供新 canonical 查询入口
- [x] 5.3 迁移 `macaca-web` 的本地 `CompositeToolSet`
- [x] 5.4 迁移 `macaca-driver::DriverToolSet`
- [x] 5.5 迁移 `macaca-integration-tests::LocalToolSet`

## 6. Verification

- [x] 6.1 运行 `openspec validate refactor-macaca-tools-patterns --strict`
- [x] 6.2 运行 `cargo test -p macaca-tools -- --nocapture`
- [x] 6.3 运行 `cargo check -p macaca-tools`
- [x] 6.4 运行 `cargo check -p macaca-framework -p macaca-web -p macaca-driver -p macaca-skill -p macaca-integration-tests`
- [x] 6.5 运行 workspace `cargo check`
- [x] 6.6 运行 `gitnexus_detect_changes(scope: "all")`
- [x] 6.7 更新 tasks.md 使其与真实状态一致

## 7. Upper-Layer Consumer Migration

- [x] 7.1 迁移 `macaca-skill` tests 中直接调用 deprecated `Tool::execute` 的路径
- [x] 7.2 收口 `macaca-runtime` tool definitions / command execution 到 canonical helper
- [x] 7.3 将 `macaca-agent` / `macaca-sdk` / `macaca-kernel` 中安全的 run contract 迁移到 `ToolCatalog`
- [x] 7.4 将 `macaca-web` shared tool state 从 `ToolSet` consumer 迁移到 `ToolCatalog`
- [x] 7.5 分类保留非 `macaca-tools` 所有者的同名 API：`Driver::tools()`、`Toolkit::get_tool()`、`ToolHandler::execute()`
- [x] 7.6 运行 deprecated-call containment grep，并确认剩余匹配只出现在兼容层或非 `macaca-tools` API
- [x] 7.7 运行 `cargo test -p macaca-skill -- --nocapture`
- [x] 7.8 运行 `cargo check -p macaca-runtime -p macaca-agent -p macaca-sdk -p macaca-kernel -p macaca-framework -p macaca-driver -p macaca-web -p macaca-integration-tests`
- [x] 7.9 运行 workspace `cargo check`
- [x] 7.10 运行 `gitnexus_detect_changes(scope: "all")`
