# macaca-sdk 设计模式渐进式重构计划

## 当前职责

`macaca-sdk` 提供外部开发者声明 Agent、persona、registry API 和 builder 的接口。它决定应用和插件开发者如何接入 Agent OS。

重点对象：

- `AgentBuilder`。
- `DeclarativeAgent`。
- Persona/config。
- registry API。

## 适用模式

| 位置 | 现状风险 | 设计模式 | 渐进目标 |
| --- | --- | --- | --- |
| agent 声明 | builder 已存在，但与 framework traced factory 需要打通 | Builder + Abstract Factory | SDK builder 最终产出 traced agent spec |
| persona 模板 | 多 agent 复用 persona 需要复制改字段 | Prototype | persona prototype clone + override |
| registry API | SDK 调用者不应知道内核注册细节 | Facade | `MacacaSdk` 门面 |
| validation | allowed tools、skills、driver、MCP 权限需要组合校验 | Chain of Responsibility | validation chain |

## 小步重构计划

1. 第一切片：让 `AgentBuilder` 输出 `AgentSpec`，不直接绑定具体 runtime。
2. 第二切片：增加 persona prototype API，支持 clone 后覆盖 identity/tools。
3. 第三切片：新增 `SdkValidationChain`，拆开 manifest、tool、skill、driver 校验。
4. 第四切片：SDK 注册 agent 时强制携带 trace policy，保证不会生成 untraced agent。

## 示例代码片段

```rust
pub struct PersonaPrototype {
    base: AgentPersona,
}

impl PersonaPrototype {
    pub fn instantiate(&self, overrides: PersonaOverrides) -> AgentPersona {
        self.base.clone().apply(overrides)
    }
}

pub struct MacacaSdk {
    registry: Arc<dyn AgentRegistryApi>,
    validator: SdkValidationChain,
}
```

## 验证策略

- builder snapshot：同一 declarative config 输出同一 AgentSpec。
- persona prototype clone 后不会修改原型。
- SDK 注册的 agent 在 runtime 中必须能看到 trace context。

