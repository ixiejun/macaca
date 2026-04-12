## 1. Implementation

- [x] 1.1 在 `framework_runner.rs` 中新增私有 helper：从 `ToolResponse` 提取 text content，并复用 `truncate_tool_output`
- [x] 1.2 将 `SseToolMiddleware::after` 切换为使用 helper，保持 SSE/EventLog payload 完全不变
- [x] 1.3 将 `ChannelToolMiddleware::after` 切换为使用 helper，保持 `AgentExecutionEvent::ToolResult` 完全不变
- [x] 1.4 将 `ExecutorToolMiddleware::after` 切换为使用 helper，保持 `ExecutorEvent::AgentEvent` 完全不变
- [x] 1.5 如 diff 足够小，抽取 `AgentExecutionEvent::ToolCall` / `ToolResult` 构造 helper；否则保留原地构造，避免扩大范围

## 2. Tests

- [x] 2.1 保留现有 UTF-8 安全截断测试
- [x] 2.2 添加 tool response 多 text block 提取测试
- [x] 2.3 添加或确认空 tool response 输出行为不变

## 3. Verification

- [x] 3.1 运行 `cargo test -p macaca-web truncate_tool_output -- --nocapture`
- [x] 3.2 运行新增 helper 相关单元测试
- [x] 3.3 运行 `cargo check -p macaca-web`
- [x] 3.4 确认本 change 未修改 SSE/EventLog event 名称和 payload schema
