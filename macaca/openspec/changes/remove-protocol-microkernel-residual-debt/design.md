# Design: 协议微内核残留债务清零

## Context

6 月 7 日的 `refactor-unified-call-path-microkernel` 已经建立正确方向：服务能力应统一进入 `ServiceRouter.route -> ServiceRuntime.call -> ServiceBus -> SystemServiceBusHandler -> ServiceCallExecutor -> SystemService.call`。6 月 9 日重审计显示，主路径虽然已经收敛，但仍存在足以破坏“终态”的残留面：kernel 中有可替换能力，SDK 仍有 provider alias bridge，shell 仍承担 construction/loop 语义，runtime-host 和 application/framework 仍有 deprecated facade，OpenSpec baseline 仍允许迁移期行为。

本设计把系统从“主路径优先”推进到“只有协议路径”。它不保留兼容路线，不设置过渡 allowlist，不通过 deprecated wrapper 降低短期迁移成本。

## Constitutional Constraints

- `macaca-os-architecture-governance.md`：Macaca 是微内核 + 服务运行时 + Application ABI + Plugin/Module ecosystem；shell 是 adapter，不是 semantic owner。
- `macaca-os-microkernel-boundaries.md`：kernel 只能拥有系统不变量；告警通知、agent/task 编排、provider transport、planner/worker-loop、MCP/tool/provider runtime 全部属于服务或模块。
- `macaca-os-serviceization-allowlist.md`：服务化是 ownership transfer，不是文件搬运；所有 service call 必须有 trace、policy、structured unavailable、health/snapshot、replacement mechanics。
- `docs/design_patterns.md`：优先用接口、组合、Facade、Command、Adapter/Bridge、Strategy、Decorator、Observer、Memento、Specification、Abstract Factory；不能为了抽象而抽象。

## Goals

- 单一协议调用路径覆盖所有 application shape 和所有 OS capability side effect。
- kernel production source 不再包含可替换 capability implementation。
- SDK 成为纯 provider-neutral facade。
- shell 不再构造 framework agent、不拥有 loop/waker/channel semantic state。
- runtime-host 只作为 composition root，不暴露 deprecated public facade。
- application/framework/context/agent 层删除旧 helper、旧 API、旧 engine entry point。
- 静态 gate 能证明旧路径、旧词、deprecated attribute、oversized file、direct provider call 全部为 0。
- 所有新增或迁移代码都具备英文注释、provider-neutral tracing、sanitized audit。

## Non-Goals

- 不改 `/api/chat/v2`、SSE、session recovery、YAML manifest、WASM ABI、GenUI surface 的外部契约。
- 不新增 application 专有能力。
- 不迁就尚未迁移的内部 caller；caller 必须改到新端口。
- 不新建第二套 service runtime。
- 不为了清词而重命名 ABI 兼容性这类领域概念；只有当词语代表旧路径债务时必须删除。实现 gate 需要明确 production debt token 与 domain-neutral compatibility metadata 的差异，但本次目标是 OS-layer production debt token 为 0。

## Final Architecture

```text
Application / Web / CLI / Gateway / Plugin / WASM host import
  -> SystemFacade or focused SDK client
  -> ProtocolClient
  -> ServiceRouter.route(ServiceRouteRequest)
  -> ServiceRuntime.call(ServiceCommand)
  -> ServiceRuntime decorators
       TraceRequired
       PolicyRequired
       Resource / Entitlement / Metering when applicable
       Audit
  -> ServiceBus
  -> SystemServiceBusHandler
  -> ServiceCallExecutor
  -> SystemService provider
  -> provider-specific implementation behind runtime-host composition root
```

Allowed exceptions:

- Presentation-only operations that do not cause OS capability side effects, such as rendering, SSE socket forwarding, local DTO mapping, and user-input parsing.
- Test-only fixtures compiled under tests and excluded by production gates.
- Foundation/proto serialization helpers that do not call provider behavior.

Everything else must enter the service path.

## Design Patterns

### Facade

`SystemFacade` and focused SDK clients remain the only upper-layer API boundary. Web, CLI, gateways, application adapters, plugins, and WASM host imports call facades/clients, not provider crates or runtime-host internals.

### Command

Every cross-boundary operation is represented as typed command/result/error DTOs. Removed direct helpers are replaced by explicit command structs with scope, trace metadata, policy-ready fields, and structured error variants.

### Adapter / Bridge

Provider-specific implementation stays behind adapters in runtime-host or service crates. Shell adapters convert HTTP/SSE/CLI shapes into commands and convert results into response DTOs. WASM host import adapters bridge guest calls into the same protocol path.

### Strategy

Provider choice, context composer choice, execution-control trigger/resume choice, optional service availability, and alert/notification sink choice are Strategy decisions registered in runtime-host, not kernel or shell branches.

### Decorator

Trace, policy, resource, entitlement, metering, audit, and redaction remain decorators around service calls. No direct caller may bypass this decorator chain.

### Observer

Trace events, audit events, service lifecycle events, task events, SSE streams, GenUI updates, and diagnostics are Observer outputs. Shells subscribe; they do not own state transitions.

### Memento

Session recovery, trace replay, task checkpoints, service snapshots, execution-control snapshots, and audit records are replayable mementos with bounded sanitized payloads.

### Specification

Boundary rules are executable specifications: dependency gates, static token gates, no-direct-provider gates, file-size gates, no-deprecated gates, shell semantic ownership gates, and OpenSpec validate gates.

### Abstract Factory

Provider factories and module bootstrapping exist only in approved composition roots, mainly runtime-host. SDK and shell never construct provider families.

## Ownership Decisions

### D1. Kernel Alert Transport Moves Out

Kernel may define alert identity, severity, dedup keys, policy decision types, and trace/audit evidence. Kernel must not own HTTP, webhook, network clients, retry transport, or provider configuration.

Implementation direction:

- Introduce `service.alert` or fold into an existing notification service family with typed `RaiseAlert`, `ResolveAlert`, `SnapshotAlerts`, and `Health` commands.
- Keep kernel-side alert emission as a typed event or port that records trace-required evidence and returns structured unavailable if no alert sink is registered.
- Runtime-host registers log/webhook/plugin/remote/mock/unavailable alert providers.
- Delete kernel HTTP transport and remove network dependencies from `macaca-kernel`.

### D2. Kernel Agent/Task Orchestrator Is Deleted

Agent/task matching, task delegation, command parsing, result aggregation, prompt keyword routing, and tool-name parsing are service semantics, not kernel invariants.

Implementation direction:

- Move any still-needed DTOs to `macaca-proto`.
- Move any still-needed behavior to `service.agent_execution`, `service.task`, or `service.execution_control`.
- Convert tests that only prove old orchestrator behavior into service-level contract tests or test-only fixtures.
- Delete production `orchestrator` module and public export.

### D3. SDK Provider Re-export Bridge Is Removed

The SDK remains the stable upper API, but it cannot hide provider/runtime/app/framework ownership violations by re-exporting lower crates.

Implementation direction:

- Inventory each consumer of `shell_provider_bridge`.
- For every alias, create or reuse a focused client or typed DTO boundary.
- Move provider construction/bootstrap into runtime-host composition root.
- Remove SDK dependency edges to provider/runtime-host/application/framework crates unless they are provider-neutral DTO crates approved by spec.
- Delete the bridge module and static gate against reintroduction.

### D4. Shell Construction And Local Execution Ownership Move Out

Web/CLI are thin shells. They may host sockets, parse requests, map DTOs, and render. They must not construct framework agents, own execution loops, manage provider anchors, or expose old routes.

Implementation direction:

- Move `FrameworkRunner` construction to runtime-host/framework construction service.
- Replace shell construction adapter with a runtime-host registered service provider.
- Move pause/resume/waker/channel ownership to execution-control/task services.
- Shell keeps SSE subscription and view model projection only.
- Delete old `/api/chat` route export and route implementation.

### D5. Runtime-host Public Surface Is Stabilized

Runtime-host is the composition root, but deprecated public facades create a second API surface.

Implementation direction:

- MCP manager internal state becomes private implementation detail behind `McpRuntimeFacade` and MCP system service commands.
- Entitlement/store callers use service provider + SDK client, not runtime facade.
- Rename and rewrite optional service bootstrap to stable optional module bootstrap semantics.
- Remove deprecated attributes and allow-deprecated uses from runtime-host production and tests.

### D6. Application / Context / Agent Old Entry Points Are Removed

Application framework can project manifests and ABI versions, but cannot keep old prompt/task planning helpers as public production APIs. Context composition can have a default strategy, but not a named old engine fallback API in production. Agent capability APIs must use canonical capability set/builder forms.

Implementation direction:

- Replace application prompt/task planning helper calls with manifest projection + service commands.
- Remove deprecated re-exports from `macaca-app`.
- Replace workflow direct `kernel`/`llm` fields with service clients or typed Application ABI command adapters.
- Replace context old engine constructors with canonical default composer/engine selection.
- Replace agent old capability conversion helpers with canonical `AgentCapabilitySet` constructors.

### D7. File-size Gate Is Restored First

The 504-line `tool.rs` failure is a current constitutional violation. It should be fixed early so subsequent refactors cannot hide inside an oversized file.

Implementation direction:

- Split by responsibility, not by formatting:
  - `tool/mod.rs`: public exports and module docs.
  - `tool/types.rs`: provider-neutral DTO/value objects.
  - `tool/registry.rs`: registry and descriptor lookup.
  - `tool/invocation.rs`: typed invocation command/result path.
  - `tool/runtime.rs`: runtime glue and policy/trace hooks.
- Add module docs and keep each file below 500 lines.

### D8. Process Composition Is Separate From The Web Shell

`macaca-web` cannot simultaneously be a presentation shell and the process
composition root. Hiding runtime-host, framework, application, persistence, or
optional-package dependencies behind SDK re-exports preserves the same
ownership violation under a different import path.

Implementation direction:

- Keep Axum routes, HTTP/SSE mapping, response projection, and presentation
  subscriptions in `macaca-web`.
- Move service/provider construction, application discovery, persistence
  construction, optional package registration, runtime handles, and process
  lifecycle into a dedicated host composition crate.
- The standalone `macaca-web-server` binary belongs to that host composition
  crate and injects a fully assembled provider-neutral Web state/facade into
  `macaca-web`.
- `macaca-web` depends only on `macaca-sdk` and `macaca-proto` among workspace
  crates. The host composition crate may depend on runtime-host and optional
  packages because it is an approved Abstract Factory/composition root, not a
  presentation shell.
- SDK features must not proxy optional packages or runtime-host types. Every SDK
  production feature remains protocol-client backed and the all-features
  dependency gate permits only `macaca-proto` as a workspace dependency.

## Migration Strategy

This is a deletion-first terminal cleanup, but deletion happens only after replacement path is proven. Each phase follows the same sequence:

1. Inventory current callers and record GitNexus impact memo.
2. Add or confirm provider-neutral command/client/service replacement.
3. Move callers to replacement.
4. Add structured unavailable behavior.
5. Add trace/audit/logging at the new boundary.
6. Add or strengthen static gate proving the old path cannot reappear.
7. Delete old API/module/route/attribute/term.
8. Run targeted tests and terminal gates.
9. Update OpenSpec/governance docs.

No phase may end by leaving a deprecated wrapper or compatibility alias.

## Logging And Audit Requirements

All new or migrated execution nodes must use provider-neutral fields:

- `service_id`
- `command`
- `operation`
- `trace_id`
- `request_id`
- `session_id`
- `task_id`
- `application_id` only when it is scope metadata, not routing logic
- `capability_id`
- `reason_code`
- `status`

Logs, traces, audit records, snapshots, and diagnostics must not include raw prompts, raw manifests, WASM bytes, package bytes, private keys, credentials, raw signatures, wallet secrets, provider payloads, raw tool payloads, or unbounded user input.

## Documentation And Comments

All new Rust modules and non-obvious functions must include English comments explaining:

- Which layer owns the behavior.
- Which pattern is being used and why.
- How trace/policy/audit are enforced.
- How unavailable behavior works.
- Why shell/kernel/SDK are not allowed to construct the provider directly.

Simple field assignments or obvious getter code should not receive noisy comments.

## Static Gate Design

### no-debt-token gate

Scans production Rust and integration-test Rust for old-path debt tokens. The final implementation should support a narrow exclusion list for this active OpenSpec change and archived historical documents, but production/test Rust must be zero.

### kernel-no-network-transport gate

Fails if `macaca-kernel` depends on network/http client crates or source references webhook/http transport implementation.

### kernel-no-orchestration-semantics gate

Fails if kernel production code contains agent/task orchestration semantics such as delegation command parsing, tool-name parser, prompt keyword routing, worker-loop, result aggregation, or agent matching.

### sdk-no-provider-reexport gate

Fails if SDK production source re-exports provider/runtime-host/application/framework crates or exposes provider constructors.

### runtime-host-no-deprecated-public-facade gate

Fails if runtime-host production or tests contain deprecated attributes, allow-deprecated attributes, or public facades kept only for old callers.

### shell-no-framework-construction gate

Fails if shell production code calls framework runner construction APIs or owns framework agent construction ports.

### shell-no-local-execution-owner gate

Fails if shell production state owns execution loop/waker/channel semantics beyond presentation event subscription.

### application-no-old-helper gate

Fails if application framework exposes old prompt/task planning helpers or workflow direct provider-looking fields.

### file-size terminal gate

Fails on any OS-layer Rust source file above 500 lines and requires zero file-size allowlist rows.

## Risk And Mitigation

- Risk: Removing old public Rust APIs breaks internal consumers.
  - Mitigation: inventory first, migrate callers to focused clients or service commands, then delete.
- Risk: Moving alert transport out of kernel changes observability timing.
  - Mitigation: preserve alert event DTO and trace evidence; provider-specific transport runs through service runtime with deterministic tests.
- Risk: Moving framework construction out of shell changes chat startup behavior.
  - Mitigation: keep `/api/chat/v2` DTO/SSE contract, add end-to-end tests for chat creation and session recovery before deletion.
- Risk: Removing context/application old helpers changes model prompt composition.
  - Mitigation: preserve canonical context plan output through default composer strategy and compare sanitized context reports in tests.
- Risk: Static token gate over-matches legitimate non-debt words.
  - Mitigation: classify findings by production code ownership and token purpose; keep final production/test code debt token count at zero, while archived historical docs may remain outside the runtime gate.

## Validation Matrix

- `openspec validate remove-protocol-microkernel-residual-debt --strict`
- `openspec validate --all --strict`
- `cargo check --workspace`
- `cargo test -p macaca-integration-tests --test os_layer_file_size_gate`
- `cargo test -p macaca-integration-tests --test kernel_purity_gate`
- `cargo test -p macaca-integration-tests --test shell_dependency_purity_gate`
- `cargo test -p macaca-integration-tests --test protocol_service_dependency_boundaries`
- `cargo test -p macaca-integration-tests --test serviceization_escape_hatches`
- `cargo test -p macaca-framework --test agentscope2_framework_boundaries`
- `cargo test -p macaca-web unified_audit_replay_convergence_tests`
- `cargo test -p macaca-web unified_delegation_path_tests`
- `cargo test -p macaca-web unified --lib`
- `cargo test -p macaca-web genui_routes --lib`
- `cargo test -p macaca-web session --lib`
- `cargo test -p macaca-web app_ui_session_projection --lib`
- `cargo test -p macaca-integration-tests --test unified_audit_replay_terminal_gate -- --nocapture`
- `cargo test -p macaca-integration-tests --test p5_external_contract_gate -- --nocapture`
- `cargo test -p macaca-runtime-host --test web3_service_provider unavailable -- --nocapture`
- `cargo test -p macaca-runtime-host --test evm_service_provider unavailable -- --nocapture`
- `cargo test -p macaca-integration-tests --test domain_pack_finance_package absent_finance_pack_leaves_service_unavailable -- --nocapture`
- `cargo test -p macaca-integration-tests --test package_certification package_certification_keeps_web3_and_evm_optional_modules_unavailable_safe -- --nocapture`
- End-to-end manual/service run for `/api/chat/v2`, session recovery, YAML, WASM, GenUI, trace replay, and optional-provider unavailable.

## Roll-forward Policy

This change does not use runtime compatibility wrappers as rollback. Rollback is done by reverting an implementation commit before merge, or by applying a new OpenSpec-approved forward fix. Once a deprecated surface is deleted in this change, it must not be reintroduced as a wrapper.
