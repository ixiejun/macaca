# Change: 拆分 tool policy / build toolkit 代码

## Why

`macaca-web/src/framework_runner.rs` 现在同时负责 agent 构建、model selection、trace middleware、tool policy 决策、Toolkit 构建、workspace tool 实现和 todo tool 注册。`AgentToolPolicy`、`TodoToolPolicy`、`build_toolkit`、`register_agent_tools` 已经是 capability-driven，但继续留在 `framework_runner.rs` 会让后续 framework 迁移和 tool policy 调整难以 review。

本次只做 web 内部模块拆分，把 tool policy / toolkit 构建胶水代码搬到独立模块，保持外部行为 1:1 不变。

## What Changes

- 新增 `macaca-web` 内部模块承载 tool policy 与 toolkit 构建逻辑。
- 从 `framework_runner.rs` 移出：
  - `TodoToolPolicy`
  - `AgentToolPolicy`
  - `build_toolkit`
  - `resolve_tool_policy`
  - `register_agent_tools`
  - workspace `file_read` / `file_write` / `shell` tool 及其输入/path helper
- `FrameworkRunner` 继续通过同名/等价 helper 构建 toolkit，调用点保持不变。
- 保持现有行为不变：
  - base tool allowlist 与 unregister 行为不变
  - workspace tool 的名称、schema、相对路径解析、错误文案、shell timeout 不变
  - goal manager / planner / worker 的 todo 工具集合不变
  - capability fallback、entry-agent fallback、disallowed assignee 策略不变
  - `create_goal` 的 goal_to_session、ExecutionContext pause 和 run_trace 行为不变

## Non-Goals

- 不改变任何 agent 的工具可见性
- 不改变 planner/worker/coordinator 的 framework agent 构建入口
- 不改变 EventLog、SSE、run_trace、browser refresh restore 行为
- 不把 tool policy 下沉到 `macaca-framework`
- 不引入 application 专属逻辑

## Impact

- Affected specs: `framework-toolkit`
- Affected code:
  - `macaca/crates/macaca-web/src/framework_runner.rs`
  - 新增 `macaca/crates/macaca-web/src/framework_toolkit.rs`
  - `macaca/crates/macaca-web/src/lib.rs`
- GitNexus impact:
  - `build_toolkit` upstream risk is `CRITICAL`
  - `register_agent_tools` upstream risk is `CRITICAL`
  - `resolve_tool_policy` upstream risk is `CRITICAL`
  - workspace tool structs are also on the `build_toolkit` path
- Risk mitigation:
  - 本次只移动代码和替换 module path，不改变 tool 构造参数、注册顺序、allowlist 过滤或错误文案。
  - 使用 unit test 锁定 workspace path/input helper 与 tool output helper 行为。
  - 使用 `cargo check -p macaca-web` 和 GitNexus `detect_changes(scope=staged)` 验证影响范围。
