# macaca-app 设计模式渐进式重构计划

## 当前职责

`macaca-app` 负责 application manifest 加载、runtime 构建、workflow prompt 生成和应用级配置解释。它承接“一个 application 如何被 Agent OS 运行”的声明式边界。

重点对象：

- `AppLoader`：读取和解析应用配置。
- `AppRuntime`：应用运行时结构。
- `WorkflowEngine`：根据应用定义生成默认 workflow prompt。
- Manifest 与 agent 配置：描述 entry agent、agents、allowed tools、capability 等。

## 适用模式

| 位置 | 现状风险 | 设计模式 | 渐进目标 |
| --- | --- | --- | --- |
| manifest 到 runtime | 解析、默认值、校验、运行时装配可能互相穿插 | Builder | 用 `AppRuntimeBuilder` 分离 parse / validate / assemble |
| workflow prompt | 默认 prompt 存在工具名和行为硬编码风险 | Template Method + Strategy | 固定 prompt 骨架，应用差异由 strategy/provider 注入 |
| application startup | loader、runtime、workflow、agents 装配边界不够集中 | Abstract Factory | 为 application runtime 创建统一工厂 |
| agent capability 聚合 | agent 能力来自 yaml、skill、driver、tools，容易散落 | Composite | 形成 application-level capability tree |

## 小步重构计划

1. 第一切片：新增 `AppRuntimeBuilder`，只承接现有构造逻辑，不改 manifest 字段。
2. 第二切片：抽出 `WorkflowPromptParts`，把当前默认 prompt 拆为 role、constraints、tools、handoff 四段。
3. 第三切片：引入 `WorkflowPromptStrategy`，默认实现保持现有输出一致。
4. 第四切片：把 driver/tool 选择规则从字符串模板迁移到 capability provider，不再在 prompt 中写死单个 driver。
5. 第五切片：增加应用配置快照测试，确保 `FULLSTACK-AUTODEV`、`NEWSROOM-AUTOWRITER` 的 runtime 输出不变。

## 示例代码片段

### Builder 分离装配

```rust
pub struct AppRuntimeBuilder {
    manifest: ApplicationManifest,
    defaults: RuntimeDefaults,
}

impl AppRuntimeBuilder {
    pub fn validate(&self) -> Result<(), AppConfigError> {
        self.manifest.validate_entry_agent()?;
        self.manifest.validate_agent_tools()?;
        Ok(())
    }

    pub fn build(self) -> Result<AppRuntime, AppConfigError> {
        self.validate()?;
        Ok(AppRuntime::new(self.manifest, self.defaults))
    }
}
```

### Template Method + Strategy 生成 workflow prompt

```rust
pub trait WorkflowPromptStrategy: Send + Sync {
    fn render_tools(&self, ctx: &WorkflowPromptContext) -> String;
    fn render_handoff_rules(&self, ctx: &WorkflowPromptContext) -> String;
}

pub struct WorkflowPromptTemplate<S> {
    strategy: S,
}

impl<S: WorkflowPromptStrategy> WorkflowPromptTemplate<S> {
    pub fn render(&self, ctx: &WorkflowPromptContext) -> String {
        format!(
            "{role}\n\n{tools}\n\n{handoff}",
            role = ctx.role(),
            tools = self.strategy.render_tools(ctx),
            handoff = self.strategy.render_handoff_rules(ctx),
        )
    }
}
```

## 验证策略

- 建立 manifest fixture，对比 builder 引入前后的 `AppRuntime` debug snapshot。
- 对 prompt strategy 做 snapshot test，确保默认 prompt 字面输出保持一致。
- 后续修改 `WorkflowEngine` 前必须先跑 GitNexus impact，因为 workflow prompt 会影响 planner/coordinator 行为。

