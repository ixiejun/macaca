## Context

当前系统有两层并存的 LLM 抽象：

- `macaca-llm::LlmProvider` / `LlmRouter`
- `macaca-framework::ChatModel`

`refactor-macaca-llm-provider-resolver` 已完成底层第一切片：`macaca-llm` 现在提供 `ProviderResolver`、`PrefixProviderResolver`、`ResolverChain`，并让 `LlmRouter::resolve_target` 通过 resolver chain 推断 provider。旧的 `LlmRouter::resolve_provider_name` 已被标记 deprecated，仅作为迁移期兼容入口。

本轮消费者迁移不再重做底层 resolver，而是把“谁负责 provider/model 选择”收敛到上层消费边界：

- framework/web agent 构建走 `LlmRouter::resolve_selection`
- framework model 执行走 `RoutedLlmAdapter`
- app 层 `LlmProxy` 不再自己实现 user/app/agent 字符串选择逻辑
- `LlmProvider` 继续作为底层执行 trait，不被 deprecated

## Goals / Non-Goals

**Goals**
- framework agent 能以统一方式解析 provider + model
- 支持 app 默认、agent 覆盖、显式模型引用与 fallback chain
- 让 coordinator / planner / worker / review 等 framework-based agents 复用同一条 routed LLM 路径
- 让 app 层 proxy 复用 `ModelSelectionRequest`，不再维护独立模型优先级逻辑
- 保留 `LlmProvider` 作为执行边界，避免把 router 语义强行扩散到 runtime/kernel/task 的纯执行路径

**Non-Goals**
- 不改 agent/tool/event 协议
- 不新增供应商私有功能暴露到 framework trait
- 不在本轮引入 prompt cache / response cache
- 不改变现有 app manifest 的高层语义，只补充更稳定的解析与兼容策略
- 不删除 `LlmProviderAdapter`；它仍可用于 legacy direct-provider integration 和 tests

## Decisions

### Decision 1: 以 `macaca-llm` 作为 provider/model routing 单一来源

`macaca-llm` 负责：

- 根据 config 注册 provider
- 提供 provider lookup
- 提供默认 provider / 默认模型解析
- 提供 fallback chain 构建
- 通过 resolver chain 推断 provider

上层代码需要做 provider/model 选择时，必须使用 `LlmRouter::resolve_selection` 或 routed adapter；仅执行模型调用的底层边界可以继续依赖 `LlmProvider`。

### Decision 2: 引入统一的 `ModelTarget` / `ModelSelection` 语义

需要一个显式结构，而不是把所有选择都塞进字符串：

- `provider`：可选；为空时按 resolver 推断
- `model`：目标模型名
- `fallbacks`：有序候选列表
- `source`：system/app/agent/request，便于调试与审计

兼容输入形式：

- `gpt-4o`
- `anthropic:claude-sonnet-4`
- `openrouter/openai/gpt-4o`
- agent manifest 中的 `model`

resolver 将这些输入收敛成统一的 `ModelSelection`。

### Decision 3: framework 侧默认使用“路由后的 ChatModel”

保留 `ChatModel` 作为 framework 内部统一接口，但默认实现从“单 provider adapter”升级为“router-backed adapter”。

这样：

- `ReActAgent` 不需要理解 provider 注册细节
- framework runner 只传入 `ModelSelection`
- provider/model/fallback 逻辑都在 adapter + router 里处理
- `LlmProviderAdapter` 保留为 legacy adapter，但 framework/web production construction 不应使用它

### Decision 4: fallback 作为 resolution 的一部分，而不是 wrapper 局部配置

当前 fallback 更多依附于 `ResilientLlmWrapper` 的内部配置，且默认围绕单 provider 组织。重构后需要明确：

- fallback first-class 出现在 selection / route plan 中
- primary 失败时可切换到同 provider 不同 model
- 也可按配置切到不同 provider
- trace / logs 中能看出实际命中的 provider/model

### Decision 5: 配置兼容优先，逐步迁移

现有配置仍保留：

- `default_provider`
- `providers.<name>.default_model`

在此基础上新增更清晰的 route 语义，而不是一次性推翻配置格式。兼容阶段：

- 老配置继续可用
- 新配置可声明更明确的 model route / fallback policy
- framework runner 统一走 resolver

### Decision 6: `LlmProxy` 迁移为 router-backed facade

`macaca-app::LlmProxy` 的职责是执行 app defaults / user overrides 的 LLM 代理，但它不应再独立实现模型优先级。迁移后：

- 新构造入口接收 `Arc<LlmRouter>`
- 通过 `ModelSelectionRequest` 表达 user/app/agent 优先级
- `chat` 使用 router 的 `chat_with_selection`
- 旧 `LlmProxy::new(inner, app_defaults, user_overrides)` 保留并标记 deprecated

## Risks / Trade-offs

- 多 provider 路由会增加模型解析复杂度
  - Mitigation: 把解析规则收敛到单独 resolver，并增加表驱动测试
- 兼容旧配置可能造成双语义并存
  - Mitigation: 明确 resolution precedence，并在日志里输出最终解析结果
- `macaca-framework` 与 `macaca-llm` 的边界如果处理不好，会重新耦合
  - Mitigation: framework 仅依赖 `ChatModel` + `ModelSelection`，provider 细节留在 adapter 层

## Migration Plan

1. 确认 `refactor-macaca-llm-provider-resolver` 已完成，作为底层前置切片
2. 补强 framework router-backed `ChatModel` adapter tests
3. 确认 framework runner 所有 traced / worker / coordinator builder 都走 routed model
4. 迁移 `LlmProxy` 到 router-backed selection
5. 增加兼容测试：
   - 旧配置
   - agent model override
   - cross-provider fallback
   - app proxy provider/model override
6. 保留 legacy adapter / constructor，但用 deprecation marker 标出迁移方向

## Open Questions

- 是否需要把 route policy 暴露到 app manifest，允许 app 为不同 agent 预声明 fallback chain
- 是否要在 EventLog / trace 中增加实际 provider/model 命中信息，便于排障
