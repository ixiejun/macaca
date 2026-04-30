# macaca-tools 设计模式渐进式重构计划

## 当前职责

`macaca-tools` 定义系统工具、工具集合、trace event、默认工具集、orchestration/todo tools 等。它是 agent 实际改变世界和观察世界的执行层。

重点对象：

- `Tool` trait。
- `ToolSet` / `DefaultToolSet`。
- `TraceEvent`。
- orchestration/todo 工具。

## 适用模式

| 位置 | 现状风险 | 设计模式 | 渐进目标 |
| --- | --- | --- | --- |
| tool execute | tool call 天然是命令，但当前可能混杂 schema/执行/trace | Command | `ToolCommand` 统一执行上下文 |
| toolset | 多个 tool group 组合和过滤 | Composite | tool group tree |
| framework tool adapter | `macaca-tools` 与 framework toolkit 需要映射 | Adapter | `FrameworkToolAdapter` |
| trace/permission/mcp | 横切逻辑不能散到每个 tool | Decorator + Chain of Responsibility | tool middleware 链 |
| tool schema | schema 生成和缓存可复用 | Flyweight | tool schema cache |

## 小步重构计划

1. 第一切片：新增 `ToolCommandContext`，先作为 execute 参数聚合对象。
2. 第二切片：抽出 `ToolSchemaProvider`，schema 生成和 execute 解耦。
3. 第三切片：将 trace emission 移入标准 tool middleware，不在具体工具中手写。
4. 第四切片：让 orchestration/todo tools 只表达业务动作，不关心 SSE/EventLog。
5. 第五切片：toolset 支持 composite group，方便 skill/MCP/application tools 合并。

## 示例代码片段

```rust
pub struct ToolCommand {
    pub tool_name: String,
    pub args: serde_json::Value,
    pub context: ToolCommandContext,
}

pub trait ToolExecutor: Send + Sync {
    async fn execute(&self, command: ToolCommand) -> Result<ToolOutput, ToolError>;
}

pub struct CompositeToolSet {
    groups: Vec<Arc<dyn ToolSet>>,
}
```

## 验证策略

- 每个 tool 保留 schema snapshot。
- tool middleware 引入后，trace event 顺序必须保持：tool_call -> tool_result。
- orchestration/todo tools 需要 session scope regression test。

