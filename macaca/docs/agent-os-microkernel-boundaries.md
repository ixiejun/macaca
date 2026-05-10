# Macaca Agent OS 微内核边界治理

## 1. 目的

本文档是路线 C 阶段 0 的架构治理基线。它规定 Macaca Agent OS 中哪些能力可以进入微内核，哪些能力必须作为系统服务、插件、可选模块、Application Framework 或表现层存在。

后续阶段如果要改变这些边界，必须先更新 OpenSpec，并说明为什么该能力属于更底层。

## 2. 核心判断

Macaca 的微内核只承载系统不变量，不承载业务能力。

```text
Kernel owns invariants.
Services own replaceable capabilities.
Applications own business behavior and UI.
Store owns distribution and entitlement.
Plugins extend system surfaces.
Web3/EVM are optional modules.
```

## 3. 允许进入 Kernel 的能力

Kernel 只允许包含以下基础原语：

| 原语 | 职责 | 设计模式 |
| --- | --- | --- |
| Identity | 定义 app、agent、session、task、service、package、developer 的系统身份 | Value Object |
| Scheduler | 调度 agent/task/service，不写业务 workflow | Strategy |
| Capability Registry | 注册、发现 capability，不实现 capability 本身 | Registry |
| Service Registry | 注册、发现 system service，不绑定具体 provider | Registry / Facade |
| IPC / Service Call Facade | 统一服务调用入口，不绑定 transport | Bridge / Command |
| Policy Engine Facade | 权限、预算、地区、支付审批的统一判定入口 | Strategy / Specification |
| Trace / Audit Bus | 系统事件、审计事件、调用链事件的统一出口 | Observer |
| Resource Manager Facade | workspace、browser、driver process、network、storage 等资源声明和锁 | Mediator |
| Session Primitive | session 生命周期、pause/resume、checkpoint 的基础状态 | State / Memento |
| Task Primitive | goal、todo、dependency、review、resume 的基础状态机 | State / Mediator |
| Package Runtime Guard | package 加载前的签名、兼容性、权限、entitlement 前置守卫 | Chain of Responsibility |

## 4. 必须作为 System Service 的能力

以下能力不得直接进入 kernel。它们可以有内置实现，但必须通过 service contract 暴露：

| 能力 | 目标归属 | 原因 |
| --- | --- | --- |
| LLM Provider / Model Router | LLM Service | provider/model 变化频繁 |
| Memory / Context Engine | Memory / Context Service | 策略可替换 |
| TodoBoard / Planner / Review | Task Service | 是系统服务，不是 kernel 内部业务 |
| Driver Execution | Driver Service | driver 可由第三方提供 |
| Skill Runtime | Skill Service | skill 可安装、可加密、可订阅 |
| MCP Runtime | MCP Service | 外部协议适配，不属于 kernel |
| Gateway | Gateway Service | Discord/Telegram/飞书/钉钉等都应由 service/plugin 提供 |
| Store / Entitlement | Store Service | 商业分发与授权独立演进 |
| Payment / A2A | Payment Service | 支付协议和链上/链下实现可替换 |
| GenUI | UI Service | UI runtime 不应定义 kernel 语义 |
| Persistence Provider | Persistence Service | 本地 redb、远程 DB、对象存储等可替换 |

## 5. 必须作为 Plugin 的能力

Plugin 用来扩展系统能力面。第三方可以提供：

- Gateway plugin：Telegram、飞书、钉钉、WhatsApp、Slack、Email。
- Driver plugin：IDE、浏览器、Office、终端、桌面软件。
- Memory plugin：向量库、企业知识库、上下文提供器。
- Skill plugin：领域技能包。
- MCP plugin：外部工具和资源协议接入。
- Payment plugin：企业账单、链下支付、未来链上支付 adapter。
- Compliance plugin：审计、数据边界、企业安全策略。

Plugin 必须声明 manifest、capability、permissions、resources、lifecycle、trace schema。Plugin 不得绕过 service registry。

## 6. 必须作为 Optional Module 的能力

以下能力必须保持可选安装，缺失时不得影响 base OS：

| 可选模块 | 缺失时行为 |
| --- | --- |
| Web3 Node Module | 返回结构化 unavailable，不影响普通应用 |
| EVM / DApp Module | DApp capability unavailable，不影响普通应用 |
| 特定 Gateway | 外部入口不可用，不影响 Web/CLI |
| 特定 paid package | entitlement missing，不影响开源/免费包 |
| 特定 Driver | capability unavailable，不影响其他 driver |

可选模块必须支持 `available / unavailable / disabled_by_policy / region_blocked` 等状态，不得通过 panic 或 hang 表达缺失。

## 7. Application Framework 边界

Application Framework 负责：

- application manifest。
- package metadata。
- WASM Application ABI。
- YAML application compatibility adapter。
- application lifecycle。
- GenUI surface declaration。
- app-scoped storage。
- capability request。
- permission declaration。

Application Framework 不得直接实现 kernel policy、driver provider、payment provider、Web3 node。

## 8. Presentation Shell 边界

`macaca-web`、frontend、`macaca-cli` 的长期职责是 thin shell：

- HTTP/CLI/Gateway command adapter。
- SSE/trace viewer。
- GenUI renderer。
- permission/payment approval surface。
- package/store manager UI。

表现层不得定义 session/task/trace/payment/package 的系统语义，只能消费 system facade。

## 9. 禁止规则

- 禁止 application-specific 逻辑进入 kernel。
- 禁止在 kernel 中硬编码 app name、workflow、driver name、gateway name、chain name、payment provider。
- 禁止无 trace 的 service call。
- 禁止无 permission 的 capability call。
- 禁止 Web3/EVM 成为 base OS 必装依赖。
- 禁止为了快速 demo 绕过 package runtime guard。
- 禁止把 `macaca-web` 作为长期系统协调中枢。

## 10. 当前 crate 归属表

Route C 的 Rust workspace 目录拓扑位于 `macaca/crates/README.md`。文件系统 layer 用来表达所有权边界，但不等于依赖许可；依赖许可仍由 Route C dependency gate 和 allowlist 判定。

| crate | 当前路径 | 当前角色 | 路线 C 目标归属 |
| --- | --- | --- | --- |
| `macaca-proto` | `crates/foundation/macaca-proto` | 共享类型 | protocol / ABI / package / service / trace 类型底座 |
| `macaca-ipc` | `crates/foundation/macaca-ipc` | IPC | service bus / transport bridge |
| `macaca-persist` | `crates/foundation/macaca-persist` | 持久化 | persistence service contract |
| `macaca-kernel` | `crates/kernel/macaca-kernel` | 内核与协调 | microkernel facade 与系统不变量 |
| `macaca-task` | `crates/services/macaca-task` | 任务系统 | task service |
| `macaca-llm` | `crates/services/macaca-llm` | LLM 抽象 | LLM service |
| `macaca-memory` | `crates/services/macaca-memory` | 记忆 | memory/context service |
| `macaca-context` | `crates/services/macaca-context` | 上下文 | context service |
| `macaca-driver` | `crates/services/macaca-driver` | 驱动 | driver service/plugin runtime |
| `macaca-skill` | `crates/services/macaca-skill` | skill | skill service / package runtime |
| `macaca-gateway` | `crates/services/macaca-gateway` | 外部入口 | gateway service/plugin adapter |
| `macaca-tools` | `crates/services/macaca-tools` | 工具能力 | tool/skill service compatibility surface |
| `macaca-runtime` | `crates/runtime/macaca-runtime` | agentic runtime | runtime service primitive |
| `macaca-runtime-host` | `crates/runtime/macaca-runtime-host` | 宿主 | plugin/skill/MCP host facade |
| `macaca-framework` | `crates/runtime/macaca-framework` | framework | traced agent / middleware / MCP primitive |
| `macaca-agent` | `crates/application/macaca-agent` | agent 原语 | agent framework primitive |
| `macaca-app` | `crates/application/macaca-app` | application runtime | Application Framework |
| `macaca-sdk` | `crates/facade/macaca-sdk` | SDK | system facade / developer API |
| `macaca-web` | `crates/shells/macaca-web` | Web 入口 | thin shell / GenUI shell / trace viewer |
| `macaca-cli` | `crates/shells/macaca-cli` | CLI 入口 | command shell / service inspector |
| `macaca-integration-tests` | `crates/tests/macaca-integration-tests` | 集成测试 | cross-layer governance / regression |
