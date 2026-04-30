# Design: macaca-proto 渐进式设计模式重构

## Context

`macaca-proto` 是 Agent OS 的共享边界语言。它不应承担 runtime strategy，也不适合做侵入式重构。本设计采用与 `macaca-agent` / `macaca-app` 相同的渐进策略：

- 先补行为锁定
- 再新增抽象
- 再让新抽象服务新代码和测试
- 最后才考虑逐步迁移调用侧

本 change 参考的设计模式：

- `Visitor`：把 event 展示/转换/持久化入口从散落 `match` 收敛到访问协议
- `Builder`：为高频 config DTO 提供 typed builder，降低构造噪音
- `Adapter`：统一错误 display/code 适配入口
- `State`：只在 proto 层文档化状态语义，真正状态转移仍在上层 state machine 执行

## Goals

- 降低上层 crate 对 proto event 直接散落 `match` 的耦合增长速度
- 为测试和新增代码提供稳定的 config builder
- 统一 proto error 的用户可见展示入口
- 明确 proto 只承载 contract，不承载 runtime strategy
- 保持 serde 兼容和上层行为 1:1 不变

## Non-Goals

- 不改变 `AgentExecutionEvent`、task/session/event DTO 的字段布局
- 不在 proto 中引入 planner / worker / coordinator 运行时逻辑
- 不在本 change 中迁移所有调用侧到 visitor 或 builder
- 不把错误恢复策略放到 proto 中

## Proposed Design

### 1. Event Visitor

第一切片只增加访问接口，不改现有 enum。

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

这个切片的关键约束：

- 旧 `match` 写法仍然可用
- `accept()` 只是 additive API
- 不能改变 serde 或 enum 语义

如果其他高频 event enum 也存在相同问题，应采用相同模式，但必须逐个小切片推进。

### 2. Config Builder

builder 只为高频 DTO 增加低噪音构造路径，不删除手写 struct 初始化能力。

```rust
pub struct AppConfigBuilder {
    inner: AppConfig,
}

impl AppConfigBuilder {
    pub fn new() -> Self {
        Self { inner: AppConfig::default() }
    }

    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.inner.model = model.into();
        self
    }

    pub fn permission(mut self, permission: PermissionMode) -> Self {
        self.inner.permission = permission;
        self
    }

    pub fn build(self) -> AppConfig {
        self.inner
    }
}
```

优先级：

- 先挑“测试和调用侧最常手写默认字段”的 DTO
- 不一次性给所有 DTO 全补 builder
- builder 默认值必须与现有 `Default` / 手写常用初始化结果一致

### 3. Proto Error Adapter

目标不是新建复杂错误体系，而是统一“展示和编码入口”。

```rust
pub trait ProtoErrorAdapter {
    fn code(&self) -> &'static str;
    fn display_message(&self) -> String;
}
```

约束：

- 旧错误类型和 `Display` 语义不破坏
- adapter 只收敛一致展示入口
- 恢复策略、HTTP status、retry policy 仍属于上层

### 4. DTO / Runtime Boundary

在 spec 中明确：

- `macaca-proto` 负责定义 DTO、event、config、error contract
- `macaca-task` 负责 task lifecycle 和 planning state machine
- `macaca-framework` 负责 traced agent construction、tool/runtime bridge
- `macaca-kernel` 负责 session/resume/executor/loop orchestration

这样可以阻止后续把 runtime policy 继续写回 proto。

## Compatibility Rules

- 不修改任何现有字段名、serde rename、枚举 variant 名称
- 不删除旧构造方式
- visitor / builder / adapter 均为 additive API
- 所有新抽象必须通过现有测试或新增黄金测试锁定兼容性

## Migration Order

1. 锁定当前 DTO 序列化、默认值和错误展示行为
2. 增加核心 event visitor
3. 增加第一批高频 config builder
4. 增加统一错误适配入口
5. 更新 spec，固定 proto 与 runtime 的边界
6. 后续独立 change 再逐步迁移上层消费方

## Verification

- `cargo test -p macaca-proto`
- `cargo check -p macaca-proto`
- 如 public API 变动引发调用侧编译影响，再运行 workspace `cargo check`
- 对关键 DTO 做序列化兼容测试，确认 visitor / builder 不改变 wire format
- GitNexus:
  - 实施前对 `AgentExecutionEvent`、高频 config DTO、proto error 类型运行 upstream impact
  - 提交前运行 `gitnexus_detect_changes(scope: "all")`

