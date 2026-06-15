# Change: 协议微内核残留债务清零

## Why

`docs/2026-06-09-macaca-os-protocol-microkernel-reaudit-design.md` 证明：AgentScope 2.0 改写后，Macaca OS 的主执行链已经接近 `refactor-unified-call-path-microkernel` 目标，但当前实现仍没有 100% 达到“唯一协议调用路径 + 纯净微内核 + 非内核能力全部服务化/模块化 + 历史债务清零”。

这不是新增业务能力，而是一次终态收口：把已服务化主链路之外仍存在的过渡面、旧入口、公开兼容面、迁移词汇、直接构造点和静态 gate 漏洞一次性删除。完成后，不管 application 是 YAML、WASM、GenUI、headless、gateway、paid、optional Web3/EVM，所有会产生 OS capability side effect 的调用都只能进入唯一协议路径。

## Hard Evidence To Resolve

本 change 必须逐项解决以下硬证据，不能通过 allowlist、注释豁免或继续保留旧符号来绕过：

- `crates/runtime/macaca-framework/src/tool.rs` 超过 500 行，导致 OS-layer file-size gate 失败。
- `macaca-kernel` 内仍存在可替换能力实现：`WebhookAlertChannel` 直接构造 HTTP client，`AlertManager::new` 直接安装 webhook channel，`macaca-kernel` 仍依赖网络 client。
- `macaca-kernel` 仍导出 `AgentOrchestrator`，并在 kernel 内拥有 task delegation、agent matching、tool command parsing、result aggregation 等 agent/task 编排语义。
- `macaca-sdk::shell_provider_bridge` 仍 re-export driver、llm、memory、skill、task、tools、kernel、agent、context、framework、app、runtime-host 等 provider/runtime/application/framework 别名。
- `macaca-web` 仍持有 framework construction adapter、本地 loop/waker/channel/session 状态、workspace memory/tool/runtime construction anchors 和旧 chat route production export。
- `macaca-runtime-host` 仍有 deprecated public facades，包括 MCP manager、entitlement runtime facade 和带迁移语义的 optional bootstrap。
- application/package 层仍 re-export deprecated prompt/task planning helpers，workflow engine 仍保留 direct provider-looking fields。
- 代码、测试、文档和基线 spec 中仍存在允许旧路径的词汇和规则，包括 `legacy`、`compat`、`Route C migration`、`#[deprecated]`、`#[allow(deprecated)]` 以及旧 route/bridge 命名。
- 当前基线 specs 自身仍包含迁移期允许项，例如 context composer 旧入口、Web/CLI 兼容 helper、SDK “preserve compatibility”、ServiceRuntime “additive non-migrating” 等规则；这些规则与债务清零终态冲突。

## What Changes

- **BREAKING（内部 OS API）**：删除所有旧调用面、兼容别名、deprecated wrapper、旧 chat route、旧 application helper、SDK provider re-export bridge、kernel orchestrator public API、kernel webhook transport 和 runtime-host deprecated facades。
- **唯一协议路径终态**：所有 application shape 和 shell/SDK/plugin/WASM host import 调用统一走 `SystemFacade`/focused SDK client → protocol client → `ServiceRouter.route` → `ServiceRuntime.call` → `ServiceBus` → `SystemServiceBusHandler` → `ServiceCallExecutor` → `SystemService.call`。
- **微内核纯净化**：kernel 只保留 identity、registry、policy facade、trace/audit primitive、scheduler primitive、resource primitive、session/task state contracts、package guard、service call executor/bridge 等系统不变量；告警通知、agent/task 编排、工具解析、provider transport 全部移出 kernel。
- **服务化和模块化收口**：alert/notification、agent execution construction、MCP runtime, entitlement/store, application execution, context composition, optional providers 全部通过 service/runtime-host composition root 管理，缺席时返回结构化 unavailable/disabled/denied。
- **SDK 纯 facade**：SDK 只提供 provider-neutral typed commands/results/errors、focused clients、SystemFacade、Null Object/unavailable clients；不能构造 provider、不能 re-export provider/runtime-host/application/framework crate alias 给 shell。
- **Web/CLI 终态 thin shell**：shell 只 parse input、map DTO、call facade/client、render/SSE/GenUI/trace/replay/approval/diagnostics、subscribe events；不构造 framework agent、不持有 execution loop ownership、不拥有 provider anchors。
- **规范清理**：OpenSpec baseline 删除迁移期允许规则，新增“终态债务词汇清零”和“公开 API 无 deprecated”要求。
- **可执行 gate 加固**：新增或强化 zero-debt static gates，包括 no-debt-token、kernel-no-network-transport、kernel-no-orchestration-semantics、sdk-no-provider-reexport、runtime-host-no-deprecated-public-facade、shell-no-framework-construction、shell-no-local-execution-owner、application-no-old-helper、no-production-deprecated、file-size zero allowlist。

## Impact

- Affected specs:
  - `unified-execution-path`
  - `microkernel-boundary-purity`
  - `service-runtime`
  - `sdk-system-facade`
  - `web-cli-thin-shell-completion`
  - `web-cli-thin-shell-v0`
  - `serviceization-dependency-gate`
  - `serviceization-escape-hatches`
  - `execution-control-service`
  - `context-composer`
- Affected code areas:
  - `crates/kernel/macaca-kernel/src/alert.rs`
  - `crates/kernel/macaca-kernel/src/orchestrator.rs`
  - `crates/kernel/macaca-kernel/src/lib.rs`
  - `crates/kernel/macaca-kernel/Cargo.toml`
  - `crates/facade/macaca-sdk/src/shell_provider_bridge.rs`
  - `crates/facade/macaca-sdk/Cargo.toml`
  - `crates/shells/macaca-web/src/framework_agent_construction_shell_adapter.rs`
  - `crates/shells/macaca-web/src/state.rs`
  - `crates/shells/macaca-web/src/chat_orchestrator/route_legacy.rs`
  - `crates/runtime/macaca-runtime-host/src/mcp_runtime/*`
  - `crates/runtime/macaca-runtime-host/src/entitlement.rs`
  - `crates/runtime/macaca-runtime-host/src/optional_service_bootstrap.rs`
  - `crates/application/macaca-app/src/lib.rs`
  - `crates/application/macaca-app/src/workflow/engine.rs`
  - `crates/application/macaca-app/src/consumption.rs`
  - `crates/runtime/macaca-framework/src/tool.rs`
  - `crates/services/macaca-context/src/engine/*`
  - `crates/application/macaca-agent/src/*` where old capability APIs remain
  - `crates/tests/macaca-integration-tests/tests/*` boundary gates
  - governance and audit docs that still describe old migration terminology as active behavior
- External contracts expected to remain stable:
  - `/api/chat/v2` HTTP contract
  - SSE event transport contract
  - session recovery contract
  - YAML application manifest behavior
  - WASM Application ABI host import behavior
  - GenUI surface rendering contract
  - trace replay contract
  - optional-provider unavailable behavior
- Expected breaking impact:
  - Internal Rust APIs and tests that call removed symbols must migrate to focused clients, service commands, service providers, or test-only fixtures.
  - If an external consumer still imports removed deprecated Rust symbols, the code should fail at compile time with clear replacement notes in the final migration report instead of retaining wrappers.

## Acceptance Definition

The proposal is complete only when implementation tasks prove all of the following:

- `/api/chat/v2`, session recovery, YAML, WASM, GenUI, trace replay, and optional-provider unavailable end-to-end scenarios pass through the same canonical protocol path.
- No production OS-layer Rust code path invokes a provider, framework runner, runtime manager, task loop, tool parser, alert transport, MCP manager, entitlement/store facade, application helper, or optional provider directly outside the protocol/service path.
- `macaca-kernel` production dependency tree contains no network transport or concrete provider dependency and exports no agent/task orchestration module.
- `macaca-framework` AgentScope 2.0 source is split below 500 lines per file and contains no AgentScope 1.0 naming concession.
- `macaca-sdk` has no provider/runtime-host/application/framework crate alias bridge and no provider construction API.
- Web and CLI depend only on `macaca-sdk` and `macaca-proto` among workspace crates.
- Production and integration-test Rust code contain zero `#[deprecated]` and zero `#[allow(deprecated)]`.
- Production OS-layer code contains zero old-path debt tokens; any remaining historical mention must be confined to archived docs or this active OpenSpec change until archival.
- OpenSpec baseline no longer permits migration anchors or deprecated surfaces after this change is archived.
- `openspec validate --all --strict`, `cargo check --workspace`, and the terminal gate matrix all pass.

## Non-Goals

- Do not redesign the existing `ServiceRouter` / `ServiceRuntime` / `ServiceBus` / `SystemServiceBusHandler` / `ServiceCallExecutor` chain unless a task proves a concrete violation.
- Do not change product/business behavior for any specific application.
- Do not hardcode application names, workflow names, provider names, model names, driver names, gateway names, chain names, payment names, or business routes.
- Do not introduce a new framework abstraction above AgentScope 2.0 just to hide naming; use stable Macaca provider-neutral ports and adapters.
- Do not add a compatibility period. This change is the final cleanup proposal; removed APIs are removed, not wrapped.

## GitNexus Note

This refactor will touch high-fanout symbols and is expected to trigger HIGH/CRITICAL impact findings. Per user instruction, those findings are recorded as implementation memos and do not block execution. They still must be captured before code edits so reviewers can audit blast radius and test coverage.
