## 1. Implementation

- [x] 1.1 新增 `framework_toolkit` web 内部模块
- [x] 1.2 将 `TodoToolPolicy`、`AgentToolPolicy`、`resolve_tool_policy` 移入新模块
- [x] 1.3 将 `build_toolkit`、`register_agent_tools` 移入新模块，并保持注册顺序和参数不变
- [x] 1.4 将 workspace `file_read` / `file_write` / `shell` tool 及输入/path helper 移入新模块
- [x] 1.5 更新 `FrameworkRunner` 调用点为新模块 helper，保持 traced agent/coordinator/runtime agent 构建行为不变

## 2. Tests

- [x] 2.1 迁移或保留现有 tool output helper 单元测试
- [x] 2.2 添加或更新 unit test，覆盖 workspace 相对路径解析行为
- [x] 2.3 添加或更新 unit test，覆盖 stringified JSON tool input normalize 行为

## 3. Verification

- [x] 3.1 运行 `openspec validate refactor-tool-policy-toolkit-module --strict`
- [x] 3.2 运行 framework toolkit 相关单元测试
- [x] 3.3 运行 `cargo check -p macaca-web`
- [x] 3.4 运行 GitNexus `detect_changes(scope=staged)` 并确认影响范围符合本次局部重构
