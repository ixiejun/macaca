# Macaca 工业级 WASM Application Runtime Brainstorm

## 1. 背景

当前 Macaca 已经完成 Application Platform 的基础阶段，并通过 `add-wasm-component-application-abi-skeleton` 提案建立了 WASM Component ABI/WIT 的骨架。但该阶段明确是 metadata-only / unavailable-safe：

- `macaca/application-wit/macaca-application.wit` 只定义了 ABI 方向，没有真实运行 guest WASM。
- `macaca/crates/foundation/macaca-proto/src/application_abi.rs` 和 `application_wasm_abi.rs` 提供 provider-neutral DTO 与 WIT 辅助类型。
- `macaca/crates/application/macaca-app/src/wasm/mod.rs` 仍以 descriptor/admission 元数据为主。
- `macaca/crates/runtime/macaca-runtime-host/src/application_hosts/mod.rs` 当前 WASM host 是 structured unavailable host。
- 现有设计避免提前引入 Wasmtime 或其他 runtime 依赖，这是正确的骨架阶段边界。

现在目标变化为：WASM application 不能停留在 v0/v1 骨架，必须成为 Macaca Application Platform 的工业级可用运行时之一。它需要支持第三方应用开发、安装、认证、执行、沙箱、能力授权、服务调用、观测、升级、回滚和长期兼容，而不是只证明 ABI 方向。

## 2. 必须遵守的边界

本能力必须遵守：

- `macaca/docs/agent-os-microkernel-boundaries.md`
- `macaca/docs/route-c-serviceization-allowlist.md`
- `macaca/docs/route-c-architecture-governance.md`
- `macaca/docs/design_patterns.md`
- `docs/openharmony-microkernel-architecture-for-macaca-agent-os.md`
- `docs/superpowers/plans/2026-05-12-application-platform-option-e-plan.md`

硬边界：

- Kernel 不拥有 WASM runtime、engine、guest lifecycle 或 host imports 实现，只保留 identity、policy facade、trace/audit、resource/session/task/package guard 等系统不变量。
- Application Framework 拥有 application、ability、manifest、package、admission、compatibility、lifecycle 语义，但不直接拥有 Wasmtime 等执行引擎。
- Runtime Host / service module 拥有可替换的 WASM execution provider 和 host service bridge。
- SDK 只依赖 provider-neutral contracts，不构造 Kernel、AppRuntime、ServiceRuntime、Web state 或具体 provider。
- Web/CLI/Gateway 只能展示状态、触发命令和读取 sanitized metadata，不得定义 WASM app workflow、permission、runtime kind、service dependency 的核心语义。
- YAML app 继续一等支持，但不能成为 WASM capability 的特权入口。
- 禁止硬编码 app name、workflow name、provider name、driver name、gateway name 或任何业务名称。
- 所有执行、host import、capability access、snapshot、upgrade、certification 必须带 trace 或 fail-closed/unavailable。
- 日志和 snapshot 必须脱敏，禁止记录 raw WASM bytes、raw manifest、raw guest payload、prompt body、secret、env、API key、private key、unbounded user input。

## 3. 设计模式候选

### 3.1 Bridge

WASM guest 与 Macaca system services 必须通过 Bridge 隔离。guest 只能看到 WIT/ABI 和 host imports，host imports 再桥接到 ApplicationHostCommand / ServiceRuntime / capability portal。

价值：

- 避免 guest 依赖 Rust 内部 crate。
- 支持多语言 guest SDK。
- 支持不同 WASM engine 或 out-of-process host 替换。

### 3.2 Abstract Factory

按 runtime kind、engine policy、deployment profile 创建 `WasmApplicationRuntimeProvider`、`WasmInstanceHost`、`WasmStoreContext`、`HostImportBridge`。

价值：

- 默认可以使用 Wasmtime provider。
- 企业 hardened deployment 可以替换为 out-of-process provider。
- 测试环境可以使用 deterministic mock provider。

### 3.3 Strategy

编译缓存、实例池、fuel/epoch 中断、WASI policy、storage backend、network policy、host import dispatch 都应是 strategy。

价值：

- 不把 Wasmtime 细节写死在 Application Framework。
- 可按应用、租户、部署环境替换安全和性能策略。

### 3.4 Command

每个 host import 都必须变成可审计 Command，例如 service call、storage read/write、GenUI render、memory recall、task spawn、plugin hook。

价值：

- 所有调用都有 trace id、capability id、permission context 和 sanitized result。
- 可复用现有 serviceization 的命令/结果模式。

### 3.5 Specification

使用 Specification 校验 manifest、WIT ABI、engine compatibility、permission、service dependency、resource limits、WASI access、certification rules。

价值：

- admission 可组合。
- certification 与 runtime admission 使用同一套规则。
- fail-closed 行为清晰。

### 3.6 State

WASM application/ability lifecycle 使用 State 模式表达 `Loaded -> Compiled -> Instantiated -> Started -> Paused -> Draining -> Stopped -> Failed`。

价值：

- 避免生命周期分支散落在 host 代码里。
- 支持 pause/resume、checkpoint、upgrade、rollback。

### 3.7 Memento

对 compiled artifact metadata、guest state checkpoint、resource usage、certification report、compatibility report 做 memento。

价值：

- 支持恢复、迁移、升级回滚。
- 支持审计和问题复现，但不泄露 raw guest payload。

### 3.8 Null Object

缺少 runtime provider、engine 不可用、capability 未授权时继续返回 structured unavailable / policy denied host，而不是 panic 或走 fallback 特权路径。

价值：

- 保持 optional module 可插拔。
- 支持 Web/CLI 展示可诊断原因。

## 4. 可选方案

### Option A：直接在 `macaca-runtime-host` 内嵌 Wasmtime

做法：

- 给 `macaca-runtime-host` 直接增加 Wasmtime dependency。
- 在现有 `WasmApplicationHostFactory` 中把 unavailable host 替换为 Wasmtime host。
- WIT bindings、host imports、resource limits 都在 runtime-host 内实现。

优点：

- 路径最短。
- 能较快跑通真实 WASM component。
- 对当前骨架改动集中。

风险：

- runtime-host 会被具体 engine 污染，后续替换 WasmEdge、Wasmer、out-of-process executor 成本高。
- 容易把安全策略、实例生命周期、host imports、service bridge 混成巨型模块。
- 不利于多部署 profile，例如本地开发和企业隔离执行。

结论：

- 只适合作为短期 spike，不适合作为工业级主线。

### Option B：定义可插拔 WASM Runtime Provider，默认 provider 使用 Wasmtime

做法：

- 在 provider-neutral contract 中定义 `WasmApplicationRuntimeProvider`、`WasmEngineCapability`、`WasmExecutionProfile`、`WasmHostImportBridge`。
- Application Framework 只做 manifest/admission/compatibility，不依赖 engine。
- Runtime Host 注册默认 Wasmtime provider，同时允许替换 provider。
- host imports 统一转成 ApplicationHostCommand / ServiceRuntime call。

优点：

- 符合 Macaca 微内核 + 服务化架构。
- 能支持默认工业实现，也允许用户替换 runtime。
- 对 SDK、多语言 guest 和 certification 都更稳定。

风险：

- 第一阶段接口设计必须谨慎，否则后续 provider 难兼容。
- 默认 Wasmtime provider 仍需大量安全和资源限制细节。

结论：

- 是工业化的必要基础，但隔离级别仍主要依赖 in-process sandbox。

### Option C：WASM Runtime 作为独立 out-of-process service

做法：

- Runtime Host 不直接执行 WASM，只通过 IPC/RPC 调用独立 WASM execution service。
- execution service 负责 engine、实例、WASI、resource limit、crash isolation。
- Host imports 通过 capability-scoped bridge 回调 Macaca service runtime。

优点：

- 最强进程隔离和崩溃隔离。
- 更适合不可信第三方 application 和多租户部署。
- 可独立升级 WASM runtime 和 engine。

风险：

- IPC、生命周期、trace 关联、性能和部署复杂度显著增加。
- host import bridge 需要处理 reentrancy、timeout、backpressure 和 cancel。
- 本地开发体验可能变重。

结论：

- 适合 hardened profile，不适合作为唯一默认路径。

### Option D：分层双执行面架构

做法：

- 控制面放在 Application Framework / Application Service：manifest、ability、package validation、ABI negotiation、policy admission、certification、metadata projection。
- 执行面通过 `WasmApplicationRuntimeProvider` 插拔：默认 in-process Wasmtime provider，hardened profile 使用 out-of-process provider。
- SDK 与 guest bindings 只依赖 WIT/ABI，不知道具体执行面。
- 当前 unavailable host 保留为 Null Object fallback。

优点：

- 同时满足工业可用、可插拔、默认可运行和 hardened 部署。
- 不把 Kernel 或 Application Framework 变成 WASM runtime。
- 允许分阶段交付：先真实 default provider，再 hardened provider，再生态工具链。

风险：

- 需要拆成多个 OpenSpec 提案实施。
- 必须严格控制 file/module 边界，否则容易退化为 runtime-host 巨型实现。

结论：

- 推荐方案。

## 5. 推荐方案：Option D 分层工业级 WASM Application Runtime

推荐将 WASM 支持设计为 `Wasm Application Runtime Fabric`：

```text
Developer SDK / Guest SDK
  ↓
WIT / ABI / Manifest v1
  ↓
Application Framework Control Plane
  - manifest admission
  - ability compatibility
  - WIT ABI negotiation
  - permission/service/resource specifications
  - certification report
  ↓
Application Service / Runtime Host
  - runtime provider registry
  - lifecycle command routing
  - host import bridge
  - sanitized diagnostics
  ↓
WASM Execution Provider
  - default in-process provider
  - hardened out-of-process provider
  - mock/test provider
  ↓
Macaca Service Runtime
  - LLM / Memory / Context / Driver / Skill / MCP
  - Plugin / GenUI / Store / Payment / Web3 / Storage
```

关键原则：

- WASM 是 Application Runtime 的一种，不是 Kernel 能力。
- WASM guest 只能通过 host imports 访问系统服务。
- host imports 必须 capability-scoped、trace-required、policy-checked。
- raw WASM artifact 只能进入 package/artifact pipeline，不进入日志或 Web metadata。
- 默认 runtime provider 要可用，但 provider 接口不能绑定到 Wasmtime。
- hardened provider 可以后续实现 out-of-process，但控制面必须从第一天支持该部署模型。

## 6. 工业级能力清单

### 6.1 ABI / WIT 稳定性

需要支持：

- WIT package versioning。
- ABI semantic version negotiation。
- guest required imports / exported abilities 校验。
- runtime capability feature detection。
- compatibility matrix。
- deprecated ABI 标记与迁移路径。

风险：

- ABI 过早冻结会限制后续能力。
- ABI 频繁变更会破坏第三方生态。

缓解：

- 使用 capability flags + version negotiation。
- certification 记录 guest 使用的 ABI surface。

### 6.2 Package / Artifact Pipeline

需要支持：

- WASM component artifact descriptor。
- artifact hash、signature metadata、origin、build profile。
- deterministic validation。
- compiled artifact cache key。
- package admission report。

风险：

- 直接信任本地路径或 raw bytes 会造成供应链风险。

缓解：

- artifact 只按 hash/id 引用。
- 日志只记录 artifact id/hash prefix，不记录 raw bytes。

### 6.3 Runtime Provider 抽象

需要支持：

- `WasmApplicationRuntimeProvider`。
- `WasmEngineCapabilities`。
- `WasmCompilationService`。
- `WasmInstanceFactory`。
- `WasmExecutionSession`。
- `WasmHostImportBridge`。
- `WasmRuntimeDiagnostics`。

风险：

- provider 接口太贴近 Wasmtime 会降低可替换性。

缓解：

- trait 使用 Macaca ABI DTO 和 provider-neutral command/result。
- Wasmtime 细节放在默认 provider module 内。

### 6.4 Sandbox / Resource Governance

需要支持：

- memory/table limits。
- fuel 或 epoch interruption。
- wall-clock timeout。
- host import timeout。
- max payload size。
- max concurrent instances。
- instance pool quota。
- denied-by-default WASI。
- no raw env by default。
- no raw filesystem by default。
- no raw network by default。

风险：

- 只依赖 WASM sandbox 不足以防资源耗尽。

缓解：

- admission + runtime 双层限制。
- 所有限制必须可 trace。

### 6.5 WASI / Host Resource Policy

需要支持：

- capability-scoped preopen。
- app-scoped virtual storage。
- session-scoped temp storage。
- sanitized clock/random/config host APIs。
- deny raw process/env/socket。

风险：

- 一旦把宿主 FS/env 暴露给 guest，后续很难收回。

缓解：

- 默认无 WASI 或最小 WASI。
- 所有资源必须通过 policy facade 和 service portal。

### 6.6 Host Imports / Service Portal

需要支持：

- service call import。
- storage import。
- GenUI render import。
- memory recall import。
- context snapshot import。
- plugin hook import。
- task/session event import。
- payment/web3 optional import。

风险：

- host import 如果直接调用 provider，会绕过 serviceization 边界。

缓解：

- host import 统一变成 Command。
- Command 经过 policy、trace、service registry、capability check。

### 6.7 Lifecycle

需要支持：

- validate。
- compile。
- instantiate。
- init。
- start。
- handle event。
- render。
- pause/resume。
- drain/shutdown。
- checkpoint/restore。
- upgrade/rollback。

风险：

- lifecycle 不完整会导致长期运行 agent app 无法 7x24 稳定运行。

缓解：

- State 模式集中生命周期转换。
- 每个 transition 都有 trace 和 fail-closed reason。

### 6.8 Observability / Audit

需要支持：

- execution trace。
- host import trace。
- policy decision audit。
- resource usage metrics。
- sanitized trap diagnostics。
- ABI mismatch diagnostics。
- runtime unavailable diagnostics。

风险：

- 观测不足会让 WASM app 出错不可定位。
- 观测过度又会泄露 raw payload。

缓解：

- 统一 sanitized diagnostic schema。
- raw payload 永不进入 trace。

### 6.9 Guest SDK / Toolchain

需要支持：

- Rust guest SDK scaffold。
- WIT binding generation workflow。
- manifest builder integration。
- local test harness。
- mock host imports。
- certification fixtures。
- packaging command integration。

风险：

- 只有 runtime 没有 SDK，第三方开发者仍不可用。

缓解：

- SDK、testkit、fixtures 和 runtime 同步推进。

### 6.10 Certification / Conformance

需要支持：

- ABI conformance tests。
- permission conformance tests。
- sandbox escape negative tests。
- host import policy tests。
- deterministic snapshot tests。
- runtime unavailable fallback tests。
- example apps。

风险：

- 没有认证体系，Store 和第三方生态无法规模化。

缓解：

- 将 certification report 作为 package admission 的一等产物。

## 7. 风险矩阵

| 风险 | 严重性 | 说明 | 缓解 |
| --- | --- | --- | --- |
| Wasmtime 细节泄漏到公共 ABI | 高 | 会破坏 runtime 可替换性 | provider-neutral trait + Bridge |
| WASI 暴露过宽 | 高 | 可能泄露文件、环境变量、网络 | denied-by-default + capability preopen |
| host import 绕过 service runtime | 高 | 破坏服务化边界和审计 | Command + policy + trace |
| 生命周期不完整 | 高 | 无法支持长期运行 app | State + transition audit |
| 编译缓存污染 | 中 | 错误复用 artifact 或 policy profile | cache key 包含 hash、ABI、engine、policy |
| 日志泄露 raw payload | 高 | 安全和隐私风险 | sanitized diagnostics schema |
| SDK 与 runtime 不一致 | 中 | 开发者本地能跑，部署失败 | certification/testkit 复用 runtime specification |
| out-of-process 过早实现 | 中 | 拖慢主线，复杂度过高 | 默认 in-process provider 先落地，保留 hardened contract |
| 只做 runtime 不做工具链 | 高 | 工业不可用 | runtime、SDK、fixtures、certification 一起规划 |

## 8. 建议阶段拆分

后续 write-plan 建议拆成 8 个阶段，并进一步拆为多个 OpenSpec：

1. WASM Runtime Provider Contract：定义 provider-neutral runtime/provider/engine/session/diagnostics/service bridge contract。
2. WASM Package Admission & ABI Negotiation：完善 manifest artifact、WIT ABI version negotiation、compatibility specification。
3. Default In-Process Runtime Provider：实现默认可运行 provider，内部可选择 Wasmtime，但不泄漏到公共 contract。
4. Sandbox & Resource Governance：fuel/epoch、timeout、memory、payload、WASI deny-by-default、quota。
5. Host Import Service Portal：把 guest imports 接入 service runtime、policy、trace、capability registry。
6. Lifecycle / State / Checkpoint：补齐 init/start/event/render/pause/resume/shutdown/checkpoint/restore/upgrade。
7. Guest SDK / Toolchain / Local Test Harness：Rust guest SDK、binding scaffold、mock host、package builder、developer examples。
8. Certification / Fixtures / Hardened Provider Contract：conformance tests、negative tests、store certification report、out-of-process provider contract。

## 9. 推荐结论

选择 **Option D：分层双执行面架构**。

短期目标不是直接把 Wasmtime 塞进当前 skeleton，而是先把工业级边界定清楚：控制面、执行面、host imports、sandbox、toolchain、certification、observability 都必须成为一等设计对象。默认实现可以是 in-process runtime provider，但架构必须允许用户替换为 hardened out-of-process provider 或其他兼容 WASM engine。

这条路线改动会比 v0/v1 skeleton 大，但符合 Macaca 作为 Agent OS 基础设施的定位：WASM application 不是 demo runtime，而是未来多语言、第三方、可分发、可认证 application 生态的核心运行边界。
