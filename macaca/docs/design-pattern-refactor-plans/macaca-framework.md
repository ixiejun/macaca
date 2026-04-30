# macaca-framework 设计模式渐进式重构计划

## 当前职责

`macaca-framework` 是 Agent OS 的核心 agent framework，包含 Agent trait、ReActAgent、HookedAgent、Toolkit、ToolMiddleware、Pipeline、Memory、PlanNotebook、SessionStore、formatter、adapter 等通用能力。后续所有 application agent 都应该尽量通过这里的 primitive 执行，而不是走 web/kernel 的临时分支逻辑。

## 已经具备的模式

- Decorator：`HookedAgent` 包装 Agent 注入 hook。
- Chain of Responsibility：`ToolMiddleware` 串联工具调用前后逻辑。
- Strategy：`ChatModel`、formatter、tool handler 都有可替换抽象。
- Composite：Pipeline、message/content block、tool group。
- Memento：SessionStore、PlanNotebook、state module。

## 需要继续强化的模式

| 位置 | 现状风险 | 设计模式 | 渐进目标 |
| --- | --- | --- | --- |
| `ReActAgent` 主循环 | reasoning、tool call、memory、trace、finish 条件交织 | Template Method | 固定 loop 骨架，拆出可替换 step |
| Tool middleware | trace、permission、MCP、skill gating 容易重复实现 | Chain of Responsibility + Decorator | 标准 middleware helper，所有 agent 统一走一套 |
| Message/content 展示 | UI markdown/json/trace 展示逻辑容易散到前端和 web | Visitor | 后端提供结构化展示 visitor，前端只渲染 |
| PlanNotebook | 与 TodoBoard 边界需要持续保持清晰 | Memento + Facade | PlanNotebook 保存 agent 脑内计划，TodoBoard 保存正式任务账本 |
| Agent 构建入口 | 历史 `build_agent` 风格容易绕过 trace | Abstract Factory | 只保留 traced agent factory，旧入口废弃并最终删除 |

## 小步重构计划

1. 第一切片：给 `ReActAgent` 内部 loop 增加私有 step 方法，不改变外部 trait。
2. 第二切片：把 trace hook 和 tool trace middleware 抽成 framework-level helper，web 侧只配置参数。
3. 第三切片：新增 `AgentFactory` trait，默认实现创建 traced ReActAgent。
4. 第四切片：给 PlanNotebook 增加正式职责注释和测试：它不是 TodoBoard，不能 claim/review task。
5. 第五切片：引入 `MessageVisitor`，先服务 event persistence 和 UI trace formatting。

## 示例代码片段

### Template Method 拆 ReAct loop

```rust
impl ReActAgent {
    async fn run_loop(&self, input: Msg, ctx: AgentLoopContext) -> AgentResult<Msg> {
        self.before_loop(&ctx).await?;

        for iter in 0..self.max_iters {
            let thought = self.reason(iter, &ctx).await?;
            let action = self.select_action(thought, &ctx).await?;
            let observation = self.execute_action(action, &ctx).await?;

            if let Some(done) = self.try_finish(observation, &ctx).await? {
                return Ok(done);
            }
        }

        self.on_loop_exhausted(ctx).await
    }
}
```

### 标准 traced agent factory

```rust
pub trait AgentFactory {
    type Agent: Agent;

    async fn build_traced_agent(
        &self,
        spec: AgentSpec,
        trace: TraceContext,
    ) -> Result<Self::Agent, AgentBuildError>;
}

#[deprecated(note = "Use build_traced_agent; untraced agents are not allowed in Agent OS")]
pub async fn build_agent(_: AgentSpec) -> ! {
    panic!("untraced agent entry is disabled")
}
```

### Visitor 处理内容展示

```rust
pub trait ContentVisitor<R> {
    fn visit_text(&mut self, text: &TextBlock) -> R;
    fn visit_json(&mut self, value: &serde_json::Value) -> R;
    fn visit_tool_call(&mut self, call: &ToolCallBlock) -> R;
}
```

## 验证策略

- 用 ReActAgent fixture 对比 loop 拆分前后的 tool call 序列。
- 每个 application agent 构建时断言存在 trace hook 和 tool trace middleware。
- 添加“所有 agent 必须 traced”测试：没有 trace context 的 factory 构建直接失败。

