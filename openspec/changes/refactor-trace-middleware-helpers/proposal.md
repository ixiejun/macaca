# Change: 统一 trace middleware helper

## Why

`macaca-web/src/framework_runner.rs` 中 `SseToolMiddleware`、`ChannelToolMiddleware`、`ExecutorToolMiddleware` 都在重复处理同一类 trace glue：

- 从 `ToolResponse` 的 text content block 中拼接输出文本
- 使用 UTF-8 安全截断逻辑生成展示输出
- 构造 `AgentExecutionEvent::ToolCall` / `AgentExecutionEvent::ToolResult` 或 SSE/EventLog payload

这些重复代码已经暴露过风险：工具输出包含多字节字符时，如果某个路径继续使用 byte slicing，就可能再次触发 UTF-8 边界 panic。第一步应该只把这部分重复逻辑收敛成私有 helper，保持所有外部行为 1:1 不变。

## What Changes

- 在 `framework_runner.rs` 中新增私有 helper：
  - 从 `ToolResponse` 提取 text content
  - 复用现有 `truncate_tool_output`
  - 可控范围内抽取 `AgentExecutionEvent` 的 tool call/result 构造
- 让 `SseToolMiddleware`、`ChannelToolMiddleware`、`ExecutorToolMiddleware` 复用这些 helper
- 保留现有 event 名称、payload 字段、source、task_id/agent 绑定和 truncation limit
- 补充或保留单元测试，覆盖 UTF-8 截断和 tool response 文本提取

## Non-Goals

- 不改变 coordinator、planner、worker 的执行链路
- 不改变 PlanLoop/WorkerLoop 调度、review、resume 语义
- 不改变 SSE event 名称或 payload schema
- 不改变 EventLog event 名称、source 或 payload schema
- 不把 helper 移动到 `macaca-framework` crate；本轮只做 web 内部低风险收敛
- 不清理其他 `loop_manager.rs` 重复代码

## Impact

- Affected specs: `framework-trace-middleware`
- Affected code:
  - `macaca/crates/macaca-web/src/framework_runner.rs`
- Expected risk: Low
- Behavioral compatibility:
  - Live SSE trace event 输出保持不变
  - EventLog 持久化内容保持不变
  - 刷新浏览器后的历史 trace 恢复保持不变
  - Worker/planner/coordinator task lifecycle 不变
