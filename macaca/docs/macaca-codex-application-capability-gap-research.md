# Macaca OS 对标 Codex 类 Application 的能力缺口研究报告

日期：2026-05-27

## 1. 研究目标

本报告研究 `/Users/quantum/Code/dev/agent/codex` 作为本地编码 Agent 产品的参考实现，并将其需要的平台能力映射到 Macaca OS。目标不是复制 Codex 的产品逻辑，而是识别 Macaca OS 需要补齐哪些通用 OS 服务，才能让上层 application 通过 manifest、service contract、plugin、skill、tool 和 shell adapter 实现 Codex 级体验。

本报告严格遵循 Macaca 开发设计宪法：

- `macaca/docs/macaca-os-architecture-governance.md`
- `macaca/docs/macaca-os-microkernel-boundaries.md`
- `macaca/docs/macaca-os-serviceization-allowlist.md`
- `macaca/docs/design_patterns.md`

边界原则：

- Codex-like 产品必须是运行在 Macaca OS 之上的 application，而不是 OS 层硬编码行为。
- Macaca OS 可以提供通用服务，例如 session、tool、filesystem、process、sandbox、approval、hook、plugin、config、MCP、model、memory、diagnostics。
- Codex-like application 自己拥有产品 workflow、prompt、UI 文案、快捷键、IDE 集成体验、coding persona。
- OS 层禁止写 application 专有逻辑，禁止 hardcode application name、provider name、model name、gateway name、业务 workflow。

## 2. Codex 源码观察

`codex` 不是单纯 CLI，而是一套本地 agent 平台：

- `codex/README.md` 描述了 Codex CLI、desktop app、IDE integration、Codex Web 等入口。
- `codex/codex-rs/Cargo.toml` 包含大量能力 crate：app-server、protocol、core、tools、filesystem、exec、sandboxing、MCP、plugin、skills、hooks、state、memories、web search、model provider、remote environments、realtime、rollout、feedback、TUI。
- `codex/codex-rs/app-server/README.md` 描述了一个 bidirectional JSON-RPC app server，核心 primitive 是 Thread、Turn、Item，并提供 streaming notification、approval、filesystem RPC、command/process execution、MCP、skills、plugins、config、model catalog、realtime、remote control、feedback。
- `codex/codex-rs/core/src/tools/router.rs` 和 `codex/codex-rs/core/src/tools/registry.rs` 展示了 model-visible tool router、typed runtime contract、tool exposure control、hook、telemetry、lifecycle notification、dispatch trace。
- `codex/codex-rs/core/src/tasks/regular.rs` 展示了 turn lifecycle execution：任务抽象、cancellation token、event emission、input steering。

Macaca 当前已有的重要基础：

- `docs/macaca-industrial-tools-system-design.md` 已定义 Tool Capability Plane：planning、policy、invocation、runtime environment、gateway、artifact、telemetry、audit。
- `docs/superpowers/plans/2026-05-26-industrial-tools-system.md` 已将该设计拆成 service-owned proposal。
- `macaca/crates/runtime/macaca-runtime-host/src/tool_family_providers.rs` 已提供 industrial tool-family catalog 和 route metadata。
- `macaca/crates/runtime/macaca-runtime-host/src/tool_service_invocation.rs` 已将 `service.tool/tool.invoke` 路由到 owner service、runtime adapter 或 gateway adapter，并具备 admission、result normalization、artifact、audit。

核心判断：

Macaca 的架构方向是对的，但要匹配 Codex-class application，还缺一组更完整的 interactive-agent substrate：Thread/Turn/Item ledger、app-server protocol gateway、真实 filesystem/process/sandbox provider、approval/hook lifecycle、plugin marketplace、per-thread config/permission profiles、IDE-grade watcher、以及可靠的 LLM/tool continuation。

## 3. Superpowers Brainstorm

### 3.1 问题定义

如果 Macaca 要运行一个 Codex 级 coding agent application，OS 应该通用地提供什么？哪些必须留给 application 自己实现？

约束：

- Macaca 是通用 Agent OS，不是单一 coding app。
- Kernel 只拥有系统不变量。
- 可替换能力必须进入 service、plugin、optional module 或 application framework。
- Shell 只负责输入适配、状态渲染、approval 展示、diagnostics 展示、event subscription。
- 所有 privileged call 必须具备 trace、policy、resource/budget/entitlement gate、structured unavailable/denied/failure、sanitized log、audit。
- OS 代码不能 hardcode application、provider、model、gateway、product workflow、business logic。

### 3.2 方案 A：把 Codex 包成 External Application

做法：Macaca 通过 process adapter 或 gateway adapter 启动 Codex，把它暴露为一个外部应用。

优点：

- 最快可见 demo。
- Macaca 平台改动最少。
- 可作为迁移兼容桥。

风险：

- Macaca 不真正拥有 generic OS capabilities。
- tool、approval、filesystem、process、memory、audit 语义仍在 Macaca service runtime 之外。
- trace 和 audit 只能浅层记录，无法满足工业级可审计。

结论：可以作为临时 adapter，不足以作为平台战略。

### 3.3 方案 B：为 Coding App 写 Macaca 专用 OS Hook

做法：在 Macaca OS 层为 coding app 增加特殊路径。

优点：

- 对单一产品体验优化最快。
- 短期实现简单。

风险：

- 违反 `macaca-os-architecture-governance.md` 和 `macaca-os-serviceization-allowlist.md`。
- 把 application behavior 写进 generic service。
- 无法支撑 trading、news、document、enterprise automation 等完全不同 application。

结论：拒绝。

### 3.4 方案 C：建设通用 Interactive Agent Workbench 服务族

做法：把 Codex 级能力拆成通用 OS 服务：interaction ledger、app protocol gateway、filesystem、process、sandbox、permission、approval、hook、plugin marketplace、MCP manager、skills、config、model catalog、realtime、diagnostics。Codex-like app 只声明和组合这些能力。

优点：

- 符合 Macaca OS 宪法。
- 支撑 coding agent，也支撑其它复杂 application。
- provider-neutral、可扩展、可审计。
- 缺 provider 时返回 structured unavailable，而不是假成功。

风险：

- 需要多个 service contract 和 integration gate。
- 必须避免把 “coding workbench” 产品语义写入 OS 服务。

结论：推荐。

### 3.5 方案 D：增加 Codex-compatible App Server Protocol Shell

做法：Macaca 暴露类似 Codex app-server 的 JSON-RPC 协议，但实现为 shell/gateway adapter，底层调用 `SystemFacade` 和 focused clients。

优点：

- 可支持 IDE/desktop 类客户端。
- 支持 bidirectional streaming、backpressure、subscription。
- 如果放在 shell/gateway 层，可以保持 OS 服务边界清晰。

风险：

- 如果 Web 直接拥有协议语义，容易变成 semantic owner。
- 协议兼容压力可能诱导 Macaca 复制 Codex 产品概念，而不是保持 provider-neutral service contract。

结论：适合作为后续 shell 协议层，但必须建立在服务契约之上。

### 3.6 推荐方向

选择方案 C 作为架构目标，方案 A 作为临时 adapter，方案 D 作为后续 shell protocol。

核心原则：

```text
Codex-like application = application package + declared capabilities + app UI
Macaca OS = generic services that make such applications possible
```

## 4. 能力映射矩阵

| Codex 能力 | Macaca 目标 owner | 当前 Macaca 状态 | 缺口 |
| --- | --- | --- | --- |
| Thread/Turn/Item lifecycle | Session/interaction service | 有 session 和 EventLog，但缺 app-server 级 item ledger | 需要 durable interaction ledger |
| JSON-RPC app server | SDK 之上的 shell/gateway adapter | 有 REST/SSE Web API | 需要 bidirectional protocol gateway |
| Tool router and visible specs | `service.tool` | 已有 industrial planning/invocation | 需要 dynamic/deferred tool lifecycle 和 tool-search parity |
| Filesystem RPC and watchers | `service.file` 或 `service.tool` file provider | 有 file family descriptor | 需要真实 read/write/metadata/watch/patch provider |
| Command exec and PTY | `service.process` 或 shell provider | 有 shell family 和 runtime environment seam | 需要 streaming PTY、stdin、resize、terminate、background process registry |
| Sandboxing and permissions | policy/resource/sandbox services | 有 policy hook 和 environment seam | 需要 local/Docker/SSH/OS sandbox provider 和 permission profiles |
| Approvals and reviewer flow | approval service + shell UI | 有基础 approval metadata | 需要结构化 approval request、reviewer、persistence、audit |
| Hooks | hook lifecycle service | 尚未成为完整 OS service | 需要 pre/post tool hooks、managed-only policy、typed outcomes |
| MCP manager | `service.mcp` | MCP service 已存在 | 需要 status/resource/OAuth/reload/watch |
| Skills | `service.skill` | skill service 已存在 | 需要 discovery/watch/config/read/app-scoped enablement |
| Plugin marketplace | plugin/store services | 有 plugin control、store、entitlement 基础 | 需要 install/upgrade/uninstall/auth policy/bundled capabilities |
| Config and requirements | config/policy service | 配置分散 | 需要 layered config、admin requirements、hot reload、schema |
| Model catalog | `service.llm`/model service | 有 LLM routing | 需要 model list、provider capabilities、service tiers、reasoning options |
| Memory mode | `service.memory`/context | 有 memory/context | 需要 per-thread memory mode/reset/recall audit |
| Git/diff/review | git/review services | 有 task/review 基础 | 需要 code-review service、diff tracking、rollback markers、patch provenance |
| Realtime text/audio | realtime service | 不是核心能力 | 需要 optional realtime provider |
| Remote environments | environment service | 有 environment seam | 需要 remote exec-server registration、health、workspace roots |
| Feedback/doctor reports | diagnostics service | 有 logs/audit | 需要 trace bundle、privacy filter、operator feedback |
| Secret/keyring | secret service | 有 config/env bridge | 需要 dedicated secret store、scoped injection、redaction、rotation |

## 5. 需要补齐的通用服务

### 5.1 Interaction Ledger Service

建议 service id：`service.interaction`

职责：拥有 Thread、Turn、Item 的持久化和流式生命周期。

建议命令：

- `interaction.thread.start`
- `interaction.thread.resume`
- `interaction.thread.fork`
- `interaction.thread.archive`
- `interaction.thread.rollback`
- `interaction.thread.list`
- `interaction.turn.start`
- `interaction.turn.interrupt`
- `interaction.turn.steer`
- `interaction.item.append`
- `interaction.item.list`
- `interaction.item.watch`

设计模式：

- Command：所有跨边界操作使用 typed command/result。
- State：thread、turn、item、interruption、archive、rollback 生命周期显式建模。
- Memento：turn/item history 可重放。
- Observer：item/turn notification 可订阅。

Trace/Audit/Log：

- 每次 mutation 记录 `trace_id`、`application_id`、`session_id`、`thread_id`、`turn_id`、sanitized item kind、status。
- raw prompt 和 raw provider payload 不进入 generic snapshot。
- 大 payload 必须落 artifact ref。

边界：

- 该服务只拥有 generic interaction state。
- application 自己拥有 “coding task”、“review mode” 等产品语义。

### 5.2 App Protocol Gateway Service

建议 service id：`service.app_protocol`

职责：提供 JSON-RPC、stdio、websocket、unix socket 等 bidirectional protocol adapter，底层只调用 SDK/SystemFacade/focused clients。

建议命令：

- `app_protocol.connection.initialize`
- `app_protocol.subscription.create`
- `app_protocol.subscription.close`
- `app_protocol.notification.emit`
- `app_protocol.health.read`

设计模式：

- Adapter/Bridge：适配 JSON-RPC、REST、SSE、websocket、stdio、unix socket。
- Facade：统一调用 focused SDK clients。
- Observer：通知订阅。
- Decorator：backpressure、rate limit、trace、redaction。

Trace/Audit/Log：

- 记录 connection lifecycle、client metadata、capability negotiation、protocol version、queue saturation、rejected request。
- 不记录包含 prompt、secret、file content、provider payload 的 raw request body。

边界：

- 这是 shell/gateway service。
- 不能实现 planning、tool execution、approval decision、plugin lifecycle、application behavior。

### 5.3 Filesystem Service

建议 service id：`service.file`

职责：提供安全 workspace filesystem capability。

建议命令：

- `file.read`
- `file.write`
- `file.patch`
- `file.copy`
- `file.remove`
- `file.metadata`
- `file.directory.list`
- `file.watch`
- `file.unwatch`
- `file.diff`

设计模式：

- Adapter：local、remote、virtual、artifact-backed filesystem。
- Strategy：workspace root、symlink policy、write policy、binary policy。
- Specification：path admission、workspace boundary。
- Memento：pre-write snapshot、patch provenance。

Trace/Audit/Log：

- 记录 path hash 或 sanitized relative path、operation kind、byte count、policy decision ref、artifact ref。
- 默认不记录完整文件内容。

边界：

- OS 不知道文件属于 coding app、document app 还是 data app。

### 5.4 Process and Terminal Service

建议 service id：`service.process`

职责：提供 command execution、PTY、background job、stdin、resize、terminate、status。

建议命令：

- `process.exec`
- `process.spawn`
- `process.stdin.write`
- `process.pty.resize`
- `process.terminate`
- `process.output.subscribe`
- `process.status`
- `process.background.clean`

设计模式：

- State：process lifecycle。
- Adapter：local shell、PTY、container、SSH、managed exec-server。
- Decorator：policy、sandbox、timeout、metering、output truncation、audit。
- Observer：output delta notification。

Trace/Audit/Log：

- 记录 command hash、sanitized executable、cwd scope、sandbox id、exit status、duration、output byte count、policy decision。
- 大输出必须转 artifact ref。

边界：

- 用户主动 unsandboxed command 和 model 发起 sandboxed command 必须是不同 policy class。

### 5.5 Sandbox and Permission Profile Service

建议 service id：`service.sandbox`

职责：统一 local、Docker、SSH、WASM、browser、OS-specific sandbox runtime environment。

建议命令：

- `sandbox.profile.list`
- `sandbox.profile.resolve`
- `sandbox.environment.prepare`
- `sandbox.environment.health`
- `sandbox.environment.cleanup`
- `sandbox.policy.explain`

设计模式：

- Strategy：sandbox mode 和 permission profile。
- Abstract Factory：runtime-host composition root 构建 provider。
- Null Object：平台不支持时返回 unavailable provider。
- Specification：allowed paths、network domains、environment variables、write scopes。

Trace/Audit/Log：

- 记录 profile id、provider class、workspace roots、network policy class、resource lease refs、cleanup status。
- 不记录 raw env values 或 secrets。

边界：

- kernel 只知道 policy decision identity，不知道具体 sandbox implementation。

### 5.6 Approval and Guardian Service

建议 service id：`service.approval`

职责：拥有 approval request、reviewer policy、decision、pending queue、approval audit。

建议命令：

- `approval.request.create`
- `approval.request.list`
- `approval.request.resolve`
- `approval.policy.explain`
- `approval.decision.audit`

设计模式：

- State：pending、approved、denied、expired、cancelled。
- Decorator：privileged side effect 前置 gate。
- Observer：shell approval UI update。
- Memento：approval evidence 可重放。

Trace/Audit/Log：

- 记录 sanitized action summary、side-effect class、reviewer class、decision、reason code、trace refs。
- 不记录 raw command input、raw file content、secret。

边界：

- Web/frontend/CLI 只展示 approval prompt，不决定 policy。

### 5.7 Hook Lifecycle Service

建议 service id：`service.hook`

职责：提供 tool、turn、session、application event 的通用 pre/post hook。

建议命令：

- `hook.catalog.list`
- `hook.policy.resolve`
- `hook.pre_tool.run`
- `hook.post_tool.run`
- `hook.session.run`
- `hook.result.audit`

设计模式：

- Chain of Responsibility：有序 hook chain。
- Decorator：包裹 tool/turn execution。
- Specification：managed-only hook policy。
- Adapter：script、plugin、WASM、remote hook provider。

Trace/Audit/Log：

- 记录 hook id、source class、stage、outcome、duration、sanitized feedback。
- 如果 hook 修改 model-visible content，必须 trace-linked。

边界：

- hook 可以影响执行，但必须通过 policy 和 service runtime。

### 5.8 Plugin and Marketplace Service

建议 service id：`service.plugin_marketplace`

职责：安装、升级、移除、查看、启用、禁用、授权 plugin。plugin 可提供 skill、MCP server、app、hook、tool、service。

建议命令：

- `marketplace.add`
- `marketplace.remove`
- `marketplace.upgrade`
- `plugin.list`
- `plugin.read`
- `plugin.install`
- `plugin.uninstall`
- `plugin.enable`
- `plugin.disable`
- `plugin.auth.status`

设计模式：

- Abstract Factory：plugin capability registration。
- Adapter：local、remote、enterprise-managed marketplace。
- Specification：manifest admission、signature、entitlement、policy。
- State：install lifecycle、rollback。

Trace/Audit/Log：

- 记录 marketplace identity、plugin identity、version、capability summary、entitlement decision、signature/admission status、install result。
- 不记录 raw package bytes 或 credentials。

边界：

- Store/entitlement 拥有 distribution 和 authorization。
- plugin 不能绕过 service runtime、policy、resource、audit。

### 5.9 MCP Control Service Upgrade

目标 service id：`service.mcp`

职责：把 MCP 从 invocation 能力升级为完整 operator lifecycle：status、resources、OAuth、reload、per-thread exposure、diagnostics。

需要补齐或强化的命令：

- `mcp.server.status.list`
- `mcp.server.reload`
- `mcp.resource.read`
- `mcp.tool.call`
- `mcp.oauth.login`
- `mcp.oauth.status`
- `mcp.diagnostics.snapshot`

设计模式：

- Adapter：MCP transports。
- State：configured、starting、ready、failed、auth-required。
- Observer：server/tool/resource change。
- Null Object：absent/unhealthy MCP server。

Trace/Audit/Log：

- 记录 server id、lifecycle state、tool count、resource count、auth state、sanitized failure reason。

边界：

- MCP server 是 external provider；`service.mcp` 拥有 protocol runtime，不拥有业务行为。

### 5.10 Skills Control Service Upgrade

目标 service id：`service.skill`

职责：达到 Codex-class skill discovery、config、watch、read、enablement、provenance，同时保持 Macaca skill governance。

需要补齐或强化的命令：

- `skill.catalog.list`
- `skill.markdown.read`
- `skill.config.write`
- `skill.watch.start`
- `skill.watch.stop`
- `skill.changed`
- `skill.enablement.set`
- `skill.provenance.audit`

设计模式：

- Facade：封装 skill storage、governance、runtime。
- Observer：skill 文件变化。
- Specification：trust、lifecycle、app scope、source policy。

Trace/Audit/Log：

- 记录 skill id、source class、lifecycle、app scope、enablement change、governance refs。

边界：

- application-specific skills 可以存在，但 OS-level skill service 必须通用。

### 5.11 Config and Requirements Service

建议 service id：`service.config`

职责：集中管理 layered config、admin requirements、hot reload、schema、runtime feature flags、allowed policies、network constraints、managed hook policy。

建议命令：

- `config.read`
- `config.value.write`
- `config.batch.write`
- `config.schema.read`
- `config.reload`
- `config.requirements.read`
- `feature.list`
- `feature.enablement.set`
- `permission_profile.list`

设计模式：

- Builder：effective config resolution。
- Specification：requirement constraints。
- Memento：config snapshot 和 rollback。
- Observer：config changed notification。

Trace/Audit/Log：

- 记录 key path、writer scope、reload decision、requirement source class、decision refs。
- 不记录 raw secret value。

边界：

- runtime-host composition root 可以根据 config 构建 provider。
- SDK 和 shell 不能构建 provider。

### 5.12 Model Catalog and LLM Capability Service

目标 service id：`service.llm`

职责：把 LLM routing 扩展为 model catalog、provider capabilities、service tiers、reasoning effort options、token/rate budgets、continuation protocol correctness。

需要补齐或强化的命令：

- `model.list`
- `model.provider.capabilities.read`
- `model.route.resolve`
- `llm.chat`
- `llm.continuation.validate`
- `llm.budget.status`
- `llm.degradation.explain`

设计模式：

- Strategy：provider/model routing。
- Decorator：budget、retry、rate limit、redaction。
- Specification：provider protocol requirements。
- Null Object：unavailable provider。

Trace/Audit/Log：

- 记录 provider class、model id hash 或 sanitized id、route decision、token budget、retry decision、failure reason code。

边界：

- 之前真实 API 测试暴露了 DeepSeek thinking-mode continuation bug。这个问题应该在 `service.llm` 的 provider protocol validation 里解决，而不是在 shell 层 workaround。

### 5.13 Code Intelligence、Patch、Review、Git Service

建议 service id：

- `service.code_intelligence`
- `service.git`
- `service.review`

职责：提供通用代码分析、file diff、patch provenance、review execution、rollback marker、repository metadata。

建议命令：

- `git.status`
- `git.diff`
- `git.apply_patch`
- `git.rollback_marker.create`
- `code.search`
- `code.symbol_context`
- `review.start`
- `review.result.get`

设计模式：

- Adapter：Git、GitNexus、language server、static analyzer、external code intelligence provider。
- Memento：diff 和 rollback marker。
- Observer：review events。
- Strategy：analyzer selection。

Trace/Audit/Log：

- 记录 repository root、sanitized paths、diff hash、analyzer provider class、review status、artifact refs。

边界：

- coding-agent 产品语义留给 application。
- generic code intelligence 可以复用于 autonomous programming、migration、documentation、QA、compliance。

### 5.14 Realtime and Remote Environment Services

建议 service id：

- `service.realtime`
- `service.remote_environment`

职责：支持 realtime text/audio session 和 remote execution environment，并保持 optional module。

建议命令：

- `realtime.start`
- `realtime.append_audio`
- `realtime.append_text`
- `realtime.stop`
- `remote_environment.add`
- `remote_environment.health`
- `remote_environment.remove`

设计模式：

- Adapter：WebRTC、websocket、local、remote exec-server provider。
- State：session/environment lifecycle。
- Null Object：absent optional provider。

Trace/Audit/Log：

- 记录 modality、provider class、environment id、health status、sanitized connection state。

边界：

- 这些服务必须是 optional module。缺失时返回 unavailable，不能 crash、hang 或 fake success。

### 5.15 Feedback and Diagnostics Service

建议 service id：`service.diagnostics`

职责：生成隐私过滤后的 operator report、feedback bundle、doctor report、health snapshot。

建议命令：

- `diagnostics.snapshot`
- `diagnostics.feedback.upload`
- `diagnostics.trace.bundle`
- `diagnostics.health.summary`

设计模式：

- Facade：聚合 logs、traces、events、configs、provider health。
- Memento：diagnostic bundle 可重放。
- Decorator：redaction 和 payload bound。

Trace/Audit/Log：

- 记录 report id、included artifact classes、redaction profile、upload status。

边界：

- diagnostics 不能泄露 raw prompt、secret、file content、provider payload、unbounded logs。

## 6. Codex-like Application 对 Application Framework 的要求

Application framework 必须支持通过 manifest 声明能力，而不是让 OS 写 coding app 分支。

Codex-like app 应该声明：

- capability families：file、shell/process、code intelligence、git、tool、MCP、skill、memory、context、sandbox、approval、hook、plugin、diagnostics、model、optional realtime/remote environment。
- permission profiles：read-only、workspace-write、full-access、remote environment、network modes、approval policy。
- UI surfaces：terminal/TUI、Web、IDE connector、desktop、GenUI。
- event subscriptions：thread/turn/item、tool lifecycle、process output、filesystem watch、approval requests、diagnostics、provider health。
- package requirements：plugin dependencies、MCP dependencies、skill bundles、optional providers、entitlement requirements。

应用可以组合这些服务实现 Codex-like orchestration。OS 禁止出现类似 “如果 app 是 coding app，则暴露 shell” 的分支。正确做法是 application manifest 声明能力，然后 policy 决定是否允许。

## 7. Shell 和 UI 要求

为了支持 Codex-class experience，Macaca shell 需要更强的 adapter，但必须保持 thin shell：

- Web/frontend 渲染 Thread/Turn/Item stream、process output、file changes、tool traces、approvals、plugins、skills、MCP status、diagnostics。
- CLI 渲染同一套 event model 的 terminal 版本。
- IDE extension 通过 `service.app_protocol` 的 JSON-RPC/websocket 接入并订阅 typed notifications。
- Shell 只能调用 `SystemFacade` 或 focused clients。
- Shell 不拥有 approval decision、sandbox rule、plugin lifecycle、planning、tool invocation、filesystem policy。

## 8. Trace、Audit、Logging 最低要求

上述每个新服务都必须具备：

- stable descriptor、command surface、lifecycle、health、snapshot。
- 每个 command 必须带 trace context。
- side effect 之前必须通过 policy/resource/budget/entitlement gate。
- structured unavailable、unsupported、denied、failure states。
- 关键执行节点必须有 sanitized structured logs：
  - command accepted
  - policy evaluated
  - resource lease acquired/released
  - provider dispatch started
  - provider dispatch completed/failed
  - artifact stored
  - audit record appended
  - event emitted
- snapshot 和 event payload 必须 bounded。
- audit refs 必须允许 operator 重建 execution chain，但不能包含 raw secret 或 unbounded provider payload。

## 9. 建议实施阶段

### Phase 0：Contract Review and OpenSpec

先创建 OpenSpec proposals：

- `add-interaction-ledger-service`
- `add-app-protocol-gateway-service`
- `add-filesystem-process-sandbox-services`
- `add-approval-hook-config-requirements-services`
- `add-plugin-marketplace-and-mcp-operator-lifecycle`
- `add-codex-class-application-proof`

### Phase 1：Interaction Ledger and Protocol Gateway

实现 `service.interaction` 和基于 focused clients 的 JSON-RPC gateway。证明 thread/start、turn/start、item streaming、interrupt、resume、fork、archive、rollback。

### Phase 2：Filesystem、Process、Sandbox Providers

实现 provider-backed file/process/sandbox services，包括 local providers、Null Object unavailable providers、resource leases、output artifacts、policy gates。

### Phase 3：Approval、Hook、Config、Requirements

增加 structured approval queues、managed hooks、layered config、admin requirements、permission profile catalog、hot reload。

### Phase 4：Plugin、Skill、MCP、Marketplace Lifecycle

完成 plugin marketplace install/upgrade/uninstall、skill watch/config、MCP status/resources/OAuth/reload、bundled capability registration。

### Phase 5：Code Intelligence、Git、Review、Diagnostics

增加 reusable code intelligence、Git/diff/patch/review、diagnostic bundle、feedback surfaces。

### Phase 6：Optional Realtime and Remote Environments

作为 optional module 增加 realtime 和 remote environment services；缺失时必须 structured unavailable。

### Phase 7：Codex-class Application Proof

创建 application-neutral proof app，声明 coding-workbench 能力并执行真实任务：

- inspect repo
- 通过 `service.file` 修改文件
- 通过 `service.process` 执行测试
- 通过 `service.git` apply patch
- 通过 `service.review` 执行 review
- 通过 `service.tool` 调用 MCP/skill tools
- 通过 `service.app_protocol` stream 所有 Thread/Turn/Item、tool、process events

证明必须不向 OS 代码加入 application-specific branch。

## 10. Readiness Verdict

Macaca 已有正确方向和关键基础：service runtime、application framework、context/memory、skill/MCP、industrial tool planning/invocation、scheduler/heartbeat、EventLog/SSE、optional services。

但要匹配 Codex as an application platform，仍需补齐更强的 interactive-agent substrate：

1. Durable Thread/Turn/Item interaction ledger。
2. Bidirectional app protocol gateway，支持 backpressure 和 subscription。
3. Production filesystem、process、PTY、watcher、sandbox providers。
4. Structured approval 和 hook lifecycle services。
5. Layered config、requirements、permission profiles、hot reload。
6. Plugin marketplace 和 MCP/skill operator lifecycle。
7. Code intelligence、Git/diff/patch/review services。
8. 更强的 model catalog 和 provider protocol validation。
9. 严格 redaction 的 diagnostics 和 feedback bundles。
10. Optional realtime 和 remote execution environments。

结论：

Macaca 现在可以支撑部分 coding-agent workflow，但还不能诚实地宣称具备 Codex-class application parity。正确路线不是写一个 Codex 专用 OS 分支，而是补齐上述通用服务族。完成这些 serviceized capabilities 后，Codex-like app 就可以作为普通 Macaca application 干净地实现，上层其它 application 也能复用同一套 OS 能力。
