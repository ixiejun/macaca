# Design: LLM / Memory / Context Services v1

## Context

Route C 的微内核边界规定：LLM provider、Memory / Context Engine 不得进入 kernel，必须作为 System Service。S5 需要把已有 LLM router/provider、Memory facade/runtime、Context composer/engine/active recall 能力转成可替换服务，同时不破坏现有 chat、framework runner、context report、active recall 和 trace 行为。

当前系统已经具备 `ServiceRuntime`、`SystemFacade`、task service 化经验、memory/context 基础能力，因此本提案不重新发明运行时，而是复用已有 service runtime 和 SDK facade，新增三个聚焦服务边界。

## Goals

- 建立 provider-neutral 的 LLM、Memory、Context 服务契约。
- 让 runtime-host 通过 Adapter / Bridge 把现有领域 facade 接入 `SystemService`。
- 让 SDK 通过 Facade 暴露 focused clients，供 Web/CLI/framework 消费。
- 让所有调用路径具有 trace、policy、structured log、event、snapshot。
- 让服务实现可替换，用户可以替换 LLM provider、memory backend、context engine、active recall policy、knowledge digest provider。
- 通过依赖门禁逐步移除 kernel/presentation 对 provider crate 的直接依赖债务。

## Non-Goals

- 不改变 Route C kernel primitive 定义。
- 不让 Context Service 直接拥有 Memory backend，也不让 Memory Service 直接拥有 LLM call。
- 不把 `/api/chat/v2`、framework runner、context report 的外部行为改成不兼容形态。
- 不在本轮引入新外部依赖。

## Design Patterns

- **Facade**: `LlmService`、`MemoryService`、`ContextService` 和 SDK focused clients 为上层提供稳定边界。
- **Adapter / Bridge**: runtime-host wrappers 把已有 domain facade/router/engine 适配成 `SystemService`，隔离领域 crate 和 runtime lifecycle。
- **Strategy**: model routing、memory routing、active recall policy、context engine selection、knowledge digest selection、fallback behavior 都必须可替换。
- **Command**: 所有服务调用先建模为 typed command，再转换为 `ServiceCommand` payload，便于验证、审计和 replay。
- **Decorator**: trace-required、policy、budget、privacy、token/cost、governance、context budget 等检查通过组合式 decorator 扩展。
- **Observer**: 服务生命周期和关键调用节点输出 structured logs/events。
- **Memento**: snapshot 输出健康、能力、拓扑、决策摘要，不默认倾倒 prompt 或 memory content。
- **Specification**: command constructors 和 runtime admission 校验 scope、trace、permission、budget、privacy tier、availability。

## Service Ownership

### LLM Service

`macaca-llm` 拥有 provider-neutral contract、command/result/event DTO 和领域 adapter。它不依赖 kernel、runtime-host、web、cli，不包含 provider URL/API key/model 名称硬编码。

`macaca-runtime-host` 拥有 LLM service provider wrapper：接收 `ServiceCommand`，校验 trace/policy，通过注入的 `LlmProvider` / `LlmRouter` strategy dispatch，并发出 runtime event/snapshot。

### Memory Service

`macaca-memory` 拥有 memory scope、visibility、provider capability、remember/recall/prefetch/forget/status/snapshot contract。它保留 application -> database、agent -> collection 的抽象拓扑，但不得把该拓扑硬编码到单一供应商实现。

Memory Service 明确支持 `AgentPrivate` 和 `SessionShared` 语义。Recall command 必须包含 application、session、agent scope，不允许默认 app-wide/global recall。

### Context Service

`macaca-context` 拥有 context assembly、active recall orchestration、knowledge digest、provider inventory、engine inventory、context report snapshot contract。它可以通过 memory service client bridge 主动召回长期记忆，但不得直接绑定具体 memory backend。

Context Service 负责组合、预算、provider chain、active recall 诊断、report assembly。LLM call 不属于 Context Service；未来需要 summarization 时也必须显式建模为 service call。

## Runtime and SDK Boundaries

`macaca-runtime-host` 只负责 service lifecycle、registration、decorator chain、dispatch、structured unavailable、snapshot/event wiring。它不得拥有领域决策，也不得依赖 Web/CLI。

`macaca-sdk` 提供 `SystemLlmClient`、`SystemMemoryClient`、`SystemContextClient`，以及 `SystemFacade` 上的薄方法。SDK 是 client/facade，不是 provider factory。未装配 runtime 的 shell 必须返回 structured unavailable，而不是 panic、阻塞或隐式构造 provider。

## Migration Strategy

1. 先建立三个领域 contract 和 provider-neutral DTO。
2. 再在 runtime-host 中注册 service provider wrappers。
3. 再添加 SDK focused clients 和 null-object unavailable clients。
4. 再迁移 framework model/context seam。
5. 再迁移 Web startup/state、`ContextReportingModel`、framework runner、active recall 路径。
6. 再迁移 CLI 生产路径的直接 LLM 构造。
7. 最后收缩 kernel compat，保留 deprecated wrappers，删除已消除的 allowlist 行。

## Trace, Policy, Snapshot, and Privacy

每个服务调用必须至少记录 command accepted、policy checked、dispatched、completed、failed、snapshot emitted 等关键节点。日志和事件必须包含 service id、operation、application/session/agent scope、trace id、status、duration 或错误摘要。

Snapshot/event 默认只输出 metadata、capability、topology label、health、decision summary、audit id、count，不输出完整 prompt、message body、memory content、embedding、secret、API key。

## Risks and Mitigations

- **Risk: 三个服务同时迁移导致 blast radius 较大。** Mitigation: contract、runtime wrapper、SDK client、framework、Web、CLI、kernel compat 分 slice 实施，每个 slice 单独验证。
- **Risk: Context 和 Memory 边界混淆。** Mitigation: Context 只编排 active recall，Memory 拥有存储和治理；两者通过 service client bridge 通信。
- **Risk: SDK 反向变成 provider factory。** Mitigation: SDK 只定义 command builders、clients、unavailable/null-object，不构造 provider/backend。
- **Risk: 删除 allowlist 过早导致依赖门禁失败。** Mitigation: 只有 cargo metadata 和 dependency gate 证明直接依赖消失后再删 allowlist 行。
- **Risk: snapshot 泄露敏感内容。** Mitigation: Memento 默认输出 metadata，内容 dump 需要未来显式 debug policy。

## Open Questions

- `macaca-sdk` 是否直接依赖领域 DTO，还是引入 proto-level DTO？
  - 推荐本轮允许 SDK 依赖 provider-neutral DTO，后续如跨进程/远程 service 需要再提升到 `macaca-proto`。
- `macaca-web -> macaca-context` 是否能在 S5 完全删除？
  - 仅当 Web 不再需要 context DTO 和 report view model 的领域类型时删除；否则先标记剩余依赖原因并保留迁移任务。
