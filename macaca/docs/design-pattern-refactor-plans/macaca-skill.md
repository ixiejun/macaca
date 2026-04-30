# macaca-skill 设计模式渐进式重构计划

## 当前职责

`macaca-skill` 管理 skill catalog、registry、发现、provision、runtime、tool exposure。它是 Agent OS 对接标准 skill 生态的基础，后续不应只实现最小闭环，而要支撑 7x24 自动运行智能体所需的完整 skill 能力。

重点对象：

- `AgentSkill`。
- `SkillCatalog`。
- `SkillRegistry`。
- discovery/provisioner/runtime/tool。

## 适用模式

| 位置 | 现状风险 | 设计模式 | 渐进目标 |
| --- | --- | --- | --- |
| skill 发现/注册 | 本地目录、下载包、内置 skill 来源不同 | Abstract Factory + Registry | 统一 skill source factory |
| metadata gating | skill 是否暴露给 agent 的条件复杂 | Strategy + Chain of Responsibility | skill exposure policy chain |
| skill tool runtime | skill tool 调用背后可能是 MCP、本地命令、HTTP | Proxy + Adapter | skill tool proxy 屏蔽运行方式 |
| skill lifecycle | installed/provisioned/active/error 状态需明确 | State | `SkillRuntimeState` |
| skill snapshot | 自动运行需要恢复安装/启用状态 | Memento | skill registry snapshot |

## 小步重构计划

1. 第一切片：定义 `SkillExposurePolicy`，把 metadata gating 从调用点抽出。
2. 第二切片：SkillRegistry 增加 snapshot/reload，支持进程重启后恢复。
3. 第三切片：把 skill tool 转换为 framework `ToolHandler` 的逻辑放入 adapter。
4. 第四切片：provisioner 输出 `SkillRuntimeHandle`，封装进程、MCP lease、env。
5. 第五切片：建立 skill contract tests：发现、metadata gating、工具调用、资源释放。

## 示例代码片段

```rust
pub trait SkillExposurePolicy: Send + Sync {
    fn allows(&self, skill: &AgentSkill, ctx: &SkillExposureContext) -> PolicyDecision;
}

pub struct SkillPolicyChain {
    policies: Vec<Box<dyn SkillExposurePolicy>>,
}

impl SkillPolicyChain {
    pub fn evaluate(&self, skill: &AgentSkill, ctx: &SkillExposureContext) -> PolicyDecision {
        for policy in &self.policies {
            if let PolicyDecision::Deny(reason) = policy.allows(skill, ctx) {
                return PolicyDecision::Deny(reason);
            }
        }
        PolicyDecision::Allow
    }
}
```

```rust
pub struct SkillToolAdapter {
    skill_id: String,
    runtime: Arc<dyn SkillRuntimeProxy>,
}
```

## 验证策略

- 用 Playwright MCP skill 做真实 fixture，验证 installed -> provisioned -> active -> released。
- metadata gating 用多 application、多 agent capability fixture 覆盖。
- skill runtime 必须写入 trace event，确保用户能看到 skill tool 调用过程。

