# Macaca OS Codex 级 Coding Application 开发者指南

日期：2026-05-29

## 1. 文档目标

本文面向在 Macaca OS 上开发 Codex 级 coding application 的开发者。读完后，开发者应当能够：

- 理解 Macaca OS 已提供哪些通用 app 开发服务和能力。
- 知道一个 Codex 级 coding application 应该通过 manifest 声明什么。
- 知道应用代码、OS 服务、SDK、Shell、插件和 UI 的边界。
- 知道如何验证自己的 application 没有绕过 Macaca OS 的 trace、policy、audit 和 service boundary。

本文基于以下材料整理：

- `docs/macaca-codex-application-capability-gap-research.md`
- `docs/superpowers/plans/2026-05-28-codex-class-application-support.md`
- `openspec/changes/complete-codex-class-application-support/`
- `macaca/docs/macaca-os-architecture-governance.md`
- `macaca/docs/macaca-os-microkernel-boundaries.md`
- `macaca/docs/macaca-os-serviceization-allowlist.md`

当前状态说明：`complete-codex-class-application-support` 的静态规范、目标 Rust gate、边界 gate、审计回放、`/api/chat/v2` regression、YAML/WASM/GenUI regression、industrial tools regression 已通过验证。live `/api/chat/v2` provider proof 已真实触达 session、tool exposure 和 generic shell tool result，但当前 DeepSeek provider continuation 配置仍返回 `reasoning_content in the thinking mode must be passed back to the API`，所以最终 live done proof 仍保持未关闭。

## 2. 核心边界

Macaca OS 支持 Codex 级应用的方式不是把 Codex 产品逻辑写进 OS，而是提供一组通用、可替换、可审计的 workbench service。Codex-like application 是普通 Macaca application：

```text
Application package
  -> manifest 声明能力、权限、工具族、UI surface
  -> SDK/SystemFacade/focused clients
  -> Macaca system services
  -> runtime-host providers/adapters
  -> service runtime trace/policy/audit decorators
  -> kernel identity/resource/trace/audit invariants
```

应用可以拥有：

- coding workflow、agent persona、prompt、UI 文案、快捷键、IDE/desktop 交互。
- app-owned GenUI surface 或 web bundle。
- 针对 coding 场景的任务编排策略。
- 对 OS 服务的组合方式。

OS 必须只拥有：

- provider-neutral service contract。
- policy、resource、budget、approval、entitlement、trace、audit。
- service descriptor、health、snapshot、structured unavailable。
- filesystem/process/sandbox/git/review/diagnostics 等通用能力。

禁止在 OS 层硬编码：

- application name。
- product workflow name。
- provider/model/driver/gateway/plugin name。
- 业务领域逻辑。
- Codex 专有 prompt、UI 文案或交互规则。

## 3. 开发者需要声明的 Workbench Manifest

Codex 级 coding application 应通过 manifest 的 `workbench` 块声明它需要的 generic capability surface。声明是数据，不是 OS 层分支。

示例：

```yaml
name: generic-coding-workbench
version: "1.0.0"
layer: L3Declarative

agents:
  - name: coordinator
    prompt_template: "app-owned prompt lives in the application package"

workbench:
  capabilities:
    - family: file
      optional: false
      reason: Read and patch workspace files through service.file
    - family: process
      optional: false
      reason: Run tests and commands through service.process
    - family: sandbox
      optional: false
      reason: Resolve permission profile before process side effects
    - family: git
      optional: false
      reason: Apply patches and rollback markers through service.git
    - family: review
      optional: false
      reason: Produce structured findings through service.review
    - family: diagnostics
      optional: false
      reason: Emit bounded trace bundles and health summaries

  permission_profiles:
    - workspace_write

  tool_families:
    - file
    - shell
    - sandbox
    - mcp
    - skill
    - code_intelligence
    - git
    - review
    - diagnostics

  service_dependencies:
    - service: service.interaction
      optional: false
      reason: Thread/Turn/Item state is service-owned
    - service: service.app_protocol
      optional: false
      reason: Bidirectional events stream through app protocol
    - service: service.file
      optional: false
      reason: Workspace filesystem operations are policy gated
    - service: service.process
      optional: false
      reason: Commands and PTY run through sandbox preflight
    - service: service.sandbox
      optional: false
      reason: Permission profile resolution is required before side effects
    - service: service.git
      optional: false
      reason: Patches require provenance and rollback markers
    - service: service.review
      optional: false
      reason: Review findings need structured evidence refs

  plugin_dependencies:
    - plugin_id: plugin.generic.example
      optional: true
      reason: Optional plugins can add tools or UI surfaces

  mcp_dependencies:
    - server_id: mcp.generic.example
      lifecycle_scope: session
      optional: true
      reason: Optional MCP servers can contribute declared tools

  skill_bundles:
    - bundle_id: skill.bundle.generic.example
      optional: true
      reason: Optional skills provide reusable procedures

  event_subscriptions:
    - topic: thread.item
      optional: false
      reason: Thread items stream through app protocol
    - topic: process.output
      optional: false
      reason: Command output must be bounded and observable
    - topic: approval.request
      optional: true
      reason: Shell renders approval state but does not own policy

  ui_surfaces:
    - surface_id: main
      schema: genui.workbench.v1
      mode: workspace
      required_bridge_capabilities:
        - service.call
        - app_protocol
      event_subscriptions:
        - thread.item
        - process.output
```

## 4. Codex 级应用能力地图

| 应用需要的能力 | Macaca owner | 开发者应该怎么用 |
| --- | --- | --- |
| Thread/Turn/Item lifecycle | `service.interaction` | 用它创建 thread、start/steer/interrupt turn、append/list/watch item。不要在 UI 或 app server 内自建最终事实源。 |
| Bidirectional protocol | `service.app_protocol` | 用它做 JSON-RPC/websocket/stdio/unix-socket gateway 和 event subscription。它只做 transport adapter。 |
| 文件读写和 watch | `service.file` | 用 `file.read/write/patch/list/metadata/watch`，让 path policy、artifact fallback 和 audit 由服务处理。 |
| 命令、PTY、后台进程 | `service.process` | 用 `process.exec/spawn/stdin.write/pty.resize/terminate/status/output`，不要从 Shell 直接拥有进程语义。 |
| 沙箱和权限 | `service.sandbox` | 在 process/file/tool side effect 前解析 permission profile、workspace root、network/write policy。 |
| Approval | `service.approval` | 文件写入、命令执行、Git patch、plugin install、MCP auth 等 privileged action 先过 approval decorator。 |
| Hook | `service.hook` | 用 pre/post tool hook、session/turn lifecycle hook；hook 可以 block/rewrite bounded input，但必须审计。 |
| Config 和 requirements | `service.config` | 读取 default/user/project/app/session/managed 配置，声明 permission profile、feature flag、managed hook policy。 |
| LLM/model catalog | `service.llm` | 通过 model catalog、provider capabilities、budget/degradation diagnostics 做模型选择。 |
| Plugin marketplace | `service.plugin_marketplace` | 插件安装、升级、卸载、auth、bundled capability 注册都经 store/entitlement/policy。 |
| MCP lifecycle | `service.mcp` | MCP status、reload、OAuth、resource read、tool call audit 都归服务；model-invoked MCP tool 继续走 `service.tool`。 |
| Skill lifecycle | `service.skill` | skill catalog、markdown read、config、watch、enablement、provenance 由服务拥有。 |
| Code intelligence | `service.code_intelligence` | code search、symbol context、file reference、analyzer diagnostics 使用 provider adapter 和 bounded snippets。 |
| Git/patch | `service.git` | status、diff、apply patch、rollback marker、path policy、pre/post hash 统一经服务。 |
| Review | `service.review` | structured findings、severity、location、rationale、evidence refs、artifact-backed payload。 |
| Diagnostics/feedback | `service.diagnostics` | 生成 privacy-filtered trace bundle、health summary、feedback receipt。 |
| Realtime | optional `service.realtime` | 缺 provider 时必须 structured unavailable；不能影响 base OS 启动。 |
| Remote environment | optional `service.remote_environment` | remote exec-server registration、workspace roots、health、cleanup 都是可选服务行为。 |
| Tool planning/invocation | `service.tool` | 所有 tool descriptor 必须能路由到 owning service，或者返回 truthful unavailable。descriptor-only 不算完成。 |

## 5. 推荐开发流程

### 5.1 设计应用 package

先定义 application 自己拥有的内容：

- 应用目标：coding assistant、review assistant、migration agent、QA repair agent 等。
- agent 划分：coordinator、planner、coder、reviewer、diagnostics 等。
- prompt/persona：只放在 application package，不进入 OS 服务。
- UI：选择 host GenUI、app-owned web bundle、headless、CLI/IDE gateway。
- 需要的 workbench capabilities、permission profiles、tool families。

### 5.2 声明 manifest

在 `workbench` 中声明：

- `capabilities`
- `permission_profiles`
- `tool_families`
- `service_dependencies`
- `optional_provider_requirements`
- `plugin_dependencies`
- `mcp_dependencies`
- `skill_bundles`
- `event_subscriptions`
- `ui_surfaces`

声明原则：

- 必需能力设 `optional: false`。
- 插件、MCP、realtime、remote environment 这类部署相关能力优先设为 optional，应用自行处理 unavailable。
- reason 要说明 generic capability 需求，不写产品专有逻辑。

### 5.3 通过 SDK/focused clients 调服务

应用或 shell adapter 不应该构造 provider。正确路径是：

```text
Application/Shell
  -> SystemFacade or focused client
  -> service runtime
  -> owning service provider
```

典型调用序列：

1. `service.interaction` 创建 thread 和 turn。
2. `service.app_protocol` 初始化连接并订阅事件。
3. `service.file` 读取仓库文件。
4. `service.code_intelligence` 查找符号和引用。
5. `service.approval` 创建/等待需要的 approval。
6. `service.hook` 跑 pre-tool hook。
7. `service.git` 应用 patch，并写 rollback marker。
8. `service.sandbox` 准备执行环境。
9. `service.process` 运行测试或构建。
10. `service.review` 生成 review findings。
11. `service.diagnostics` 生成 trace bundle 和 health summary。
12. `service.interaction` append final item 并 complete turn。

### 5.4 处理 structured unavailable

Codex 级应用必须把 provider absence 当作正常状态处理。例如：

- remote environment 不可用：降级到 local sandbox 或提示用户配置 remote provider。
- realtime 不可用：继续使用 text/SSE。
- MCP OAuth 需要登录：通过 approval/auth UI 展示，不在 OS 层伪造成功。
- plugin marketplace 被 admin 禁用：展示 disabled reason，不能绕过 store/entitlement。

### 5.5 UI 和 Shell 只做 adapter

Web、CLI、frontend、IDE adapter 可以：

- parse input。
- 调 focused clients。
- render Thread/Turn/Item、process output、file change、approval、hook、plugin/MCP/skill status、review findings、diagnostics。
- 订阅 typed events。

不能：

- 决定 approval policy。
- 自己实现 file/process/sandbox semantics。
- 自己实现 tool routing、plugin lifecycle、MCP lifecycle、skill governance。
- 根据 application name 或 provider name 走特殊分支。

## 6. 最小 Codex 级 Coding Workflow

一个最小但完整的 Codex 级 coding workflow 应覆盖以下链路：

```text
start thread
  -> start turn
  -> inspect files
  -> search symbols
  -> request approval if side effect is privileged
  -> run pre-tool hook
  -> apply patch
  -> prepare sandbox
  -> run tests
  -> invoke MCP/skill/tool if needed
  -> run review
  -> emit diagnostics
  -> stream all events through app protocol
  -> complete turn
  -> replay audit evidence
```

完成标准：

- 每一步都有 `trace_id`。
- privileged side effect 发生前有 policy/resource/approval gate。
- 输出过大或敏感时进入 artifact ref。
- event payload bounded and sanitized。
- audit refs 可 replay。
- provider 不可用时返回 structured unavailable。
- 没有 OS-layer application-specific branch。

## 7. 代码和测试入口

已实现能力的主要入口：

- Workbench DTO 和 service constants：
  - `macaca/crates/foundation/macaca-proto/src/workbench.rs`
  - `macaca/crates/foundation/macaca-proto/src/workbench/`
- Workbench manifest declaration：
  - `macaca/crates/foundation/macaca-proto/src/application_workbench_manifest.rs`
  - `macaca/crates/application/macaca-app/src/model.rs`
  - `macaca/crates/application/macaca-app/src/manifest_v1/yaml_adapter.rs`
  - `macaca/crates/application/macaca-app/src/service_projection.rs`
- SDK focused clients：
  - `macaca/crates/facade/macaca-sdk/src/workbench_client.rs`
- Shell adapters：
  - `macaca/crates/shells/macaca-web/src/workbench_routes.rs`
  - `macaca/crates/shells/macaca-cli/src/workbench_operations.rs`
- Application-neutral proof：
  - `macaca/crates/tests/macaca-integration-tests/tests/codex_class_application_neutral_proof.rs`
  - `macaca/crates/tests/macaca-integration-tests/tests/support/codex_class_application_neutral_proof/`
- Boundary gates：
  - `macaca/crates/tests/macaca-integration-tests/tests/codex_class_scope_control.rs`
  - `macaca/crates/tests/macaca-integration-tests/tests/serviceization_escape_hatches.rs`
  - `macaca/crates/tests/macaca-integration-tests/tests/tool_service_boundaries.rs`
- Full validation facade：
  - `scripts/validate-codex-class-application-support.sh`

## 8. 开发者验证清单

开发一个 Codex 级 coding application 后，至少跑这些验证：

```bash
openspec validate complete-codex-class-application-support --strict
cargo test --manifest-path macaca/Cargo.toml -p macaca-app --test workbench_manifest
cargo test --manifest-path macaca/Cargo.toml -p macaca-integration-tests --test codex_class_application_neutral_proof
cargo test --manifest-path macaca/Cargo.toml -p macaca-integration-tests --test codex_class_scope_control
cargo test --manifest-path macaca/Cargo.toml -p macaca-integration-tests --test serviceization_escape_hatches
cargo test --manifest-path macaca/Cargo.toml -p macaca-integration-tests --test tool_service_boundaries
scripts/validate-codex-class-application-support.sh
```

如果需要运行 deployment-specific live proof：

```bash
RUN_LIVE_API_PROOF=1 \
MACACA_LIVE_API_PROOF_CMD='<your application-neutral /api/chat/v2 proof command>' \
scripts/validate-codex-class-application-support.sh
```

live proof 的命令必须由部署方提供，原因是它依赖当前 provider、model、app id、auth、network、runtime topology。验证脚本不会硬编码 provider、model 或 application name。

## 9. 常见错误

不要把这些逻辑写进 OS 层：

- `if app_name == "coding-workbench"`。
- `if provider == "deepseek"` 或 `if model == "..."`
- coding app 专属 prompt 或 shortcut。
- 针对某个 MCP server、skill bundle、plugin id 的 OS 分支。
- 在 Web/CLI/frontend 中直接决定 approval、path policy、sandbox profile。
- descriptor 注册成功就宣称 provider-backed execution 完成。

正确做法：

- provider-specific 兼容差异放在 provider adapter 或 service strategy。
- application-specific workflow 放在 application package。
- shell-specific presentation 放在 shell adapter。
- OS service 只看 provider-neutral command、policy、trace、audit、artifact refs。

## 10. 当前已知缺口

当前静态和集成验证已经证明 Macaca 可以提供 Codex 级 application 所需的 generic service substrate，但仍有一个 completion gate 保持打开：

- Live `/api/chat/v2` provider proof 尚未完成 terminal done。2026-05-29 的 live smoke 已创建 session、暴露工具并执行 generic shell tool result，但 DeepSeek thinking-mode continuation 返回 provider protocol error：`reasoning_content in the thinking mode must be passed back to the API`。

因此，面向开发者的结论是：

- 可以基于当前 workbench service surface 开始开发 Codex 级 coding application。
- 应用必须按本文声明 capabilities，并通过 SDK/focused clients 调用 OS 服务。
- 如果要宣称生产级 provider-backed Codex-class parity，仍需要在目标部署环境补齐 live `/api/chat/v2` terminal done proof。
