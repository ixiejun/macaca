## Context

当前系统有两层并存但未完全打通的 LLM 抽象：

- `macaca-llm::LlmProvider` / `LlmRouter`
- `macaca-framework::ChatModel`

但实际运行链路里，`macaca-web` 启动时仍直接构造单个 provider，再通过 `LlmProviderAdapter` 交给 framework。这样 framework 虽然“看起来抽象了模型”，实际上仍受限于单 provider 入口。

与此同时，配置层已经具备多 provider 信息，`macaca-llm` 也已有 router 雏形，因此这轮重构的目标不是重新发明一套模型层，而是让：

- provider 注册归位到 `macaca-llm`
- model resolution 归位到一处
- framework 只依赖统一的 routed `ChatModel`

## Goals / Non-Goals

**Goals**
- provider 初始化不再散落在 `macaca-web::start_server`
- framework agent 能以统一方式解析 provider + model
- 支持新增 provider 时仅新增 provider 注册与配置，不必修改 framework runner
- 支持 app 默认、agent 覆盖、显式模型引用与 fallback chain
- 让 coordinator / planner / worker / review 等全部复用同一条 routed LLM 路径

**Non-Goals**
- 不改 agent/tool/event 协议
- 不新增供应商私有功能暴露到 framework trait
- 不在本轮引入 prompt cache / response cache
- 不改变现有 app manifest 的高层语义，只补充更稳定的解析与兼容策略

## Decisions

### Decision 1: 以 `macaca-llm` 作为 provider registry 单一来源

`macaca-llm` 负责：

- 根据 config 注册 provider
- 提供 provider lookup
- 提供默认 provider / 默认模型解析
- 提供 fallback chain 构建

`macaca-web` 只负责调用初始化，不再自己 `match` provider 名。

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

### Decision 3: framework 侧只暴露“路由后的 ChatModel”

保留 `ChatModel` 作为 framework 内部统一接口，但默认实现从“单 provider adapter”升级为“router-backed adapter”。

这样：

- `ReActAgent` 不需要理解 provider 注册细节
- framework runner 只传入 `ModelSelection`
- provider/model/fallback 逻辑都在 adapter + router 里处理

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

## Risks / Trade-offs

- 多 provider 路由会增加模型解析复杂度
  - Mitigation: 把解析规则收敛到单独 resolver，并增加表驱动测试
- 兼容旧配置可能造成双语义并存
  - Mitigation: 明确 resolution precedence，并在日志里输出最终解析结果
- `macaca-framework` 与 `macaca-llm` 的边界如果处理不好，会重新耦合
  - Mitigation: framework 仅依赖 `ChatModel` + `ModelSelection`，provider 细节留在 adapter 层

## Migration Plan

1. 在 `macaca-llm` 中补齐 registry / resolver / route plan 能力
2. 新增 framework router-backed `ChatModel` adapter
3. 修改 `macaca-web` bootstrap，统一从 config 初始化 router
4. 修改 framework runner，所有 traced / worker / coordinator builder 都改走 routed model
5. 增加兼容测试：
   - 旧配置
   - agent model override
   - cross-provider fallback
6. 移除 `macaca-web` 中硬编码 provider 构造分支

## Open Questions

- 是否需要把 route policy 暴露到 app manifest，允许 app 为不同 agent 预声明 fallback chain
- 是否要在 EventLog / trace 中增加实际 provider/model 命中信息，便于排障
