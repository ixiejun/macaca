# macaca-runtime 设计模式渐进式重构计划

## 当前职责

`macaca-runtime` 包含 agentic loop、context window manager、loop detector、permission checker 等运行时能力。它决定单个 agent 执行时如何处理上下文、工具权限、循环检测和恢复。

重点对象：

- `AgenticLoop`。
- `ContextWindowManager`。
- `LoopDetector`。
- `PermissionChecker`。

## 适用模式

| 位置 | 现状风险 | 设计模式 | 渐进目标 |
| --- | --- | --- | --- |
| agentic loop | 步骤固定但细节复杂，易插入临时逻辑 | Template Method | 固定 loop 骨架和可替换策略 |
| context compaction | 不同模型/任务有不同压缩策略 | Strategy | `ContextCompactionStrategy` |
| permission | 工具权限、用户确认、policy、skill gating 需要串联 | Chain of Responsibility | 权限决策链 |
| loop detection | 启发式可能随模型变化调整 | Strategy | 可替换 loop detector |
| runtime state | Running/Paused/WaitingApproval/Completed | State | 显式运行状态机 |

## 小步重构计划

1. 第一切片：把 `AgenticLoop` 执行阶段命名为 template steps，逻辑不变。
2. 第二切片：抽出 `PermissionDecisionChain`，先包装现有 `PermissionChecker`。
3. 第三切片：为 context window 增加 strategy interface，默认策略保持现状。
4. 第四切片：把 loop detector 参数配置化，避免写死阈值。
5. 第五切片：为 resume/pause 增加 runtime state transition tests。

## 示例代码片段

```rust
pub trait RuntimeStepStrategy: Send + Sync {
    async fn compact_context(&self, ctx: RuntimeContext) -> Result<RuntimeContext, RuntimeError>;
    async fn check_permission(&self, action: &ToolAction) -> Result<PermissionDecision, RuntimeError>;
    async fn detect_loop(&self, history: &LoopHistory) -> Result<LoopDecision, RuntimeError>;
}

impl AgenticLoop {
    pub async fn run(&self, input: RuntimeInput) -> Result<RuntimeOutput, RuntimeError> {
        let ctx = self.prepare_context(input).await?;
        let ctx = self.strategy.compact_context(ctx).await?;
        let action = self.next_action(&ctx).await?;
        self.strategy.check_permission(&action).await?;
        self.execute_action(action).await
    }
}
```

## 验证策略

- 用 deterministic fake model 固定 agentic loop 的 step 序列。
- permission chain 改造前后比较 allow/deny/ask 输出。
- context compaction 用 token budget fixture 做 golden test。

