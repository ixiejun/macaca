# Macaca OS 路线 C 微内核生态实施总计划

> **给后续 agentic worker 的要求：** 实施本计划时必须使用 `superpowers:subagent-driven-development`（推荐）或 `superpowers:executing-plans`，按任务逐步执行。本文档是顶层路线图，后续每个阶段都需要拆成独立 OpenSpec 和更细的实施计划。

**目标：** 基于 `docs/openharmony-microkernel-architecture-for-macaca-agent-os.md` 中确定的路线 C，渐进式实现 Macaca OS 的长期架构：微内核 + 系统服务 + WASM Application ABI + Store + 可选 Web3/EVM 模块。

**架构策略：** 本文档不是单个实现任务，而是全系统重构路线图。它定义阶段顺序、依赖边界、OpenSpec 创建顺序、crate 归属、风险控制和验收门禁。每个阶段真正进入实现前，都必须再产出更细的 OpenSpec `proposal/design/tasks/spec` 和对应阶段 implementation plan。

**技术栈：** `macaca/crates` Rust workspace、OpenSpec、Superpowers plans、未来用于 Application ABI 的 WebAssembly Component Model/WASI、现有 Macaca task/trace/session 基础设施，以及未来可选的 Substrate/EVM 模块集成。

---

## 1. 计划范围

这份总计划刻意不写函数级代码修改，因为目标覆盖整个操作系统级架构：

- 微内核原语
- 系统服务注册表和服务总线
- Application package 与 WASM ABI
- GenUI runtime
- Plugin runtime
- Store、订阅、entitlement、加密 skill/application 分发
- Agent-to-Agent 协作与支付
- 可选 Web3 node 模块
- 可选 EVM/DApp 模块
- Web/CLI thin shell 迁移

如果把这些内容塞进一个 OpenSpec 或一个实现分支，会变成高风险重写。正确做法是：按阶段 additive-first 推进，每一步可编译、可验证、可回滚。现有 YAML/demo application 在整个迁移过程中必须持续可用。

## 2. Superpowers Brainstorm 结论

### 2.1 推荐路线

采用路线 C：

```text
微内核
  + 系统服务
  + WASM Application ABI
  + Store / entitlement
  + 可选 Web3 / EVM 模块
```

这条路线既保留 OS 级一致性，也支持第三方生态。它避免两个失败方向：

- 宏内核式 Agent 平台：所有能力都硬编码进 `macaca-web`、`macaca-kernel` 或 `macaca-framework`。
- 松散插件容器：trace、policy、payment、资源隔离、session、task 语义在各插件里碎片化。

### 2.2 核心设计原则

```text
Kernel 负责不可变系统约束。
Service 负责可替换能力。
Application 负责业务行为和 UI。
Store 负责分发与 entitlement。
Plugin 负责扩展系统能力面。
Web3/EVM 是可选模块。
```

### 2.3 不可妥协的约束

- 现有 application 必须继续运行。
- 现有 `/api/chat/v2`、task board、trace、resume、driver、skill、MCP、Web UI 链路不得退化。
- 所有行为变更必须先走 OpenSpec。
- 每个实现阶段必须 additive-first。
- kernel 中不得硬编码 application name、workflow、driver name、gateway name、chain name、payment provider。
- Web3/EVM 必须保持可选；缺失时不得影响基础 OS。
- GenUI 必须成为平台原语，但当前 chat/trace shell 必须继续可用。

## 3. 目标分层

### 3.1 最终分层模型

```text
Applications
  YAML app / WASM app / GUI app / headless app / paid app / Web3 app

Application Framework
  manifest, ABI, SDK, GenUI, lifecycle, permissions, package runtime

System Services
  LLM, Task, Memory, Context, Driver, Skill, MCP, Gateway, Store,
  Payment, Trace, Persistence, Web3, EVM

Microkernel
  identity, scheduler, service registry, IPC/service bus, policy,
  trace/audit bus, resource manager, session/task primitives,
  package runtime guard

Host Runtime
  process, workspace, sandbox, browser, filesystem, network,
  optional blockchain node
```

### 3.2 当前 crate 映射

| 目标层 | 当前 crate | 长期方向 |
| --- | --- | --- |
| 协议与 ABI | `macaca-proto` | 共享 identity、service、package、trace、entitlement、ABI 类型 |
| 持久化 | `macaca-persist` | event/session/package/license/checkpoint 存储 contract |
| IPC / Service Bus | `macaca-ipc` | 本地与远程 service call 平面 |
| 微内核 | `macaca-kernel` | registry、scheduler、policy、trace bus、resource/session/task primitives |
| Agent Framework | `macaca-agent`, `macaca-framework`, `macaca-runtime`, `macaca-runtime-host` | traced agent runtime 与 framework primitives |
| Application Framework | `macaca-app`, `macaca-sdk` | manifest、package、WASM ABI、app lifecycle、SDK |
| 系统服务 | `macaca-llm`, `macaca-memory`, `macaca-task`, `macaca-tools`, `macaca-driver`, `macaca-skill`, `macaca-gateway` | service contracts 与 plugin-compatible providers |
| Store / Commerce | 未来 `macaca-store` 或 service module | package install、signature、entitlement、metering |
| Web3 / EVM | 未来可选模块 | Substrate node integration、wallet、EVM、contract calls |
| 表现层 | `macaca-web`, frontend, `macaca-cli` | thin shell、GenUI renderer、API/gateway adapters |

## 4. 阶段总览

每个阶段都必须继续拆成一个或多个 OpenSpec change。下表列出阶段、目标产物和依赖关系。

| 阶段 | 名称 | 主要产物 | 前置依赖 |
| --- | --- | --- | --- |
| 0 | 基线与治理 | 回归测试矩阵、架构规则、阶段模板 | 无 |
| 1 | 微内核原语边界 | kernel primitive contract、service registry skeleton | 0 |
| 2 | 系统服务 Contract | service identity、lifecycle、call、trace、permission contracts | 1 |
| 3 | IPC / Service Bus | local-first service call bus、transport abstraction | 2 |
| 4 | Package Manifest 与 Runtime Guard | 统一 package metadata、signature/permission schema | 2 |
| 5 | Application ABI v0 | YAML 兼容 + WASM ABI 设计 + loader stub | 4 |
| 6 | GenUI Runtime v0 | UI intent/schema contract、shell renderer boundary | 5 |
| 7 | Plugin Runtime v0 | plugin manifest、install/register lifecycle、gateway plugin path | 4 |
| 8 | Store / Entitlement v0 | install source、license、subscription、metering、encrypted skill hooks | 4, 7 |
| 9 | A2A 协作与支付 v0 | agent identity、quote、payment intent、budget policy、receipt trace | 2, 8 |
| 10 | 可选 Web3 Node Module | optional wallet/node/signing service contract | 9 |
| 11 | 可选 EVM / DApp Module | Substrate/EVM adapter contract、DApp capability surface | 10 |
| 12 | Web/CLI Thin Shell 迁移 | presentation layer 中的 orchestration semantics 下沉 | 1-8 |
| 13 | 生态硬化 | compatibility、security、developer SDK、marketplace readiness | 全部 |

## 5. 阶段 0：基线与治理

### 目标

先建立安全底座，让后续阶段不会反复破坏现有 Agent OS 行为。

### 涉及范围

- `macaca-integration-tests`
- `macaca/docs`
- `docs`
- `openspec`
- CI 或本地验证脚本

### 必须产出

- 基线 E2E 场景：
  - 现有 YAML application 能启动
  - `/api/chat/v2` 能创建/恢复 session
  - goal -> planner -> task -> worker -> review -> coordinator resume 链路可跑
  - trace 实时推送和历史恢复可用
  - driver execution trace 可见
  - skill/MCP runtime smoke path 可用
  - task board 按 session scope 获取
- 架构治理文档：
  - 什么属于 kernel
  - 什么必须是 service
  - 什么必须是 plugin
  - 什么必须是 optional module
- 阶段实施模板：
  - Superpowers brainstorm
  - OpenSpec proposal/design/tasks/spec
  - GitNexus impact
  - additive implementation
  - targeted tests
  - integration smoke
  - commit

### 验收门禁

- 阶段 1 开始前，所有现有 smoke tests 必须通过。
- 后续任意阶段都必须能指出至少一个不能破坏的回归场景。

### 风险

- 现有自动化测试可能覆盖不到真实 Web UI 回归。
- 部分现有行为仍依赖手工验证。

### 后续细分计划

创建 `docs/superpowers/plans/YYYY-MM-DD-phase-0-baseline-governance.md`。

## 6. 阶段 1：微内核原语边界

### 目标

定义最小、稳定、不可或缺的 Macaca microkernel surface。

### 涉及范围

- `macaca-proto`
- `macaca-kernel`
- `macaca-sdk`
- `macaca/docs`

### 必须产出

- Kernel primitive 清单：
  - identity
  - scheduler
  - capability registry
  - service registry
  - IPC/service call facade
  - policy engine facade
  - trace/audit bus
  - resource manager facade
  - session primitive
  - task primitive
  - package runtime guard
- 需要时新增 additive Rust traits 或 data contracts：
  - `KernelServiceId`
  - `CapabilityId`
  - `ServiceScope`
  - `TraceContext`
  - `PolicyDecision`
  - `ResourceScope`
- 对不应继续被上层消费的 direct kernel internals 标记 deprecated。

### 明确非目标

- 不迁移 `macaca-web` 中所有现有逻辑。
- 不实现 WASM runtime。
- 不实现 Store。
- 不实现 Web3。

### 验收门禁

- 现有 `macaca-web` 和 application 流程仍能编译运行。
- 新 kernel primitive contract 不需要依赖 `macaca-web` 即可使用。
- kernel 中不出现 application-specific 名称。

### 风险

- Kernel abstraction 容易过度泛化。必须保持为明确 typed contract，不做空泛企业式抽象。

### 后续细分计划

创建 `docs/superpowers/plans/YYYY-MM-DD-phase-1-microkernel-boundary.md`。

## 7. 阶段 2：系统服务 Contract

### 目标

引入所有可替换能力都能实现的统一 system service model。

### 涉及范围

- `macaca-proto`
- `macaca-kernel`
- `macaca-llm`
- `macaca-memory`
- `macaca-task`
- `macaca-driver`
- `macaca-skill`
- `macaca-gateway`
- `macaca-tools`

### 必须产出

- `SystemService` contract：
  - service id
  - service type
  - capabilities
  - lifecycle
  - health check
  - permissions
  - trace schema
  - cleanup behavior
- kernel facade 中的 service registration path。
- 携带 trace metadata 的 service call result format。
- 现有内置服务的初始 adapters。

### Service Types

- `llm`
- `memory`
- `context`
- `task`
- `trace`
- `driver`
- `skill`
- `mcp`
- `gateway`
- `store`
- `payment`
- `web3`
- `evm`
- `ui`

### 验收门禁

- 现有内置 LLM、task、driver、skill、gateway 可以被描述成 service。
- 现有 runtime path 可以继续使用旧 direct calls，但新 contract 已经存在。

### 风险

- 一次性迁移所有 service 风险过高。阶段 2 只定义和适配，不做全面重写。

### 后续细分计划

创建 `docs/superpowers/plans/YYYY-MM-DD-phase-2-system-service-contract.md`。

## 8. 阶段 3：IPC / Service Bus

### 目标

让 service invocation 与 transport 解耦，先从 local-first 开始。

### 涉及范围

- `macaca-ipc`
- `macaca-proto`
- `macaca-kernel`
- `macaca-runtime-host`
- `macaca-framework`

### 必须产出

- `ServiceCommand`
- `ServiceEnvelope`
- `ServiceReply`
- `ServiceTransport`
- 本地进程内 transport adapter。
- 未来 transport extension points：
  - process-local
  - child process
  - MCP
  - HTTP
  - signed remote A2A
- 强制 trace context propagation。

### 验收门禁

- 一个 local service call 可以通过 service bus 发起，并产生 trace。
- 现有 direct calls 继续通过 adapters 保持可用。

### 风险

- 如果过早序列化所有 hot-path calls，会带来性能开销。初期应使用 typed in-process transport，只在边界处序列化。

### 后续细分计划

创建 `docs/superpowers/plans/YYYY-MM-DD-phase-3-service-bus.md`。

## 9. 阶段 4：Package Manifest 与 Runtime Guard

### 目标

为 Application、Plugin、Skill、MCP、Driver 和 optional modules 定义统一 package metadata。

### 涉及范围

- `macaca-proto`
- `macaca-app`
- `macaca-skill`
- `macaca-driver`
- `macaca-runtime-host`
- 未来 `macaca-store`

### 必须产出

- Package manifest v0：
  - package id
  - package type
  - version
  - developer id
  - signature metadata
  - runtime kind
  - ABI version
  - permissions
  - service requirements
  - optional service requirements
  - commerce metadata
- Package runtime guard：
  - validate manifest
  - validate compatibility
  - validate permissions
  - reject unsupported package type with explainable error
- 当前 YAML applications 的 compatibility adapter。

### Package Types

- application
- skill
- plugin
- mcp
- driver
- system_module
- ui_component_pack

### 验收门禁

- 现有 YAML applications 可以被表示成 package manifests。
- 不支持的 package type 会安全失败，并返回可解释错误。

### 风险

- Store 概念可能过早泄漏。commerce metadata 在 entitlement 阶段前必须保持 inert。

### 后续细分计划

创建 `docs/superpowers/plans/YYYY-MM-DD-phase-4-package-manifest-runtime-guard.md`。

## 10. 阶段 5：Application ABI v0

### 目标

把 Application 从 YAML-only 扩展为标准 package model，并为未来 WASM 支持建立 ABI。

### 涉及范围

- `macaca-app`
- `macaca-sdk`
- `macaca-proto`
- `macaca-framework`
- `macaca-runtime-host`
- `macaca-web`

### 必须产出

- Application ABI v0 文档：
  - lifecycle exports
  - event handling
  - capability calls
  - task calls
  - trace calls
  - UI calls
  - storage calls
  - payment calls
- ABI 概念对应的 Rust SDK facade。
- Loader abstraction：
  - YAML loader
  - future WASM component loader stub
  - hybrid package loader stub
- Capability declaration and resolution path。

### 验收门禁

- 当前 YAML applications 继续可加载。
- package-shaped application metadata 可以加载，但不要求执行 WASM。
- Web UI 代码不需要理解 application package 内部细节。

### 风险

- WASM runtime 实现容易把阶段 5 拖成大项目。阶段 5 只定义 ABI 和 loader stub，除非另行批准，不实现完整 WASM 执行。

### 后续细分计划

创建 `docs/superpowers/plans/YYYY-MM-DD-phase-5-application-abi-v0.md`。

## 11. 阶段 6：GenUI Runtime v0

### 目标

让 application-specific UI 成为平台原语，同时保留当前 chat/trace shell。

### 涉及范围

- `macaca-proto`
- `macaca-app`
- `macaca-framework`
- `macaca-web`
- `frontend`

### 必须产出

- `UiIntent`
- `UiEvent`
- `UiComponentTree` 或 UI schema v0
- frontend 中的 `GenUiRenderer` boundary
- trace overlay contract
- permission prompt contract
- application UI route/shell mounting model

### 验收门禁

- 现有 chat UI 仍是默认入口。
- application 可以声明 custom UI surface。
- UI events 能带 trace 回流到 application/session。

### 风险

- 完全 UI 自由会带来安全问题。先从 declarative schema 或受限 component tree 开始，不直接开放任意远程 UI 代码。

### 后续细分计划

创建 `docs/superpowers/plans/YYYY-MM-DD-phase-6-genui-runtime-v0.md`。

## 12. 阶段 7：Plugin Runtime v0

### 目标

允许第三方在不修改核心代码的情况下扩展系统能力。

### 涉及范围

- `macaca-proto`
- `macaca-kernel`
- `macaca-runtime-host`
- `macaca-gateway`
- `macaca-driver`
- `macaca-skill`
- `macaca-memory`
- `macaca-tools`

### 必须产出

- Plugin manifest v0。
- Plugin lifecycle：
  - install
  - register
  - start
  - stop
  - uninstall
  - health check
- Plugin-provided service registration。
- Gateway plugin adapter path。
- Driver plugin adapter path。
- Memory/context plugin adapter path。

### 验收门禁

- 现有内置 Discord 或类似 gateway 功能可以被建模为 plugin-provided gateway。
- plugin 缺失不会影响 base OS。

### 风险

- 任意 plugin code execution 风险很高。先从 manifest + built-in adapter modeling 开始，再考虑第三方二进制加载。

### 后续细分计划

创建 `docs/superpowers/plans/YYYY-MM-DD-phase-7-plugin-runtime-v0.md`。

## 13. 阶段 8：Store / Entitlement v0

### 目标

为 paid skills、MCP、plugins、applications 建立商业化基础。

### 涉及范围

- 未来 `macaca-store`
- `macaca-proto`
- `macaca-persist`
- `macaca-skill`
- `macaca-app`
- `macaca-runtime-host`
- `macaca-web`

### 必须产出

- Store package source abstraction。
- Package signature verification contract。
- Entitlement model：
  - free
  - open source
  - paid one-time
  - subscription
  - usage-metered
  - enterprise license
- License check service。
- Encrypted skill loading hook。
- Paid application install guard。
- Usage metering event。

### 验收门禁

- free/open packages 继续可用。
- paid package metadata 可以表达，并能在无 entitlement 时被拒绝。
- skill/application execution path 可以在执行前查询 entitlement。

### 风险

- 本地代码无法保证绝对防破解。必须采用分层保护：signature、entitlement、metering，以及高价值资产可选 remote execution。

### 后续细分计划

创建 `docs/superpowers/plans/YYYY-MM-DD-phase-8-store-entitlement-v0.md`。

## 14. 阶段 9：A2A 协作与支付 v0

### 目标

支持 agent-to-agent collaboration 和 paid service exchange，同时不把 kernel 绑定到某一个具体协议。

### 涉及范围

- `macaca-proto`
- `macaca-kernel`
- `macaca-ipc`
- `macaca-task`
- `macaca-persist`
- 未来 payment service

### 必须产出

- Agent identity contract。
- Remote capability discovery contract。
- Quote request/response。
- Payment intent。
- Budget policy。
- Approval policy。
- Receipt trace event。
- Protocol adapter interface：
  - local
  - signed HTTP
  - MCP-based
  - future Ethereum/EIP/ERC adapter
  - enterprise billing adapter

### 验收门禁

- local A2A service exchange 可以在无真实支付的情况下表达。
- payment intent 可以创建、批准/拒绝、trace、持久化。
- A2A disabled 时不影响现有 task execution。

### 风险

- Agent 自主花钱风险很高。任何真实支付 provider 前，budget 和 approval 必须先成为 kernel policy concepts。

### 后续细分计划

创建 `docs/superpowers/plans/YYYY-MM-DD-phase-9-a2a-payment-v0.md`。

## 15. 阶段 10：可选 Web3 Node Module

### 目标

把 Web3 node capability 作为 optional installable module 加入系统。

### 涉及范围

- 未来 `macaca-web3`
- `macaca-proto`
- `macaca-kernel`
- `macaca-ipc`
- `macaca-persist`
- `macaca-web`

### 必须产出

- Web3 module manifest。
- Wallet service contract。
- Signing policy。
- Transaction service contract。
- Chain query service contract。
- Region/compliance disabled state。
- Optional installation and absence behavior。

### 验收门禁

- Web3 module 缺失时 base OS 正常工作。
- App 查询 Web3 availability 时能得到 unavailable response。
- Application 无法直接读取私钥。

### 风险

- 不同地区监管差异很大。Web3 必须 opt-in 且 policy-gated。

### 后续细分计划

创建 `docs/superpowers/plans/YYYY-MM-DD-phase-10-optional-web3-node.md`。

## 16. 阶段 11：可选 EVM / DApp Module

### 目标

通过 optional Substrate/EVM-compatible module 支持 AI + Web3 DApp 开发。

### 涉及范围

- 未来 `macaca-evm`
- 未来 `macaca-web3`
- `macaca-proto`
- `macaca-app`
- `macaca-sdk`

### 必须产出

- EVM service contract：
  - deploy contract
  - call contract
  - read state
  - subscribe events
  - estimate gas
  - transaction receipt
- Substrate/EVM adapter design。
- DApp application capability declaration。
- EVM disabled/unavailable behavior。

### 验收门禁

- DApp capability 可以声明，并在 EVM module 缺失时安全拒绝。
- Contract call 可以表示为 service command 和 trace event。

### 风险

- 运行链节点有较高运维成本。EVM module 必须 optional 且 adapter-based。

### 后续细分计划

创建 `docs/superpowers/plans/YYYY-MM-DD-phase-11-optional-evm-dapp.md`。

## 17. 阶段 12：Web/CLI Thin Shell 迁移

### 目标

把剩余 OS semantics 从 presentation entrypoints 中迁出。

### 涉及范围

- `macaca-web`
- `frontend`
- `macaca-cli`
- `macaca-sdk`
- `macaca-app`
- `macaca-kernel`

### 必须产出

- Web routes 调用 SDK/application/kernel facades。
- `macaca-web` 不再拥有核心 session/task/resume semantics。
- Frontend 变成：
  - Macaca Shell
  - trace viewer
  - GenUI renderer
  - package/store manager UI
  - permission/payment approval surface
- CLI 变成：
  - command adapter
  - service inspector
  - package installer
  - daemon controller

### 验收门禁

- 现有 Web UI 继续可用。
- 新 GenUI app surface 可以 mount，且不替换 chat shell。
- CLI 可以通过 SDK facade 调用 system services。

### 风险

- `macaca-web` 是当前最高风险集成点。这个阶段必须拆成很多小 OpenSpec change。

### 后续细分计划

创建 `docs/superpowers/plans/YYYY-MM-DD-phase-12-web-cli-thin-shell.md`。

## 18. 阶段 13：生态硬化

### 目标

让 Macaca OS 具备真实第三方开发者和 marketplace 使用条件。

### 涉及范围

- Developer docs
- SDK examples
- Package signing
- Store policy
- Security review
- Compatibility testing
- E2E certification
- Migration guides

### 必须产出

- Developer guide：
  - YAML app
  - WASM app
  - GenUI app
  - plugin
  - gateway plugin
  - skill package
  - paid package
  - Web3 app
- Certification tests。
- Package compatibility checker。
- Security checklist。
- Store submission checklist。
- OS upgrade compatibility policy。

### 验收门禁

- 第三方开发者可以在不修改 Macaca 源码的情况下，构建、打包、安装、运行、trace，并可选地商业化一个 application。

### 风险

- 生态支持范围可能膨胀。第一条认证路径必须窄：一个 YAML app、一个 WASM-stub app、一个 plugin、一个 GenUI app、一个 paid-simulated package。

### 后续细分计划

创建 `docs/superpowers/plans/YYYY-MM-DD-phase-13-ecosystem-hardening.md`。

## 19. 横切设计规则

### 19.1 Trace Rule

所有系统活动必须携带 trace context：

- service calls
- app lifecycle
- UI events
- plugin calls
- driver calls
- skill usage
- MCP calls
- gateway events
- entitlement checks
- payment intents
- Web3 transactions
- EVM calls

无 trace，不执行。

### 19.2 Permission Rule

所有 capability call 都必须经过 policy：

- application permissions
- plugin permissions
- user approvals
- spending budgets
- region compliance
- optional module availability

无权限，不调用。

### 19.3 Optional Module Rule

可选模块不得破坏 base OS：

- Web3 缺失是合法状态。
- EVM 缺失是合法状态。
- Gateway plugin 缺失是合法状态。
- Paid package unavailable 是合法状态。

缺失必须返回结构化 unavailable error，不能 panic 或 hang。

### 19.4 Backward Compatibility Rule

在正式迁移路径存在之前，现有 YAML apps 必须继续作为一等兼容 application。

### 19.5 Store Rule

Store 支持不能阻断开源和本地开发包；但 paid packages 必须被 entitlement-gated。

## 20. 后续 OpenSpec 创建顺序

应按以下顺序创建 OpenSpec changes：

1. `define-microkernel-primitive-boundary`
2. `add-system-service-contract`
3. `add-local-service-bus`
4. `add-package-manifest-runtime-guard`
5. `add-application-abi-v0`
6. `add-genui-runtime-v0`
7. `add-plugin-runtime-v0`
8. `add-store-entitlement-v0`
9. `add-a2a-payment-v0`
10. `add-optional-web3-node-module`
11. `add-optional-evm-dapp-module`
12. `migrate-web-cli-to-thin-shell`
13. `add-ecosystem-hardening-certification`

每个 change 都必须包含：

- `proposal.md`
- `design.md`
- `tasks.md`
- delta specs
- migration notes
- compatibility notes
- regression tests

## 21. 推荐执行节奏

采用小批次实现：

- 每个 phase 或 subphase 一个 OpenSpec change。
- 每个 implementation batch 只处理一个 crate family。
- 每个 commit 只落一个 public contract。
- 每个 commit 只迁移一个 migration slice。
- 每个 phase 后跑 E2E smoke。

每个阶段内部推荐顺序：

1. 写或更新文档。
2. 添加 proto/types/contracts。
3. 添加 facade/adapters，不改变行为。
4. 为新 contract 添加测试。
5. 迁移一个内部 consumer。
6. 迁移剩余 consumers。
7. 标记旧 direct path deprecated。
8. 跑 integration smoke。
9. commit。

## 22. 风险表

| 风险 | 影响 | 缓解 |
| --- | --- | --- |
| 过早过度抽象 | 拖慢开发，引入无用类型 | 每个 contract 必须至少映射一个当前 consumer 和一个明确未来扩展点 |
| trace/session 行为被破坏 | 用户可见回归 | 涉及 runtime 的每个阶段都必须跑 trace/replay E2E |
| Web UI 过早 shell 化 | demo 不可用 | chat shell 和 trace UI 在 GenUI 稳定前继续作为默认入口 |
| WASM runtime 范围膨胀 | 延迟核心重构 | 阶段 5 只做 ABI 和 loader stub |
| Store/payment 范围膨胀 | 法务和安全复杂度上升 | 阶段 8 先做 metadata、entitlement simulation 和 denial path |
| Web3 合规风险 | base OS 采用受阻 | Web3/EVM optional 且默认关闭 |
| Plugin 安全风险 | 任意代码执行风险 | 先做 manifest + built-in adapters，再考虑第三方二进制执行 |
| Application package 演进不兼容 | 生态破坏 | ABI versioning 和 compatibility checker |

## 23. 成功定义

路线 C 成功的标准：

- Macaca kernel 暴露稳定 primitives，且不包含 application-specific 逻辑。
- 可替换能力都成为 system services 或 plugins。
- 现有 YAML applications 仍然可运行。
- 新 package-shaped applications 可以热安装，不需要重启。
- WASM Application ABI 已定义，至少一个 stub/fixture app 可加载。
- GenUI application surface 可以在 chat-only model 之外渲染。
- Gateway/driver/memory/skill 能力可以由 plugin 提供。
- Store entitlement 可以 allow/deny package execution。
- Paid skill/application metadata 可表达、可 trace。
- A2A payment intent 具备 budget 和 approval。
- Web3/EVM modules 可选，且缺失安全。
- Web/CLI 是 system facades 之上的 thin presentation/command shells。

## 24. 立即下一步

不要开始编码整个路线图。下一步应该是：

```text
创建 OpenSpec change: define-microkernel-primitive-boundary
```

这个第一个 change 只定义并添加 additive contracts for kernel primitives。不要实现 WASM、Store、Plugin binary loading、Web3、EVM 或 GenUI。
