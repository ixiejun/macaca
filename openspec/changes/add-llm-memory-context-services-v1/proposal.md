# Change: Add LLM / Memory / Context Services v1

## Why

Route C 要求 kernel 只保留系统不变量，LLM provider、Memory backend、Context engine 都属于可替换能力，必须通过 System Service 暴露。当前 `macaca-kernel`、`macaca-web`、`macaca-cli`、`macaca-framework` 仍存在直接持有或构造 provider/backend/context engine 的兼容路径，这会让 presentation shell 和 kernel 继续承担 provider hub 职责。

S5 的目标是把 LLM、Memory、Context 三类能力收敛到独立、provider-neutral、可 trace、可审计、可替换的服务边界中，同时保持现有 `/api/chat/v2`、framework runner、active recall、memory tools、context report 与 Route C regression 行为不退化。

## What Changes

- 新增 `LlmService` 契约，覆盖 chat、model selection、model inventory、token/cost metadata、health snapshot。
- 新增 `MemoryService` 契约，覆盖 remember、recall/search、prefetch、forget/delete、status、governance snapshot。
- 新增 `ContextService` 契约，覆盖 model context assembly、active recall orchestration、knowledge digest composition、provider/engine inventory、context report snapshot。
- 在 `macaca-runtime-host` 中增加 host-owned service provider wrappers，把现有领域 facade/strategy 适配为 `SystemService` 生命周期与 dispatch。
- 在 `macaca-sdk` 中增加 focused service clients，让 Web/CLI/framework 通过 SystemFacade/service client 消费服务，而不是直接构造 provider/backend。
- 将 Web/CLI/framework/agent construction 迁移为 adapter 和 strategy seam，保留旧入口并标记 deprecated，便于后续检索和迁移。
- 在服务调用路径强制 trace、policy admission、structured log、snapshot/event 输出，敏感 prompt 与 memory 内容默认不得进入 snapshot/event。
- S5 完成并通过依赖门禁后，删除或收窄 S5 对应 allowlist 债务：`macaca-kernel -> macaca-llm`、`macaca-kernel -> macaca-memory`、`macaca-cli -> macaca-llm`、`macaca-web -> macaca-llm`、`macaca-web -> macaca-memory`。

## Impact

- Affected specs: `llm-service` (new), `memory-service` (new), `context-service` (new)
- Affected governance:
  - `macaca/docs/agent-os-microkernel-boundaries.md`
  - `macaca/docs/route-c-serviceization-allowlist.md`
  - `macaca/docs/route-c-architecture-governance.md`
  - `macaca/crates/macaca-integration-tests/tests/route_c_dependency_boundaries.rs`
- Affected code:
  - `macaca/crates/macaca-llm/src/service_contract.rs`
  - `macaca/crates/macaca-llm/src/service_adapter.rs`
  - `macaca/crates/macaca-memory/src/service_contract.rs`
  - `macaca/crates/macaca-memory/src/service_adapter.rs`
  - `macaca/crates/macaca-context/src/service_contract.rs`
  - `macaca/crates/macaca-context/src/service_adapter.rs`
  - `macaca/crates/macaca-runtime-host/src/llm_service_provider.rs`
  - `macaca/crates/macaca-runtime-host/src/memory_service_provider.rs`
  - `macaca/crates/macaca-runtime-host/src/context_service_provider.rs`
  - `macaca/crates/macaca-sdk/src/llm_client.rs`
  - `macaca/crates/macaca-sdk/src/memory_client.rs`
  - `macaca/crates/macaca-sdk/src/context_client.rs`
  - `macaca/crates/macaca-framework/src/adapter.rs`
  - `macaca/crates/macaca-framework/src/react_agent.rs`
  - `macaca/crates/macaca-web/src/state.rs`
  - `macaca/crates/macaca-web/src/framework_runner.rs`
  - `macaca/crates/macaca-web/src/context_reporting_model.rs`
  - `macaca/crates/macaca-cli/src/commands.rs`
  - `macaca/crates/macaca-kernel/src/provider_compat.rs`
  - `macaca/crates/macaca-kernel/src/kernel_builder.rs`

## Non-Goals

- 不在本轮迁移 Driver、Skill、MCP、Gateway、Application lifecycle、Store、Payment、Web3、EVM 能力。
- 不删除旧 provider/backend/context 兼容入口；旧入口必须 deprecated 且可搜索。
- 不把所有 LLM/Memory/Context 领域逻辑搬进 `macaca-runtime-host`；runtime-host 只拥有生命周期、dispatch、decorator、snapshot 适配。
- 不让 Web/CLI/framework 成为新的 provider hub。
- 不新增 app-specific、workflow-specific、provider-specific、model-specific 硬编码。
