# macaca-integration-tests 设计模式渐进式重构计划

## 当前职责

`macaca-integration-tests` 保存端到端和集成测试，验证 LLM provider、agent loop、web/session/task 流程是否连通。这个 crate 本身不承载生产逻辑，但决定渐进式重构能不能安全推进。

## 适用模式

| 位置 | 现状风险 | 设计模式 | 渐进目标 |
| --- | --- | --- | --- |
| 测试场景搭建 | 每个测试手写 session/app/task 数据，重复且易漏字段 | Test Data Builder | 统一构建 fixture |
| 多步骤 E2E | 测试流程长，断言点散 | Template Method | 固定 arrange/act/assert/replay 阶段 |
| application fixture | 不同应用测试需要复用一组基础配置 | Prototype | 从原型复制后局部覆盖 |
| fake LLM/tool | mock 行为分散 | Strategy | 可插拔 fake provider/tool strategy |

## 小步重构计划

1. 第一切片：新增 `SessionScenarioBuilder`，只服务一个现有测试。
2. 第二切片：把“创建 session -> 发送消息 -> 等待 event -> 查询 todo”的流程抽成 `E2eScenario`。
3. 第三切片：引入 fake LLM strategy，支持快速 deterministic decomposition/review。
4. 第四切片：把关键回归问题固化为场景：实时 trace、刷新恢复、任务依赖、resume coordinator。

## 示例代码片段

```rust
pub struct SessionScenarioBuilder {
    app_id: ApplicationId,
    user_prompt: String,
    fake_llm: Option<FakeLlmScript>,
}

impl SessionScenarioBuilder {
    pub fn fullstack_hello() -> Self {
        Self {
            app_id: ApplicationId::from_static("FULLSTACK-AUTODEV"),
            user_prompt: "创建一个最小全栈 hello 页面".into(),
            fake_llm: None,
        }
    }

    pub async fn run(self) -> E2eScenarioResult {
        E2eScenario::from_builder(self).execute().await
    }
}
```

## 验证策略

- 每个生产重构切片都至少补一个 integration fixture。
- 优先把历史 bug 写成 regression test，例如 duplicate event、planner review 覆盖 trace、frontend claim 卡住。
- 测试 builder 本身必须保持薄，不引入生产逻辑判断。
