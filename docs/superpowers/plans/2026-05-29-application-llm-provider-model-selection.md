# Application LLM Provider And Model Selection

> Superpowers brainstorm + write-plan：让 Macaca application 通过后端拥有的
> `service.llm` 能力选择大模型提供商和模型，同时不把 provider/model 语义写进
> Web、frontend 或某个 application。

## 背景

当前 Macaca 已经具备一部分底层能力：

- `service.llm` 已有 `model.list`、`model.provider.capabilities.read`、
  `model.route.resolve`、`llm.chat`、`llm.snapshot` 等 typed command。
- `SystemLlmClient` 已经是 SDK/facade 层的服务访问入口。
- `LlmRouter` 已经支持 request、agent、app、system、fallback 维度的模型路由。
- `default.toml` 可以配置多个 provider 和 provider default model。

问题在于应用侧链路没有闭环：application 无法从后端服务发现真实可用的
provider/model，app-owned UI 无法渲染后端驱动的选择器，`/api/chat/v2` 里的
`model` 字段也没有成为可审计的路由决策。

## 约束

- 遵守 `macaca-os-architecture-governance.md`。
- 遵守 `macaca-os-microkernel-boundaries.md`。
- 遵守 `macaca-os-serviceization-allowlist.md`。
- LLM provider/model 路由属于 `service.llm`，不属于 kernel、Web、frontend 或
  application。
- Web/frontend 只能做 adapter：解析请求、调用 facade/service bridge、渲染已脱敏状态。
- OS 层不得根据 app id、provider 业务名、model 名、workflow 名或业务域写分支。
- 日志、trace、snapshot、UI response 不得暴露 API key、原始 prompt、原始 provider
  payload、provider secret 或无界输出。

## Superpowers Brainstorm

### 方案 A：前端读取静态配置生成模型下拉框

frontend 或 app-owned UI 复制 `default.toml` 中的 provider/model 列表，然后把用户选择的
字符串提交给执行接口。

风险：

- 违反 shell boundary，frontend 变成 provider 可用性和 model catalog 的语义所有者。
- 与运行时事实容易漂移；配置存在不代表 provider 已初始化或 API key 可用。
- 无法通过 `service.llm` 审计 route resolution 和 policy decision。

结论：拒绝。

### 方案 B：Web 增加薄 HTTP 路由封装 `SystemLlmClient`

Web 增加通用 route，例如 `/api/llm/models` 和 `/api/llm/routes/resolve`，内部调用
`SystemLlmClient::list_models`、`provider_capabilities` 和 `resolve_route`。

风险：

- 适合 host UI，但 app-owned UI 更应该通过声明的 `service.call` 能力访问服务。
- 如果只做 HTTP catalog，不保证 `/api/chat/v2` 真正使用用户选择的 route。
- 需要明确 app/session/agent/trace scope，否则 catalog read 缺少审计上下文。

结论：可作为 shell adapter，但不能单独作为完整方案。

### 方案 C：服务拥有 catalog + application bridge + execution override

application 声明 `service.llm` 和 bridge 能力。app-owned UI 通过 host bridge 调用
`service.llm` catalog 和 route resolution command。提交任务时，UI 把选中的
provider/model 作为 provider-neutral hint 传入 execution。`/api/chat/v2` 将该 hint
进入执行 envelope，framework/WASM session 记录请求 route 和 effective route。

风险：

- 当前 `LlmSystemServiceProvider` 更偏单 provider profile，需要扩展成可表示多个已配置
  provider、健康状态和 unavailable reason 的 catalog shape。
- framework execution 当前主要使用 agent/app/system default，需要补 request override。
- WASM fast path 不一定直接发起 LLM 调用，因此仍要把 route intent 作为 session metadata
  和 trace/audit evidence 记录下来。

结论：首选方案。它保持 service ownership，也能支撑 Codex-class application 的完整体验。

### 方案 D：先做 manifest scoped model policy

application manifest 声明允许选择的 provider/model，UI 只能在 policy 允许的范围内选择。

风险：

- 长期安全形态更强，但如果没有 `service.llm` catalog 和 route resolution，policy 无法
  独立解决当前问题。
- 会扩大权限和 entitlement 面，容易拖慢本次可验证闭环。

结论：作为第二阶段 hardening，不阻塞当前修复。

## 选择

采用方案 C；必要时用方案 B 作为薄 shell adapter；方案 D 留作后续安全增强。

设计模式：

- `Facade`：application 和 shell 通过 `SystemLlmClient` 或 generic app UI bridge 调用服务。
- `Command`：catalog、route resolution、chat dispatch 都使用 typed command/result。
- `Adapter/Bridge`：frontend app-owned UI bridge 将 iframe message 转成后端 service call。
- `Strategy`：`service.llm` 负责 provider/model 选择和 fallback 策略。
- `Decorator`：trace、policy、budget、audit、metering 包裹 route selection 和 dispatch。
- `Memento`：catalog snapshot、selected route、diagnostics 和 session metadata 可回放。
- `Specification`：route validation 用结构化诊断拒绝 unavailable/unsupported/undeclared 调用。

## Write-Plan

### 1. OpenSpec 与契约对齐

- 创建 `add-application-llm-model-selection` OpenSpec change。
- 在 `llm-service` 中增加 application-visible catalog、request-level route override、
  route audit metadata 需求。
- 在 `application-ui-runtime` 中增加 app-owned UI 通过 bridge 发现模型并提交 route hint 的需求。
- 运行 `openspec validate add-application-llm-model-selection --strict`。

### 2. LLM Service Catalog Profile

- 将 runtime-host LLM metadata 从单 `LlmProviderProfile` 扩展为可表达所有已配置 provider 的 catalog。
- catalog row 包含 provider id、health/availability、default model、known models、protocol metadata、
  sanitized unavailable reason。
- catalog 在 approved composition root 基于 `LlmConfig` 和 router registration outcome 构建。
- 禁止在 catalog、日志和 snapshot 中暴露 API key、base URL、raw prompt、raw provider payload。

### 3. Route Resolution 与 Chat Override

- 确保 `service.llm` 在 agent/app/system default 之前解析 request-level provider/model hint。
- 为 unavailable provider、unsupported model、missing default model、fallback route 生成结构化诊断。
- 将 `/api/chat/v2` 的 request model/provider override 传入 framework execution construction。
- 对 WASM host-dispatch session，即使 WASM export 不直接调用 LLM，也要持久化 requested route 和
  resolved route metadata。

### 4. Application-Owned UI Bridge

- app-owned UI 通过声明的 `service.call` bridge capability 调用 `service.llm` catalog 和 route command。
- `app.execution` 只负责启动执行和 stream SSE event，不拥有 provider routing 语义。
- bridge 日志记录 catalog read、route resolution、undeclared call denial、execution start，
  包含 trace id 和脱敏 metadata。

### 5. Codex WASM Workbench UI

- 将自由文本 model input 替换为由后端 `service.llm` 数据驱动的 provider/model selector。
- 展示 default route、unavailable provider、route diagnostics 和当前 selected route。
- 启动任务时提交 selected model/provider hint。
- 在 done event、route metadata 或 service audit 可用时展示 effective provider/model。

### 6. 测试与验证

- 增加 service-level tests：multi-provider catalog、unavailable providers、request-model precedence。
- 增加 Web/API tests：证明 `/api/chat/v2` 会把 request model 传入 route resolution/session metadata。
- 增加 frontend bridge checks：catalog/route message shape 与 structured unavailable response。
- 运行受影响 Rust crate 的 targeted tests。
- 运行 frontend lint。
- 用 Codex WASM Workbench 选择 provider/model，发送真实编程任务，验证 SSE/session/audit 中有 route evidence。

## 风险备忘

- 配置了 provider 不代表 provider 可用；catalog 必须报告 unavailable，而不是假装可用。
- 当前 provider 可能只知道 default model，不一定能枚举远端所有模型；初版不能夸大 catalog 完整性。
- 如果只接 UI selector，不接 framework runner，界面会显示已选择但执行仍走默认模型。
- 如果 route selection 没有 audit，刷新或回放后无法证明当时 intended/effective route。
- 如果 app UI 绕过声明的 bridge capability，会削弱 application permission model。

## Definition Of Done

- application 能通过后端服务能力发现 provider/model 选择。
- application 能提交带 provider/model hint 的任务，且 OS 层没有 application 专有硬编码。
- route 由 `service.llm` 解析、审计、记录日志，并出现在有界 session/trace metadata 中。
- unavailable provider/model 返回结构化诊断。
- Codex WASM Workbench 能渲染真实 model selector，并用所选模型发送真实 Macaca OS 执行任务。
