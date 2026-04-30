# macaca-proto 设计模式渐进式重构计划

## 当前职责

`macaca-proto` 定义跨 crate 共享的数据结构、配置、错误、事件和 orchestration DTO。它是系统边界语言，应该尽量稳定、纯粹、低业务逻辑。

重点对象：

- Agent/task/session/event DTO。
- Config DTO。
- Error DTO。
- Orchestration event 枚举。

## 适用模式

| 位置 | 现状风险 | 设计模式 | 渐进目标 |
| --- | --- | --- | --- |
| 大量 enum event | 转换、展示、持久化逻辑容易写在 match 中 | Visitor | 行为从 DTO 中移出 |
| config 构造 | 测试和调用侧需要填大量字段 | Builder | typed config builder |
| 状态枚举 | task/agent/session 状态合法转移不清晰 | State | 状态语义文档化并由上层 state machine 执行 |
| error 转换 | 各 crate 自己转错误，用户可见信息不一致 | Adapter | `ProtoErrorAdapter` |

## 小步重构计划

1. 第一切片：给核心 event enum 增加 visitor trait，不改 enum 字段。
2. 第二切片：为 config DTO 增加 builder，仅在测试和新代码使用。
3. 第三切片：在 proto 文档中明确 DTO 不承载运行时策略，策略放 framework/task/kernel。
4. 第四切片：统一错误 display code，保证 web/API/trace 展示一致。

## 示例代码片段

```rust
pub trait AgentExecutionEventVisitor<R> {
    fn thinking(&mut self, iteration: usize, content: Option<&str>) -> R;
    fn tool_call(&mut self, tool_name: &str, input: &serde_json::Value) -> R;
    fn tool_result(&mut self, tool_name: &str, output: &str, is_error: Option<bool>) -> R;
    fn assistant(&mut self, content: &str) -> R;
    fn unknown(&mut self, event: &AgentExecutionEvent) -> R;
}

impl AgentExecutionEvent {
    pub fn accept<R>(&self, visitor: &mut dyn AgentExecutionEventVisitor<R>) -> R {
        match self {
            AgentExecutionEvent::Thinking { iteration, content } => {
                visitor.thinking(*iteration, content.as_deref())
            }
            AgentExecutionEvent::ToolCall { tool_name, tool_input, .. } => {
                visitor.tool_call(tool_name, tool_input)
            }
            AgentExecutionEvent::ToolResult { tool_name, output, is_error } => {
                visitor.tool_result(tool_name, output, *is_error)
            }
            AgentExecutionEvent::Assistant { content } => visitor.assistant(content),
            other => visitor.unknown(other),
        }
    }
}
```

## 验证策略

- DTO 序列化 snapshot 不能因为 visitor/builder 引入发生变化。
- config builder 输出与手写 struct 构造完全一致。
- 修改 proto enum 前必须评估所有 crate 的反序列化兼容性。
