# Route C 非内核能力服务化与模块化设计

## 背景

`docs/openharmony-microkernel-architecture-for-macaca-agent-os.md` 已经明确 Macaca OS 选择路线 C：

```text
微内核 + 系统服务 + WASM Application ABI + Store + 可选 Web3/EVM 模块
```

现有 `docs/superpowers/plans/route-c-microkernel-ecosystem/` 下的阶段 0-13 计划建立了治理、contract、package、service bus、entitlement、Web3/EVM 等基础代码，但这些实现仍主要停留在“地基”层：

- `macaca-kernel` 仍直接依赖 LLM、Memory、Task、Tools、Persist 等可替换能力。
- `macaca-web` 仍直接拼装 Application、Task loop、Driver、Skill、MCP、Memory、LLM、Trace、Gateway 等系统能力。
- Store、Payment、Web3、EVM 已有部分类型或 mock，但还没有作为可安装 optional module 接入统一 service runtime。
- Application、Driver、Skill、MCP、Gateway、Memory、LLM 等能力仍主要以 crate 直接调用存在，而不是通过 system service contract + service bus + policy + trace 被调用。

这意味着当前架构仍有明显宏内核倾向。Route C 的后续工作必须从“定义 contract”进入“真实迁移边界与消费路径”。

## Superpowers Brainstorm

### 当前问题

Macaca 的长期目标不是一个 Agent 编排平台，而是一个 7x24 小时运行、支持第三方应用和可安装模块的 Agent OS。要达到这个目标，内核必须只保留不可绕开的系统不变量：

- identity
- scheduler
- IPC / service bus
- policy / permission
- trace / audit
- resource manager
- session/task primitive
- service registry
- package runtime guard

其余能力都必须变成可替换、可安装、可审计、可权限治理的 system service 或 module：

- Application
- Driver
- Skill
- MCP
- Gateway
- Memory
- LLM
- Store
- Payment
- Web3
- EVM

### 方案 A：继续在现有 crate 上追加 facade

做法：保持当前依赖关系，只在每个 crate 外面包一层 facade。

优点：

- 短期改动小。
- 对现有链路冲击最小。

缺点：

- `macaca-kernel` 和 `macaca-web` 依然直接依赖具体服务 crate。
- facade 会变成装饰，不会形成真实服务边界。
- 后续 Store、Web3、EVM、Plugin 仍会继续穿透 Web/Kernal。

结论：不能满足用户提出的“非内核能力真正服务化、模块化”要求。

### 方案 B：新增完整 service runtime，把现有能力逐步包成 built-in service provider

做法：

1. 保留现有 crate 的内部实现。
2. 为每类能力定义 typed service contract。
3. 在 `macaca-runtime-host` 或新增 `macaca-service-runtime` 中注册 built-in provider。
4. 上层 consumer 改为通过 `ServiceRuntime` / `SystemFacade` / `ServiceBus` 调用。
5. 旧 direct path 标记 deprecated，并用依赖门禁逐步禁止。

优点：

- additive-first，可小步迁移。
- 现有功能可 1:1 保持。
- 第三方 provider、远程 service、plugin service、optional module 可以复用同一模型。
- 可以通过 trace/policy/metering decorator 保证所有调用可审计。

缺点：

- 需要较多迁移切片。
- 初期会同时存在 direct path 和 service path，需要严格治理。

结论：推荐。

### 方案 C：一次性拆分所有 crate，重写为服务架构

做法：快速重组 workspace，强制把所有 capability 移出 kernel/web。

优点：

- 理论上边界最干净。

缺点：

- 风险极高。
- 会破坏已有 demo、trace、resume、task board、driver、skill、MCP 链路。
- 不符合“每次只做小的、可审查、可逆变更”的项目约定。

结论：禁止。

## 推荐架构

推荐方案 B：以 `ServiceRuntime` 为中心，做渐进式真实服务化。

```text
Application / Agent / Web / CLI / Gateway
  ↓
SystemFacade / SDK Facade
  ↓
ServiceRuntime
  ↓
Policy + Trace + Resource + Entitlement Decorators
  ↓
ServiceBus
  ↓
Built-in Service Provider / Plugin Provider / Remote Provider / Optional Module
```

### 内核边界

`macaca-kernel` 的最终职责：

- 定义不可绕开的系统原语。
- 持有 service/capability/resource/session/task/policy/trace 的 registry 或 facade。
- 不直接构建 LLM provider、Memory backend、Driver runtime、Skill runtime、Gateway runtime、Application runtime。
- 不直接依赖应用层 crate。

允许保留在 kernel 的内容：

- identity value objects
- service registry facade
- policy facade
- trace/audit bus facade
- resource manager facade
- session/task primitive state machine
- scheduler contract
- package runtime guard interface

必须迁出的内容：

- LLM provider construction
- Memory backend implementation
- Task PlanLoop/WorkerLoop orchestration implementation
- Driver execution implementation
- Skill/MCP runtime implementation
- Gateway adapters
- Store/payment/Web3/EVM provider implementation

### 系统服务边界

所有服务必须满足同一组运行时约束：

- 有 `ServiceDescriptor`。
- 有 lifecycle：install/register/start/call/stop/cleanup。
- 有 typed request/response 或稳定 envelope schema。
- 每次 call 必须携带 `TraceContext`。
- 每次 call 必须经过 policy。
- 可声明 required/optional capabilities。
- 缺失 optional module 时返回结构化 `Unavailable`，不得 panic/hang。
- 可由 built-in provider、plugin provider、remote provider、optional module provider 实现。

### 可安装模块边界

可安装模块不是普通 crate 依赖，而是 package + manifest + runtime guard + service provider：

```text
package manifest
  -> runtime guard
  -> entitlement check
  -> module host
  -> service registration
  -> service call via bus
  -> trace/metering/audit
```

Application、Driver、Skill、MCP、Gateway、Store connector、Payment adapter、Web3 node、EVM adapter 都应进入这个模型。

## 设计模式使用

| 设计模式 | 用途 |
| --- | --- |
| Facade | `SystemFacade` / `ApplicationHost` / `KernelFacade` 隐藏内部复杂度 |
| Bridge | service contract 与 local/remote/plugin transport 解耦 |
| Adapter | 现有 crate 实现适配为 service provider |
| Strategy | provider routing、policy、scheduler、resource allocation、payment policy |
| Command | service call、UI event、payment intent、contract call |
| Decorator | trace、policy、entitlement、metering、resource lock 包装调用链 |
| Observer | trace/audit/event bus |
| State | service/module/session/task/payment/plugin lifecycle |
| Memento | session checkpoint、task history、receipt、event replay |
| Specification | manifest、permission、compatibility、entitlement、module availability 校验 |
| Abstract Factory | 按 manifest/runtime kind/provider type 创建 service provider |
| Null Object | 缺失 optional module 时提供 unavailable provider |

## 服务化目标映射

| 能力 | 目标服务 | 现有实现迁移方式 |
| --- | --- | --- |
| Application | Application Service / Application Framework | `macaca-app` 保留实现，注册为 built-in app service |
| LLM | LLM Service | `macaca-llm` provider/router 迁为 LLM provider adapter |
| Memory/Context | Memory Service / Context Service | `macaca-memory` 和 `macaca-context` 迁为 memory/context provider |
| Task/Planner/Review | Task Service | `macaca-task` 提供 task board/plan/review service，kernel 只保留 primitive |
| Driver | Driver Service | `macaca-driver` runtime 迁为 driver provider，第三方 driver 走 plugin/module |
| Skill | Skill Service | `macaca-skill` 迁为 skill package/service provider |
| MCP | MCP Service | `macaca-runtime-host` MCP runtime 迁为 MCP provider/module host |
| Gateway | Gateway Service | `macaca-gateway` 和第三方入口都走 gateway provider |
| Store/Entitlement | Store Service | entitlement facade 升级为 store service provider |
| Payment/A2A | Payment Service | payment intent/receipt/policy 迁为 payment provider |
| Web3 | Optional Web3 Service | 未安装返回 unavailable，安装后注册 wallet/signing/tx provider |
| EVM | Optional EVM Service | 作为 Web3 子模块，注册 contract call/read/deploy provider |
| Web/CLI | Presentation Shell | 只通过 SDK/SystemFacade 调用服务，不持有系统语义 |

## 风险

- 双路径风险：direct path 和 service path 并存时可能产生 trace 重复或行为不一致。
- 性能风险：所有调用都走 envelope/serde 会增加开销，第一阶段应 local typed-first。
- 过度抽象风险：service contract 不能只剩 `serde_json::Value`，核心路径必须 typed-first。
- 回归风险：现有 `/api/chat/v2`、trace、task board、resume、driver、skill/MCP 最容易受影响。
- 生态风险：Store/Payment/Web3 如果提前绑定具体 provider，会污染长期架构。

## 必须增加的治理门禁

1. `macaca-kernel` forbidden dependency gate：
   - 禁止依赖 `macaca-app`、`macaca-driver`、`macaca-skill`、`macaca-gateway`、`macaca-llm`、`macaca-memory`、`macaca-tools` 等 provider crate。
   - 允许短期例外必须登记在 migration allowlist，并有到期阶段。

2. `macaca-web` orchestration shrink gate：
   - 禁止新增直接构建 PlanLoop/WorkerLoop/Agent/Driver/Skill/MCP 的 Web 代码。
   - 新路径必须走 `SystemFacade`。

3. service call trace gate：
   - 所有 service call 无 trace context 必须拒绝。

4. policy gate：
   - 所有 capability call 必须经过 permission/resource/entitlement policy。

5. optional module gate：
   - Web3/EVM/某 gateway/某 driver 缺失时必须返回结构化 unavailable。

## 结论

Route C 不能继续只实现 contract skeleton。下一轮必须开始做真实迁移：先建立统一 `ServiceRuntime` 和依赖门禁，再逐个把 LLM、Memory、Task、Driver、Skill、MCP、Gateway、Application、Store、Payment、Web3、EVM 从 direct crate call 迁移为 service provider。每个迁移切片都必须保持现有功能 1:1，且通过 Route C regression matrix 验证。
