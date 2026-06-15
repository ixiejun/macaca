# Tasks: 协议微内核残留债务清零

> 执行约束：
> - 实现前必须逐项读取本 change 的 `proposal.md`、`design.md`、`tasks.md` 和相关 spec delta。
> - 改任意 Rust symbol 前运行 GitNexus impact，记录 blast radius；HIGH/CRITICAL 仅备忘不阻塞。
> - 每个替换都遵循“替身就位 -> caller 迁移 -> structured unavailable -> trace/audit/logging -> static gate -> 删除旧面”的顺序。
> - 不写 application 专有代码，不硬编码 application/workflow/provider/model/driver/gateway/chain/payment/business 名称。
> - 所有新增 Rust 代码必须有英文注释；关键执行节点必须有 provider-neutral `tracing`。
> - 不新增兼容层，不新增 deprecated wrapper，不新增 allowlist。

## 0. 基线确认与影响备忘

- [x] 0.1 确认工作树状态，记录当前 branch、HEAD、未跟踪文件，不修改用户已有变更。
- [x] 0.2 运行 `openspec list` 和 `openspec list --specs`，确认本 change 与现有 active changes 不冲突。
- [x] 0.3 运行 `rg -n "deprecated|allow\\(deprecated\\)|legacy|compat|Route C migration|shell_provider_bridge|route_legacy|AgentOrchestrator|WebhookAlertChannel|McpRuntimeManager|EntitlementRuntimeFacade" crates openspec/specs docs`，保存初始命中清单到本 change 的 implementation memo。
- [x] 0.4 运行 `cargo tree -e normal -p macaca-kernel --depth 1`，记录 kernel 当前 dependency baseline。
- [x] 0.5 运行 `cargo tree -e normal -p macaca-sdk --depth 1`，记录 SDK 当前 provider/runtime/application/framework dependency baseline。
- [x] 0.6 运行 `cargo tree -e normal -p macaca-web --depth 1` 和 `cargo tree -e normal -p macaca-cli --depth 1`，记录 shell dependency baseline。
- [x] 0.7 运行现有失败/风险 gate：`cargo test -p macaca-integration-tests --test os_layer_file_size_gate`，确认 `tool.rs` 超限仍可复现。
- [x] 0.8 运行现有主路径 gate，记录起点：`cargo test -p macaca-web unified_audit_replay_convergence_tests`、`cargo test -p macaca-web unified_delegation_path_tests`。
- [x] 0.9 为高扇出 symbols 建立 impact memo：`AlertManager`、`WebhookAlertChannel`、`AgentOrchestrator`、`shell_provider_bridge`、`FrameworkRunner` construction APIs、`McpRuntimeManager`、`EntitlementRuntimeFacade`、application old helpers、context old engine APIs。

## 1. 先恢复文件尺寸宪法 gate

- [x] 1.1 对 `crates/runtime/macaca-framework/src/tool.rs` 做职责切分设计，明确 public exports、types、registry、invocation、runtime/policy/trace hooks 的归属。
- [x] 1.2 新建 `crates/runtime/macaca-framework/src/tool/mod.rs`，只保留模块文档和 public re-export。
- [x] 1.3 迁出 provider-neutral value objects 到 `tool/types.rs`，保留 Apache 2.0 attribution（如源自 AgentScope 2.0 设计）和英文模块注释。
- [x] 1.4 迁出 registry/descriptor lookup 到 `tool/registry.rs`，确保 registry 不硬编码 application/provider 名。
- [x] 1.5 迁出 typed invocation command/result path 到 `tool/invocation.rs`，确保调用仍通过 service/protocol boundary。
- [x] 1.6 迁出 invocation/runtime glue、policy、trace hooks 到 `tool/invocation.rs`。
- [x] 1.7 删除旧单文件 `tool.rs` 或改为 `mod.rs` 入口，确保没有重复定义。
- [x] 1.8 跑 `cargo fmt`。
- [x] 1.9 跑 `cargo test -p macaca-framework --test agentscope2_framework_boundaries`。
- [x] 1.10 跑 `cargo test -p macaca-integration-tests --test os_layer_file_size_gate`，确认无 OS-layer 文件超过 500 行。

## 2. Kernel alert transport 服务化

- [x] 2.1 impact 分析 `AlertManager`、`WebhookAlertChannel`、`AlertChannel`、kernel alert public exports。
- [x] 2.2 将 alert event/severity/dedup/policy DTO 固化为 provider-neutral proto/kernel invariant；只保留不含 transport 的类型。
- [x] 2.3 在 runtime-host 建立 `service.alert` 或 `service.notification` provider factory，命令至少包括 `raise_alert`、`resolve_alert`、`health`、`snapshot`。
- [x] 2.4 实现 log/no-op/unavailable alert provider，确保缺席时返回结构化 unavailable 而不是 fake success。
- [x] 2.5 将 webhook sender 迁入 runtime-host alert provider，HTTP client 只能在 provider implementation 内构造。
- [x] 2.6 在 alert service call 上增加 trace-required、policy-before-side-effect、sanitized audit event。
- [x] 2.7 将所有 caller 从 kernel `AlertManager::new(webhook_url)` 或 direct channel construction 迁移到 alert service client。
- [x] 2.8 删除 kernel `WebhookAlertChannel`、kernel HTTP POST 逻辑和 direct webhook config handling。
- [x] 2.9 从 `macaca-kernel/Cargo.toml` 删除 network/http client 依赖。
- [x] 2.10 增强 `kernel_purity_gate` 或新增 `kernel_no_network_transport_gate`，禁止 kernel 依赖网络/http client 或实现 webhook/http transport。
- [x] 2.11 跑 `cargo tree -e normal -p macaca-kernel --depth 1`，确认 kernel 无网络 transport。
- [x] 2.12 跑 alert provider 单测、kernel 单测、`cargo test -p macaca-integration-tests --test kernel_purity_gate`。

## 3. Kernel AgentOrchestrator 删除

- [x] 3.1 impact 分析 `AgentOrchestrator`、builder、delegate methods、parse command helpers、result aggregation helpers。
- [x] 3.2 将仍有价值的 request/response DTO 下沉到 `macaca-proto` 或现有 agent/task service command DTO。
- [x] 3.3 将 task delegation 行为迁移到 `service.agent_execution` 或 `service.task` provider；agent matching 使用 service capability descriptors，不使用 prompt keyword。
- [x] 3.4 将 pause/resume、worker-loop、fork/join 交互统一交给 `service.execution_control`。
- [x] 3.5 把 `delegate_task`、`aggregate_results`、`report_to_coordinator` 这类 tool command parsing 从 kernel 删除；如仍需要，由 tool/task service 使用 typed command enum 表达。
- [x] 3.6 将只测试旧 orchestrator 的单测迁移为 service-level contract tests 或 test-only fixtures。
- [x] 3.7 删除 `crates/kernel/macaca-kernel/src/orchestrator.rs` production module。
- [x] 3.8 删除 `macaca-kernel/src/lib.rs` 中 orchestrator export。
- [x] 3.9 新增 `kernel_no_orchestration_semantics_gate`，禁止 kernel production source 出现 agent/task delegation parser、tool command parser、prompt keyword routing、worker-loop、result aggregation ownership。
- [x] 3.10 跑 kernel 单测、agent execution service tests、delegation MCP path tests、kernel purity gate。

## 4. SDK facade 纯化

- [x] 4.1 impact 分析 `macaca-sdk/src/shell_provider_bridge.rs` 和所有 imports。
- [x] 4.2 为每个 bridge alias 建立替换表：driver、llm、memory、skill、task、tools、kernel、agent、context、framework、app、runtime-host。
- [x] 4.3 对 driver/tool/skill/MCP aliases，迁移 caller 到 focused SDK clients 或 service snapshot commands。
- [x] 4.4 对 kernel/session/task aliases，迁移 caller 到 `SystemFacade` 或 proto DTO。
- [x] 4.5 对 context/framework/agent aliases，迁移 caller 到 provider-neutral client/port；framework construction 必须经 runtime-host service。
- [x] 4.6 对 app/runtime-host aliases，迁移 caller 到 Application ABI client 或 host bootstrap facade，shell 不再看到 runtime-host internals。
- [x] 4.7 删除 `shell_provider_bridge.rs`。
- [x] 4.8 从 `macaca-sdk/Cargo.toml` 删除 provider/runtime-host/application/framework 生产依赖；保留只含 proto/foundation/facade 必需依赖。
- [x] 4.8a 新建独立 host composition crate，迁移 Web 进程 bootstrap、provider/runtime/application/persistence anchors 和 optional package registration。
- [x] 4.8b 将 `macaca-web-server` binary 迁入 host composition crate；`macaca-web` 仅接受已组装的 provider-neutral state/facade。
- [x] 4.8c 将 SDK dependency purity gate 提升为 `--all-features`，禁止 feature 隐藏 runtime-host 或 optional-package workspace edge。
- [x] 4.9 新增 `sdk_no_provider_reexport_gate`，禁止 SDK production source `pub use macaca_* as ...` 暴露 lower-layer provider/runtime/app/framework alias。
- [x] 4.10 新增 SDK provider construction gate，禁止 SDK 构造 providers、runtimes、database backends、wallets、chain clients。
- [x] 4.11 跑 SDK 单测、web/cli dependency gate、`cargo tree -e normal -p macaca-sdk --depth 1`。

## 5. Web/CLI shell 终态 thin shell

- [x] 5.1 impact 分析 web `framework_agent_construction_shell_adapter.rs`、`state.rs`、`chat_orchestrator/route_legacy.rs`、loop/waker/channel fields。
- [x] 5.2 在 runtime-host/framework 建立 framework agent construction service/provider，输入为 typed context snapshot 和 execution policy，输出为 provider-neutral runtime agent handle/result。
- [x] 5.3 将 `FrameworkRunner::build_runtime_agent*` 调用从 web shell 迁到 runtime-host/framework provider。
- [x] 5.4 删除 web shell construction port 和 adapter production implementation。
- [x] 5.5 shell 只通过 SDK/focused client 请求 agent construction/execution。
- [x] 5.6 将 `ActiveSession` 中 pause flag、resume channel、SSE sender 的语义拆开：SSE sender 留 shell，pause/resume state/channel ownership 迁入 `service.execution_control`。
- [x] 5.7 将 `LoopState` 中 PlanLoop/WorkerLoop/Scheduler handles 和 wakers 迁入 execution-control/task service 或 runtime-host service owner。
- [x] 5.8 删除 workspace memory/tool/runtime construction anchors，替换为 service clients 或 runtime-host composition handles。
- [x] 5.9 删除旧 `/api/chat` production route implementation。
- [x] 5.10 删除 `chat_orchestrator/mod.rs` 中旧 route re-export。
- [x] 5.11 如消费代码仍调用旧 route symbol，直接迁移到 `/api/chat/v2` route/client；不要保留 wrapper。
- [x] 5.12 新增 `shell_no_framework_construction_gate`。
- [x] 5.13 新增 `shell_no_local_execution_owner_gate`。
- [x] 5.14 确认 web/cli workspace dependencies 仅为 `macaca-sdk` 和 `macaca-proto`。
- [x] 5.15 跑 `cargo test -p macaca-web unified_audit_replay_convergence_tests`。
- [x] 5.16 跑 `cargo test -p macaca-web unified_delegation_path_tests`。
- [x] 5.17 跑 `cargo test -p macaca-integration-tests --test shell_dependency_purity_gate`。

## 6. Runtime-host deprecated public facade 清理

- [x] 6.1 impact 分析 `McpRuntimeManager`、`mcp_runtime/facade.rs`、`registration.rs`、`manager_invoke.rs`。
- [x] 6.2 将 MCP manager 状态私有化为 implementation detail，不作为 deprecated public API 暴露。
- [x] 6.3 确保 MCP public entry 只有 `McpRuntimeFacade` 或 typed MCP system service commands。
- [x] 6.4 删除 MCP 相关 `#[deprecated]` 和 `#[allow(deprecated)]`。
- [x] 6.5 impact 分析 `EntitlementRuntimeFacade` 和 entitlement/store callers。
- [x] 6.6 将 entitlement/store callers 迁移到 `EntitlementSystemServiceProvider`、`SystemEntitlementClient` 或 provider-internal repository。
- [x] 6.7 删除 `EntitlementRuntimeFacade` public deprecated API。
- [x] 6.8 将 optional service bootstrap 重命名为 stable optional module/service bootstrap；删除带迁移路线语义的 module/function/file names。
- [x] 6.9 删除 runtime-host 中所有 deprecated public facade tests 或迁移为 service contract tests。
- [x] 6.10 新增 `runtime_host_no_deprecated_public_facade_gate`。
- [x] 6.11 跑 `cargo test -p macaca-runtime-host` 和 runtime-host static gates。

## 7. Application / framework / context / agent 旧 API 清理

- [x] 7.1 impact 分析 `macaca-app` deprecated re-exports：old prompt helper、entry agent helper、task planning contract helper。
- [x] 7.2 将 prompt/task planning helper caller 迁移到 manifest projection + Application ABI service commands。
- [x] 7.3 删除 `macaca-app/src/lib.rs` 中 old helper public re-exports。
- [x] 7.4 impact 分析 `WorkflowEngine` direct `kernel`/`llm` fields。
- [x] 7.5 将 workflow execution 改为 Application ABI adapter + service client，不保留 direct provider-looking fields。
- [x] 7.6 删除 `macaca-app/src/consumption.rs` 中旧 planning contract production API；测试改用 canonical command fixtures。
- [x] 7.7 impact 分析 `macaca-context/src/engine` old engine/default fallback APIs。
- [x] 7.8 将 context old entry points 替换为 canonical default composer/engine strategy，不使用 old-path naming。
- [x] 7.9 删除 context service production source 中 old engine module/function/type names；保留行为等价的 default strategy。
- [x] 7.10 impact 分析 `macaca-agent` old capability conversion helpers。
- [x] 7.11 将 caller 迁移到 canonical `AgentCapabilitySet` builder/value object APIs。
- [x] 7.12 删除 `BasicAgent` deprecated constructors 和 old capability flattening helpers；如测试需要，改为 canonical fixtures。
- [x] 7.13 新增 `application_no_old_helper_gate` 和 `context_no_old_entrypoint_gate`。
- [x] 7.14 跑 `cargo test -p macaca-app`、`cargo test -p macaca-context`、`cargo test -p macaca-agent`。

## 8. Repository-wide debt token and deprecated attribute cleanup

- [x] 8.1 扫描 production Rust：`rg -n "legacy|compat|Route C migration|#\\[deprecated|#\\[allow\\(deprecated\\)\\]" crates --glob '*.rs'`。
- [x] 8.2 对每个 production hit 分类：old-path debt、domain-neutral protocol term、test-only fixture、historical comment。
- [x] 8.3 对 old-path debt hit 执行迁移和删除；不能仅改注释绕过。
- [x] 8.4 对 test/integration hit，优先迁移到 canonical APIs；只有 test fixture 中必须表达 old external input 的情况可由 gate 明确排除。
- [x] 8.5 删除 production 和 integration-test Rust 中全部 `#[deprecated]`。
- [x] 8.6 删除 production 和 integration-test Rust 中全部 `#[allow(deprecated)]`。
- [x] 8.7 删除 `Route C migration` 活跃文案，改为 stable microkernel/serviceization terminology。
- [x] 8.8 对资源文件中的旧 schema key 做迁移；如 operator-facing config 必须变更，提供 one-shot config migration，不在 runtime 留双读 fallback。
- [x] 8.9 新增 `no_debt_token_gate`，默认扫描 production + integration-test Rust。
- [x] 8.10 跑 gate，确保 old-path debt token count 为 0。

## 9. OpenSpec baseline 收敛

- [x] 9.1 更新 `unified-execution-path` baseline：唯一协议路径不再允许 alternate route、old route、migration marker。
- [x] 9.2 更新 `microkernel-boundary-purity` baseline：kernel 禁止 network transport 和 agent/task orchestration semantics。
- [x] 9.3 更新 `service-runtime` baseline：删除 additive non-migrating 阶段语义，明确 terminal runtime ownership。
- [x] 9.4 更新 `sdk-system-facade` baseline：SDK 不再 preserve old compatibility wrappers，只 preserve stable response contracts。
- [x] 9.5 更新 `web-cli-thin-shell-v0` 和 `web-cli-thin-shell-completion` baseline：shell 不再允许 deprecated helper 或 compatibility anchors。
- [x] 9.6 更新 `serviceization-dependency-gate` baseline：allowlist 和 migration exception 规则变为 terminal zero-debt enforcement。
- [x] 9.7 更新 `serviceization-escape-hatches` baseline：freeze-only 规则移除，escape hatch 必须删除。
- [x] 9.8 更新 `context-composer` baseline：old context entry points removed，default composer replaces rollback path。
- [x] 9.9 更新治理文档：三部开发宪法不降级，只同步当前终态证据和 gate 名称。
- [x] 9.10 跑 `openspec validate --all --strict`。

## 10. End-to-end validation

- [x] 10.1 启动服务，跑 `/api/chat/v2` 新会话端到端，确认 trace/audit chain 只有 protocol/service path。
- [x] 10.2 跑 session recovery，确认历史/live trace 不重复，恢复不依赖 shell local loop ownership。
- [x] 10.3 跑 YAML application delegate，确认 Application ABI -> service path。
- [x] 10.4 跑 WASM host import，确认 guest call -> protocol/service path。
- [x] 10.5 跑 GenUI surface，确认 shell 只 render schema/intent，不拥有业务语义。
- [x] 10.6 跑 trace replay，确认每个 service call 有 trace id/session id/evidence id。
- [x] 10.7 跑 optional-provider unavailable 场景，确认 absent provider 返回 structured unavailable/disabled/denied。
- [x] 10.8 跑当前 workspace 等价内存/MCP/执行路径验证：`cargo test -p macaca-web unified --lib`。
- [x] 10.9 跑当前 workspace 等价 executor MCP path 验证：`cargo test -p macaca-integration-tests --test unified_audit_replay_terminal_gate -- --nocapture`。
- [x] 10.10 跑当前 workspace 等价 delegation MCP path 验证：`cargo test -p macaca-web unified_delegation_path_tests`。
- [x] 10.11 跑当前 workspace 等价 system-state/session path 验证：`cargo test -p macaca-web session --lib`。
- [x] 10.12 跑当前 workspace 等价 optional-provider unavailable path 验证：runtime-host Web3/EVM unavailable tests、finance package absent-provider test、package certification optional-module test。
- [x] 10.13 跑当前 workspace 等价 execution-control path 验证：`cargo test -p macaca-integration-tests --test p5_external_contract_gate -- --nocapture`。
- [x] 10.14 跑当前 workspace 等价 YAML application path 验证：`cargo test -p macaca-web unified --lib`。
- [x] 10.15 跑当前 workspace 等价 WASM host-import path 验证：`cargo test -p macaca-web unified --lib` 和 `/api/chat/v2` WASM host-dispatch live smoke。
- [x] 10.16 跑当前 workspace 等价 GenUI path 验证：`cargo test -p macaca-web genui_routes --lib` 和 live GenUI surface/event smoke。

## 11. Terminal gates

- [x] 11.1 `cargo check --workspace`。
- [x] 11.2 `cargo test --workspace` or documented targeted equivalent if workspace runtime is too large.
- [x] 11.3 `cargo test -p macaca-integration-tests --test os_layer_file_size_gate`。
- [x] 11.4 `cargo test -p macaca-integration-tests --test kernel_purity_gate`。
- [x] 11.5 `cargo test -p macaca-integration-tests --test shell_dependency_purity_gate`。
- [x] 11.6 `cargo test -p macaca-integration-tests --test protocol_service_dependency_boundaries`。
- [x] 11.7 `cargo test -p macaca-integration-tests --test serviceization_escape_hatches`。
- [x] 11.8 `cargo test -p macaca-framework --test agentscope2_framework_boundaries`。
- [x] 11.9 Run all new zero-debt gates: no-debt-token, kernel-no-network-transport, kernel-no-orchestration-semantics, sdk-no-provider-reexport, runtime-host-no-deprecated-public-facade, shell-no-framework-construction, shell-no-local-execution-owner, application-no-old-helper, context-no-old-entrypoint, no-production-deprecated.
- [x] 11.10 `rg -n "#\\[deprecated|#\\[allow\\(deprecated\\)\\]" crates --glob '*.rs'` returns zero production/integration-test debt hits.
- [x] 11.11 `rg -n "legacy|compat|Route C migration" crates --glob '*.rs'` returns zero old-path debt hits after gate exclusions.
- [x] 11.12 `openspec validate --all --strict`。

## 12. Final migration report

- [x] 12.1 Write an implementation report listing every deleted symbol/module/route/file and its canonical replacement.
- [x] 12.2 Include dependency before/after snapshots for kernel, SDK, web, and CLI.
- [x] 12.3 Include audit replay evidence for `/api/chat/v2`, YAML, WASM, GenUI, session recovery, trace replay, and optional-provider unavailable.
- [x] 12.4 Include GitNexus impact memo summary and note that HIGH/CRITICAL findings were recorded but not blocking.
- [x] 12.5 Include final `rg` and gate outputs proving debt token, deprecated attribute, allowlist, and file-size counts are zero.
- [x] 12.6 Request review before archiving the OpenSpec change.
