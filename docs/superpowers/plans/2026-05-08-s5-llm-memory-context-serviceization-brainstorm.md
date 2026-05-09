# S5 LLM / Memory / Context 服务化 Brainstorm

## 背景

本次 S5 来自 `docs/superpowers/plans/2026-05-08-route-c-serviceize-and-modularize-non-kernel-capabilities.md`，目标是把 LLM provider、Memory backend、Context engine/composer 从 kernel、Web、CLI、agent/framework 构建路径中的直接持有关系收敛到可替换、可审计、可通过 `ServiceRuntime` 托管的 system service。

必须遵守：

- `macaca/docs/agent-os-microkernel-boundaries.md`
- `macaca/docs/route-c-serviceization-allowlist.md`
- `macaca/docs/route-c-architecture-governance.md`

当前诊断：

- `macaca-llm` 已有 `LlmProvider`、`LlmRouter`、model resolver、resilience/cost/rate-limit decorator，以及 descriptor-only `llm_service_descriptor()`。
- `macaca-memory` 已有 `MemoryFacade`、`MemoryFabricFacade`、AgentPrivate / SessionShared scope、provider registry/runtime、governance、active recall、vector topology，以及 descriptor-only `memory_service_descriptor()`。
- `macaca-context` 已有 `ContextFacade`、`ContextEngine`、`ContextProvider`、composer、governance pipeline、active recall provider、knowledge digest provider、external adapter engine。
- `macaca-runtime-host` 已有 host-owned `ServiceRuntime`、`ServiceProviderFactory`、trace-required decorator、policy decorator、service event/snapshot。
- `macaca-sdk` 已有 `SystemFacade`、`SystemServiceClient`、`ServiceCallCommand`，但还没有 LLM / Memory / Context focused client。
- `macaca-kernel` 仍通过 `KernelProviderCompat` 持有 `LegacyLlmProvider`，allowlist 中标记为 S5 债务。
- `macaca-web` 仍直接构造 `LlmRouter`、保存 `Arc<dyn LlmProvider>`、保存 `WebMemoryRuntime` 和 context registries，并在 `ContextReportingModel` / `FrameworkRunner` 中组合 LLM、Memory、Context。
- `macaca-cli` 仍构造 `StubLlmProvider` 并直接依赖 `macaca-llm`。
- `macaca-framework` 的 `LlmProviderAdapter`、`RoutedLlmAdapter`、`ReActAgent` 仍直接以 `ChatModel` / `LlmProvider` 路径调用模型，并直接调用 `ContextFacade::builtins(...)`。

## 设计模式候选

### Facade

建立 `LlmServiceFacade`、`MemoryServiceFacade`、`ContextServiceFacade` 的服务边界，对上只暴露 chat、model routing、remember、recall、forget、digest、assemble context、active recall、snapshot 等能力。

适用原因：

- Web/CLI/framework/agent builder 只依赖稳定服务 façade，不再拿 concrete provider/backend。
- 现有 `LlmRouter`、`MemoryFabricFacade`、`ContextFacade` 可以作为内置实现挂在 façade 后面。
- 与 S3 `SystemFacade`、S1 `ServiceRuntime` 对齐。

风险：

- 如果 façade 同时承载 LLM、Memory、Context 的全部细节，会形成新的巨型服务。
- 需要保持能力拆分：LLM Service、Memory Service、Context Service 可以协作，但不能互相硬耦合。

### Adapter / Bridge

把现有 `LlmProvider`、`MemoryFacade`、`ContextFacade` 包装为 `SystemService` provider；同时提供 SDK client adapter，把 typed command 转成 `ServiceCallCommand`。

适用原因：

- 可以复用现有实现，不重写 provider/backend/context composer。
- 可以渐进替换 Web/CLI/framework 调用路径。
- 支持未来 remote provider、enterprise memory、custom context system 替换。

风险：

- Adapter 如果放错层，比如 `macaca-llm -> macaca-kernel`，会产生反向依赖。
- 应优先让 service provider wrapper 放在能合法依赖两边的宿主层，或使用 provider-neutral crate-local contract + runtime-host factory。

### Strategy

LLM model/provider routing、memory routing、context engine selection、context provider assembly、active recall policy、digest selection都应是 Strategy。

适用原因：

- 用户必须能替换自己的上下文管理系统、记忆系统、模型路由策略。
- Memory 当前已有 `MemoryRouter`；Context 当前已有 engine/provider registry；LLM 当前已有 resolver chain。
- 不同 application/agent/session 可以使用不同服务实现或策略。

风险：

- 策略维度太多会过度设计。
- 首版应只稳定 service-level 策略 seam，不在所有调用点暴露细粒度配置。

### Command

所有服务调用用 typed command 建模：`LlmChatCommand`、`LlmModelSelectionCommand`、`MemoryRememberCommand`、`MemoryRecallCommand`、`MemoryForgetCommand`、`ContextAssembleCommand`、`ContextActiveRecallCommand`、`ContextSnapshotCommand`。

适用原因：

- `ServiceRuntime` 已以 `ServiceCommand` 调用服务。
- Command 可序列化、可 trace、可审计、可做 permission/policy。
- SDK/Web/CLI/Gateway 都能统一把输入转成命令。

风险：

- 命令字段若过早暴露 provider-specific 参数，会破坏可替换性。
- 必须保留 provider-neutral 字段：app/session/agent scope、trace、budget、policy hints、model hint、query、limit、context profile，而不是具体 provider 名称硬编码。

### Decorator

LLM token/cost/rate-limit、memory governance/tombstone/audit、context governance/budget/trust policy、trace-required、policy check 都应通过 Decorator 组合。

适用原因：

- S1 `ServiceRuntime` 已有 trace/policy decorator。
- LLM 已有 resilience/cost/rate-limit wrapper。
- Memory 已有 governance facade。
- Context 已有 governance pipeline。

风险：

- 多层 decorator 可能重复记录 trace 或重复执行 budget。
- 需要明确：runtime decorator 负责 service admission，domain decorator 负责 domain policy。

### Observer

LLM call、model route、memory recall/write、context assemble、active recall、knowledge digest、fallback/unavailable 都必须发出结构化 event 和 log。

适用原因：

- Route C 要求 service call、task lifecycle、capability call 都可 trace。
- Context/Memory 的主动召回需要可解释：召回了什么、为什么选中、为什么丢弃。
- 便于前端 trace viewer 和审计系统消费。

风险：

- 事件 payload 太大可能泄漏 prompt/memory 内容或拖慢 UI。
- 首版事件应记录 metadata、hash、counts、source ids、policy decisions，默认不记录完整敏感内容。

### Memento

为 LLM、Memory、Context 提供 snapshot：service health、model inventory、memory capability/status、context engine/provider inventory、last decision summaries。

适用原因：

- ServiceRuntime already supports deterministic snapshots。
- Web/CLI 需要展示状态，但不应直接读 provider/backend。
- Memory/Context 的治理、active recall、digest selection 都需要可复盘。

风险：

- snapshot 容易被误用成大数据导出。
- 必须限制 snapshot 内容为诊断摘要和可审计 metadata。

### Specification

命令校验、scope、trace、permission、budget、max context tokens、memory visibility、privacy tier、optional service availability 都应由 Specification 表达。

适用原因：

- 避免 Web/CLI/framework 各自判断。
- 避免 memory recall 默认 app-wide / global 扫描。
- 让 service unavailable / disabled_by_policy / missing_permission 有一致行为。

风险：

- 过严可能破坏 legacy tests 和 local stub flows。
- 首版应保留 deprecated compatibility wrapper，但新代码默认走 service command。

## 方案 A：一次性把 LLM / Memory / Context 全部迁入 `ServiceRuntime`

做法：

- 直接实现三个 `SystemService` provider。
- Web/CLI/framework/agent builder 立刻改为 service client。
- 删除或废弃所有 direct provider/backend 调用。

优点：

- 架构目标最清晰。
- allowlist 债务下降最快。

缺点：

- 涉及 `macaca-web::ContextReportingModel`、`FrameworkRunner`、`ReActAgent`、`KernelProviderCompat`、CLI startup、memory runtime、context provider assembly。
- 风险集中，容易破坏 `/api/chat/v2`、framework runner、active recall、task planner/review。
- 一次性修改面过大，不符合小步可审查原则。

结论：拒绝。

## 方案 B：先建三类服务 contract + SDK client + runtime-host provider wrapper，再逐步迁移消费者

做法：

- 在 `macaca-llm`、`macaca-memory`、`macaca-context` 内补 typed command/event/snapshot/service contract。
- 在合法宿主层提供 `SystemService` provider wrapper，把现有 `LlmRouter`、`MemoryFacade`、`ContextFacade` 适配到 `ServiceRuntime`。
- 在 `macaca-sdk` 增加 focused clients：`LlmServiceClient`、`MemoryServiceClient`、`ContextServiceClient`。
- Web/CLI/framework 先接入 client seam，保留 deprecated direct adapters 方便迁移检索。
- 通过回归后再删除 allowlist 行。

优点：

- 复用现有 LLM / Memory / Context 投资，不重写系统。
- 保持 provider/backend/context system 可插拔。
- 与 S1/S3/S4 架构一致。
- 可以按 slice 验证，不一次性破坏上层行为。

缺点：

- 第一阶段 allowlist 不会立刻清零。
- 会存在 service path 与 legacy path 并存一段时间，需要明确 deprecated 与禁止新调用规则。

结论：推荐。

## 方案 C：只做 SDK client，不做 runtime-host provider

做法：

- SDK 定义 LLM/Memory/Context client trait。
- Web/CLI/framework 注入 SDK client，但本地实现仍直接调用 provider/backend。

优点：

- 改动小。
- 上层调用形态先统一。

缺点：

- 没有真正 service runtime，trace/policy/decorator/snapshot 不完整。
- provider hub 只是从 Web 挪到 SDK local adapter，债务难以关闭。

结论：只能作为过渡子步骤，不能作为 S5 最终方案。

## 方案 D：先迁 LLM，Memory/Context 后置

做法：

- S5 本轮只实现 LLM Service。
- Memory/Context 保持旧路径。

优点：

- 范围最小。
- LLM service command 容易定义。

缺点：

- S5 标题和目标明确包含 LLM / Memory / Context。
- Context 组装依赖 Memory active recall，拆开会留下 Web/framework 继续作为组合 hub。
- 不能解决 agent builder 仍直接拿 backend/context system 的问题。

结论：不推荐作为完整 S5 plan；可以作为实施顺序中的第一 slice。

## 推荐方案

采用方案 B，并按“LLM contract → Memory contract → Context contract → runtime-host wrappers → SDK clients → upper consumer migration → allowlist removal”的顺序实施。

核心原则：

- Domain crates own contracts and provider-neutral descriptors.
- Runtime host owns `SystemService` lifecycle wrappers and ServiceRuntime registration.
- SDK owns upper-layer clients and command builders.
- Web/CLI/framework are adapters only.
- Kernel only keeps deprecated compatibility until all callers migrate.
- No service call without trace.
- No capability call without policy.
- No provider/backend hardcoding in Web/CLI/framework.

## 关键风险与缓解

- `macaca-framework` 依赖 `macaca-llm` 的 compat feature 会阻碍去 provider 化。
  - 缓解：先增加 `ChatModel`-level service adapter，保留 `LlmProviderAdapter` deprecated。
- `ContextReportingModel` 同时组合 context、memory、LLM，迁移风险高。
  - 缓解：先让它调用 `ContextServiceClient` 产出 assembled messages，再调用 `LlmServiceClient`，保留 current model wrapper 做兼容。
- Memory active recall 与 Context composer 耦合。
  - 缓解：Memory Service 只负责 recall/prefetch；Context Service 负责 provider chain/composer/budget。两者通过 service client bridge 协作，不直接依赖 concrete backend。
- Snapshot/trace 可能泄漏 prompt 或 memory。
  - 缓解：默认记录 ids/hash/counts/policy decision，敏感内容需要 explicit debug policy。
- Allowlist 删除过早会破坏构建。
  - 缓解：每删除一行前先跑 dependency gate 和 workspace check。

## OpenSpec 建议

建议后续创建一个主提案：

- `add-llm-memory-context-services-v1`

如果评审认为范围过大，可以拆成三个互相依赖的提案：

- `add-llm-service-v1`
- `add-memory-service-v1`
- `add-context-service-v1`

但从 S5 目标看，三者强相关，推荐一个提案内分多个 slice，避免 LLM 先迁后 Context/Memory 继续强耦合 Web。
