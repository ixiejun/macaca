# OpenHarmony 微内核启发下的 Macaca Agent OS 生态架构研究

## 1. 研究目的

Macaca Agent OS 仍处在快速开发阶段，能力形态尚未冻结。当前已有 Agent、Application、Task、Driver、Skill、MCP、Memory、Trace、Gateway、Web UI 等基础设施，但这些能力如果继续以“所有东西都进核心”的方式增长，最终会走向宏内核式复杂度：内核、Web 后端、Application 语义、Driver、Skill、LLM、Memory、Gateway、支付、商店、UI 展示全部交织在一起，任何新增能力都会穿透多个层级。

本报告重新研究 OpenHarmony 的分层、微内核思想、系统能力服务化、组件化、可裁剪和生态模型，并把 Macaca 的目标从“一个能跑 Agent 的开发框架”上升为“一个真正支持第三方软件生态的 Agent 操作系统”：

- Macaca 不只是 Chat UI，也不只是几个 demo application。
- Macaca 需要支持真正意义上的上层软件开发，类似 Windows / macOS / iOS / HarmonyOS / 微信小程序上的应用生态。
- Application 可以有 GUI，也可以无 GUI；可以用 Go、Rust、C++、TypeScript、Python 等语言开发，只要能编译为 WASM 并遵循 Macaca Application ABI / SDK / 权限协议。
- GenUI 应成为基础能力，让每个 Application 都能拥有符合自身业务、品牌和设计语言的 UI，而不是复用单一聊天界面。
- Plugin、Skill、MCP、Application、Gateway、Driver、Memory、A2A 支付、Web3 节点、EVM/DApp 能力都应成为可安装、可治理、可计费、可审计的系统能力。

本报告不是实现计划，也不是 OpenSpec proposal。它用于定义后续架构设计的方向：**Macaca 应走微内核 + 系统服务 + WASM Application + Plugin/Store/Web3 可选模块的 OS 路线，而不是继续把所有能力塞进基础设施层。**

## 2. 资料来源与外部参考

主要参考：

- [OpenHarmony Overview](https://github.com/openharmony/docs/blob/master/en/OpenHarmony-Overview.md)
- [OpenHarmony 官方文档仓库](https://github.com/openharmony/docs)
- [OpenHarmony 官方站点](https://www.openharmony.cn/)
- [OpenHarmony 源码组织](https://gitee.com/openharmony)
- [WebAssembly Component Model](https://component-model.bytecodealliance.org/)
- [WASI](https://wasi.dev/)
- [Substrate / Polkadot SDK](https://github.com/paritytech/polkadot-sdk)
- [Frontier EVM pallet / Substrate Ethereum compatibility](https://github.com/polkadot-evm/frontier)
- Ethereum Agent / A2A 相关探索：以 EIP/ERC、智能合约钱包、账号抽象、agent identity、agent payment、on-chain reputation 等方向为代表，具体标准仍在快速演进中，Macaca 不应绑定单一草案，而应预留 protocol adapter。

说明：

- OpenHarmony 不是单一纯微内核系统。它支持不同设备形态下的可裁剪内核和系统服务组合。Macaca 应借鉴的是它的“内核小、服务化、组件化、统一系统能力、应用生态”思想，而不是机械复制其移动 OS 实现。
- Web3 / A2A 标准仍在变化。本报告只把它作为 Macaca 必须支持的能力方向，不把任何尚未稳定的 ERC/EIP 写死为底层内核协议。

## 3. Superpowers Brainstorm 结论

### 3.1 问题定义

用户提出的六点要求本质上指向同一个架构命题：

> Macaca 不能只是 Agent 编排平台，而应成为可运行第三方 Application、可安装系统服务、可形成商业生态、可接入 Web3 和 A2A 交易网络的 Agent OS。

这意味着 Macaca 的“Application”不能继续只被理解为 YAML 配置 + persona + tools。配置型 application 可以继续存在，但它只是最轻量的一种应用形态。长期上，Macaca 至少要支持：

- 配置包 Application
- WASM 二进制 Application
- GUI Application
- Headless service Application
- GenUI Application
- Plugin-enhanced Application
- Store-distributed paid Application
- A2A / Web3 enabled Application

### 3.2 可选架构路线

#### 路线 A：继续宏内核式集成

所有能力继续进入 `macaca-web`、`macaca-kernel`、`macaca-framework` 或若干 runner。

优点：

- 短期实现快。
- demo 迭代成本低。
- 初期调试路径少。

缺点：

- 无法支撑第三方生态。
- Application、Plugin、Skill、Driver、Gateway、Payment 都会变成硬编码分支。
- 任何付费订阅、防破解、热安装、能力治理都会反复穿透底层。
- 很难让不同语言、不同 UI、不同分发形态的应用安全运行。

结论：不推荐。这会把 Macaca 锁死为一个复杂 Agent 平台，而不是 OS。

#### 路线 B：完全插件平台化

Macaca 只做一个插件宿主，所有能力都由插件提供。

优点：

- 极度灵活。
- 第三方扩展容易。
- 核心很小。

缺点：

- OS 级一致性不足。
- Trace、权限、支付、生命周期、资源隔离、任务调度可能碎片化。
- 安全边界和商业分发难以统一。
- Application 生态会变成“插件市场”，而不是真正操作系统上的软件生态。

结论：也不推荐。Macaca 需要插件，但不能退化成无内核治理的插件容器。

#### 路线 C：微内核 + 系统服务 + WASM Application ABI + Store + 可选 Web3 模块

内核只保留身份、调度、IPC、权限、trace、资源、session/task、服务注册等不可或缺原语。Application、Driver、Skill、MCP、Gateway、Memory、LLM、Store、Payment、Web3、EVM 都作为系统服务或可安装模块存在。

优点：

- 具备 OS 级一致性。
- 支持第三方 Application 和 Plugin 生态。
- 支持热安装、付费订阅、权限治理、防破解。
- 支持多语言应用开发，只要编译到 WASM 并遵循 ABI。
- 支持 GenUI，使应用界面不再被聊天框限制。
- 支持 Web3 / A2A 支付作为可选能力，不污染核心用户场景。

缺点：

- 需要设计清晰 ABI、权限模型、包格式、商店协议和运行时。
- 初始工程量更大。
- 必须非常克制，避免把“未来能力”一次性过度实现。

结论：推荐路线。Macaca 应按这条路线设计长期架构，但实现上必须渐进，每次只落一个基础 contract。

## 4. OpenHarmony 可借鉴的核心机制

### 4.1 分层架构

OpenHarmony 的典型结构可以概括为：

```text
Application
  ↓
Application Framework / Ability Framework
  ↓
System Services / Subsystems
  ↓
Runtime / Distributed Capabilities / Basic Services
  ↓
Kernel / Driver Framework / Hardware Abstraction
```

Macaca 应映射为：

```text
Macaca Applications
  ↓
Application Framework / GenUI / WASM ABI / SDK
  ↓
System Services
  LLM / Task / Memory / Driver / Skill / MCP / Gateway / Store / Payment / Web3
  ↓
Agent OS Microkernel
  identity / scheduler / IPC / policy / trace / resource / session / task primitive
  ↓
Host Runtime
  process / workspace / sandbox / browser / filesystem / network / blockchain node
```

关键点：Application Framework 不等于 Web UI；System Services 不等于内核；Host Runtime 不等于业务逻辑。

### 4.2 系统能力服务化

OpenHarmony 的系统能力通过统一注册、发现、权限和 IPC 调用。Macaca 也需要系统能力服务化：

```text
Application / Agent
  ↓ request capability
Kernel Facade
  ↓ resolve service
Service Registry
  ↓ call via protocol
System Service
  ↓ emit trace / enforce policy / manage resource
```

所有服务都应具备统一生命周期：

```text
install -> register -> authorize -> start -> call -> trace -> update -> stop -> cleanup
```

### 4.3 驱动框架思想

OpenHarmony 的 HDF 屏蔽硬件差异。Macaca 的 `driver` 应屏蔽 AI 操纵现实软件的差异：

- Claude Code、OpenCode、shell、browser、Playwright、desktop app、IDE、office、terminal 都是 driver。
- 上层不应硬编码 driver 名称。
- driver 必须有 capability metadata、resource lock、trace event、status、cleanup。
- driver 可以由第三方通过 plugin 提供。

### 4.4 分布式软总线思想

OpenHarmony 的 DSoftBus 让跨设备通信成为系统能力。Macaca 的对应能力是：

- 本地 agent 与远程 agent 通信。
- 本地 service 与进程外 service 通信。
- MCP server、gateway、driver daemon、browser daemon、区块链节点都可在独立进程或远端运行。
- A2A 协作和交易需要可扩展 protocol adapter。

因此 `macaca-ipc` 不应只是内部 crate，而应成为服务调用平面。

### 4.5 可裁剪系统

OpenHarmony 面向不同设备形态裁剪系统能力。Macaca 也必须支持可裁剪：

- 不安装 Web3 模块也能完整使用普通 Agent OS。
- 不安装 EVM 模块也能运行普通 Application。
- 不安装某个 Gateway，也不影响 Web UI 或 CLI。
- 不安装某个 paid skill，不影响 open skill。
- Application 只声明需要的能力，系统按需加载。

## 5. Macaca OS 的目标能力模型

### 5.1 Application 不只是 YAML 配置

当前演示应用主要通过配置文件完成，这是合理的早期形态，但长期不应把 Application 限定为 YAML。

Macaca Application 应支持多种开发方式：

| 应用形态 | 说明 | 典型用途 |
| --- | --- | --- |
| YAML 配置包 | manifest、agents、tools、prompts、workflow | 快速声明式 agent application |
| WASM 二进制包 | 多语言编译到 WASM，遵循 ABI | 高度定制业务逻辑 |
| GUI Application | 自带 UI schema / component bundle / GenUI policy | 品牌化、有交互界面的应用 |
| Headless Application | 无 GUI，只作为后台服务或 automation | daemon、gateway-triggered app |
| Hybrid Application | YAML + WASM + GenUI + plugin dependencies | 成熟商业应用 |

这类似微信小程序、iOS App、Harmony App、Windows 软件：开发者遵循公开标准，就能在 Macaca OS 上运行。

### 5.2 WASM 作为应用二进制基础

WASM 适合作为 Macaca Application 的二进制基础，因为它天然支持多语言编译、沙箱隔离和 host capability 调用。

目标：

- Go / Rust / C++ / TypeScript / Python 等语言可以开发 application。
- Application 编译为 WASM component 或 WASI-compatible module。
- Application 通过 Macaca Host ABI 调用系统服务。
- 系统通过 capability-based permission 控制应用能访问什么。

概念示意：

```text
app.wasm
  imports:
    macaca:task/create_goal
    macaca:trace/emit
    macaca:llm/chat
    macaca:driver/call
    macaca:ui/render
  exports:
    app:init
    app:handle_event
    app:render
    app:shutdown
```

### 5.3 Application ABI

Macaca Application 需要公开稳定 ABI，而不是让第三方依赖内部 Rust crate。

最小 ABI 应包含：

| ABI | 职责 |
| --- | --- |
| lifecycle | init、start、pause、resume、shutdown、upgrade |
| event | handle user event、system event、agent event、payment event |
| capability | request service、check permission、declare dependency |
| task | create goal、query task、subscribe task state |
| trace | emit structured trace、subscribe trace |
| UI | render GenUI、handle UI action |
| storage | scoped kv、artifact、secret |
| network | controlled HTTP/WebSocket/MCP access |
| payment | subscription check、metered usage、A2A payment intent |

ABI 必须版本化：

```yaml
macaca_application:
  abi_version: "1.0"
  package_type: wasm_component
  min_os_version: "0.1.0"
```

### 5.4 GenUI 是基础能力，不是附属 UI 功能

当前 Macaca Web UI 以聊天和 trace 为主，这适合调试 Agent OS，但不适合作为所有应用的最终交互形态。未来 Application 需要自主定义 UI：

- 新闻写作应用可以是 newsroom dashboard。
- 研发应用可以是 IDE-like 工作台。
- 金融应用可以是 portfolio + risk console。
- Web3 应用可以是 wallet + transaction approval + agent deal room。
- 教育应用可以是课程界面。
- 企业自动化应用可以是 workflow cockpit。

GenUI 应作为系统能力：

```text
Application emits UI intent / UI schema / component tree
  ↓
Macaca UI Runtime validates permission and data binding
  ↓
Renderer displays application-specific interface
  ↓
User interaction returns structured UI event
  ↓
Application handles event via ABI
```

关键原则：

- Macaca 提供 UI runtime，不限制应用只能聊天。
- Application 可以完全定制品牌、设计语言、交互流程。
- 系统仍保留统一安全边界、权限提示、支付提示、trace overlay。
- GenUI 生成的界面必须可审计：用户点击、agent 建议、交易确认都进入 trace。

## 6. Plugin：第三方能力扩展机制

Macaca 不可能自己提供所有能力。操作系统生态必须允许第三方扩展系统能力。

### 6.1 Plugin 与 Application 的区别

| 类型 | 目的 | 面向对象 |
| --- | --- | --- |
| Application | 面向最终用户的软件 | 用户打开并使用 |
| Plugin | 扩展系统能力 | Application、Agent 或系统服务调用 |
| Skill | 面向 Agent 的可执行知识/流程能力 | Agent 调用 |
| MCP | 外部工具/资源协议接入 | Agent / Application 调用 |
| Driver | AI 操纵现实软件的抽象 | Agent / Application 调用 |
| Gateway | 外部入口接入 | 用户从外部平台进入 Macaca |

Plugin 可以提供：

- Telegram Gateway
- 飞书 Gateway
- 钉钉 Gateway
- WhatsApp Gateway
- 企业 SSO
- 第三方 Memory provider
- 第三方 Context Engine
- 第三方 Vector DB
- 第三方 Payment provider
- 第三方 Driver
- 第三方 Skill runtime
- 第三方 Compliance / Audit sink

### 6.2 Plugin Contract

Plugin 必须声明：

```yaml
plugin:
  id: com.vendor.telegram-gateway
  version: 1.0.0
  provides:
    - gateway.telegram
  requires:
    - macaca.gateway
    - macaca.trace
  permissions:
    - network.outbound
    - secret.read.telegram_token
  entry:
    type: wasm_component
    path: plugin.wasm
```

系统负责：

- 安装
- 签名验证
- 权限授权
- 生命周期管理
- 热加载/热卸载
- trace
- 资源清理
- 版本兼容检查

### 6.3 Gateway Plugin

Gateway plugin 对 Macaca 非常关键，因为用户不一定从 Web UI 进入系统。第三方可以让 Macaca 接入：

- Telegram
- 飞书
- 钉钉
- WhatsApp
- WeCom
- Discord
- Slack
- Email
- SMS
- 企业 IM

Gateway 的职责不是实现业务，而是把外部消息转成统一 Macaca Intent：

```text
external platform event
  ↓
gateway plugin
  ↓
Macaca Intent
  ↓
Application selection / session routing
  ↓
Agent OS execution
```

## 7. Store、订阅、加密与生态商业化

一个操作系统要形成成熟生态，开发者必须能赚钱。Macaca 需要官方商店和商业分发能力。

### 7.1 商店分发对象

Macaca Store 应支持：

- Application
- Skill
- MCP package
- Plugin
- Driver
- Gateway
- UI component pack
- Memory provider
- Web3 module

### 7.2 免费、开源、付费并存

Macaca 应同时支持：

- 明文开源 Skill / Application
- 免费闭源包
- 一次性购买包
- 订阅制包
- 用量计费包
- 企业授权包

### 7.3 Skill 加密与解密

Skill 很多时候是文字版能力，容易被复制。因此需要同时支持：

- 明文 skill：开源共享、可审计、可 fork。
- 加密 skill：付费订阅、授权后本地解密执行。

基本模型：

```text
encrypted skill package
  ↓ install from store
license token / subscription proof
  ↓
local decryption service
  ↓
runtime loads plaintext into protected execution context
  ↓
trace records usage, not necessarily泄露完整付费内容
```

注意：加密 skill 只能提高商业保护，不应假装能绝对防止所有本地逆向。真正防破解需要组合：

- 签名
- license check
- subscription validation
- remote attestation 可选
- usage metering
- legal + store policy
- 对高价值能力采用远程执行模式

### 7.4 付费 Application 与防绕过

付费 Application 应只能通过 Store 下载、授权和扣费。Macaca 需要：

- package signature
- license binding
- subscription entitlement
- offline grace period
- usage metering
- update channel
- revocation
- anti-tamper metadata
- developer payout

不应允许付费订阅应用绕过 Store 通过官网独立分发后仍使用 Macaca 的商业 entitlement。否则平台无法形成开发者收益闭环。

### 7.5 热安装与热更新

作为 OS，Macaca 不能每安装一个应用就重启。

需要支持：

- hot install
- hot uninstall
- hot update
- dependency resolution
- compatibility check
- running session migration
- rollback
- quarantine failed package

## 8. A2A 协作、支付与 Agent 交易能力

未来 Agent 与 Agent 之间不仅会协作，还会交易。一个 Agent 可以为另一个 Agent 提供付费服务，例如：

- 付费调研
- 付费代码审查
- 付费数据查询
- 付费模型推理
- 付费浏览器自动化
- 付费专业代理服务

### 8.1 A2A 基础协议

Macaca 需要支持本地 agent 和远程 agent 协作：

```text
local agent
  ↓ discover remote capability
remote agent/service
  ↓ quote price / terms
local agent
  ↓ request user or policy approval
payment escrow / smart contract / subscription
  ↓
remote execution
  ↓
result + proof + trace
```

### 8.2 A2A 交易能力

交易需要至少支持：

- agent identity
- capability discovery
- service quote
- terms negotiation
- payment intent
- escrow
- settlement
- dispute evidence
- reputation
- audit trail

Macaca 不应把某一个 A2A 标准写死到内核，而应提供 protocol adapter：

```text
macaca-a2a-core
  ├── local protocol
  ├── MCP-based protocol
  ├── HTTP signed intent
  ├── Ethereum smart contract adapter
  ├── future ERC/EIP adapter
  └── enterprise billing adapter
```

### 8.3 用户与策略控制

Agent 不能无约束花钱。必须支持：

- per app spending limit
- per agent spending limit
- per task budget
- user approval threshold
- allowlist / denylist
- risk scoring
- on-chain / off-chain receipt
- trace-visible payment event

## 9. Web3 Node：可选安装的系统模块

每台运行 Macaca OS 的电脑都可以选择成为区块链节点，但不能强制。

### 9.1 为什么必须可选

原因：

- 部分国家或地区对 crypto 不友好。
- 很多普通用户不需要 Web3。
- 节点会消耗磁盘、网络、CPU。
- 合规风险高，不应污染基础安装。

因此 Web3 Node 必须是 optional system module：

```text
Macaca Base OS
  works without web3

Macaca + Web3 Node Module
  enables wallet / transaction / smart contract / agent payment / DApp
```

### 9.2 模块能力

安装 Web3 Node 后提供：

- wallet service
- signing service
- transaction service
- chain index / query
- agent payment
- smart contract interaction
- on-chain identity
- token-gated app / skill entitlement
- DeFi / payment / settlement integration

### 9.3 合规与隔离

Web3 模块必须：

- 默认不安装。
- 默认不自动启用。
- 明确地区/合规提示。
- 权限独立授权。
- 私钥隔离存储。
- 所有签名需要 policy 或用户确认。
- Application 不能直接读取私钥。

## 10. EVM / DApp：基于 Substrate 的可选能力

Macaca 未来需要支持 AI + Web3 的 Application 开发。用户如果选择成为区块链节点，可以安装 EVM 模块，让 Macaca 支持 DApp。

### 10.1 为什么选择 Substrate + EVM

Substrate / Polkadot SDK 提供成熟的区块链开发框架，Frontier 等方案支持 EVM 兼容。Macaca 不应自己重造区块链和 EVM。

推荐方向：

```text
Macaca Web3 Module
  ↓
Substrate-based chain runtime
  ↓
EVM compatibility layer
  ↓
Macaca DApp / Agent Payment / Smart Contract
```

### 10.2 EVM 模块能力

EVM Module 提供：

- deploy contract
- call contract
- read contract state
- sign transaction
- subscribe event
- index contract event
- agent payment settlement
- token-gated entitlement
- DApp UI binding

### 10.3 与 Application 的关系

Application 可以声明：

```yaml
requires:
  optional_services:
    - web3.wallet
    - web3.evm
  capabilities:
    - contract.call
    - payment.agent_to_agent
```

如果用户未安装 Web3/EVM 模块：

- 普通功能仍可运行。
- Web3 功能显示为 unavailable。
- Application 可提示安装模块，但不能强制破坏基础使用。

## 11. 新的 Macaca OS 目标分层

### 11.1 微内核层

只保留最不可或缺原语：

| 原语 | 职责 |
| --- | --- |
| Identity | app、agent、session、task、service、package、developer identity |
| Scheduler | task / agent / service 调度 |
| IPC / Service Bus | 进程内、进程外、远程服务调用 |
| Capability Registry | service、plugin、driver、skill、mcp、gateway 注册发现 |
| Permission / Policy | 权限、预算、地域、合规、支付授权 |
| Trace / Audit Bus | 所有活动可追踪 |
| Resource Manager | workspace、browser、driver process、node、network、storage |
| Session / Task Primitive | session、goal、todo、review、resume、checkpoint |
| Package Runtime Guard | 安装、签名、授权、热加载基本守卫 |

### 11.2 系统服务层

可以内置，也可以第三方替换：

- LLM Service
- Memory / Context Service
- Task Service
- Trace Service
- Driver Service
- Skill Service
- MCP Service
- Gateway Service
- Store Service
- Payment Service
- Web3 Node Service
- EVM Service
- UI / GenUI Service

### 11.3 Application Framework 层

提供：

- Application SDK
- WASM ABI
- manifest schema
- GenUI runtime
- application lifecycle
- application permission declaration
- application package loader
- application event model
- application state / storage API

### 11.4 Application 层

第三方开发者构建：

- GUI app
- headless app
- agent app
- enterprise automation app
- AI + Web3 app
- paid subscription app
- marketplace skill bundle

## 12. Package 与 Manifest 设计方向

### 12.1 统一包格式

Macaca 应支持多种包：

| 包类型 | 扩展方向 |
| --- | --- |
| `.macaca-app` | Application 包 |
| `.macaca-skill` | Skill 包 |
| `.macaca-plugin` | Plugin 包 |
| `.macaca-mcp` | MCP 服务包 |
| `.macaca-driver` | Driver 包 |
| `.macaca-module` | Web3/EVM 等系统模块 |

这些可以只是逻辑包类型，底层可统一为 signed archive。

### 12.2 Manifest 示例

```yaml
package:
  id: com.example.newsroom
  type: application
  version: 1.2.0
  developer: did:macaca:example
  signature: required

runtime:
  kind: wasm_component
  abi_version: "1.0"
  entry: app.wasm

ui:
  kind: genui
  entry: ui.wasm
  permissions:
    - ui.render
    - ui.user_input

requires:
  services:
    - macaca.task
    - macaca.trace
    - macaca.llm
  optional_services:
    - macaca.web3.wallet
    - macaca.web3.evm
  capabilities:
    - browser.search
    - document.write

commerce:
  license: subscription
  store_required: true
  metering:
    - app.launch
    - premium.workflow

permissions:
  - network.outbound
  - storage.scoped
  - llm.invoke
```

## 13. Trace、权限和审计必须贯穿所有生态能力

Macaca 的核心竞争力之一是全程透明。无论是 Application、Plugin、Skill、Driver、MCP、Gateway、A2A Payment、Web3 Transaction，都必须进入 trace。

最小 trace context：

```text
app_id
package_id
developer_id
session_id
agent_id
task_id
service_id
capability
permission_scope
trace_id
event_seq
payment_intent_id optional
chain_tx_hash optional
```

原则：

- 无 trace 不执行。
- 无权限不调用。
- 无预算不交易。
- 无签名不安装。
- 无 entitlement 不运行付费能力。

## 14. 与当前 Macaca crate 的映射

| 目标层 | 当前 crate / 方向 | 调整建议 |
| --- | --- | --- |
| Microkernel | `macaca-kernel` | 收敛为 registry、scheduler、policy、trace、session/task primitive |
| IPC / Service Bus | `macaca-ipc` | 升级为服务调用平面，支持进程外与远程 agent |
| Protocol | `macaca-proto` | 承载 ABI、package、capability、trace、payment 基础类型 |
| Application Framework | `macaca-app`, `macaca-framework`, `macaca-sdk` | 支持 WASM ABI、GenUI、manifest、lifecycle |
| Task | `macaca-task` | 保持系统任务账本，不混入 app 特定 workflow |
| Driver | `macaca-driver` | 成为可插拔 driver framework |
| Skill | `macaca-skill` | 支持明文/加密、license、store entitlement |
| MCP | `macaca-runtime-host`, `macaca-framework` | 收敛为 MCP system service |
| Store / Commerce | 新服务或 `macaca-store` | 包管理、签名、订阅、扣费、开发者分成 |
| Web3 / EVM | 新模块 | optional module，基于 Substrate/EVM |
| Web UI | `macaca-web`, frontend | 降级为 Shell + Renderer，不定义 OS 核心语义 |
| Gateway | `macaca-gateway` | 支持 plugin-provided gateways |

## 15. 渐进式落地路线

### 阶段 1：定义 OS 边界与 Application 标准

产物：

- `Macaca Application ABI v0`
- `Macaca Package Manifest v0`
- `Kernel Primitive Boundary`
- `System Service Contract`

不做：

- 不立即实现商店。
- 不立即实现 EVM。
- 不立即重写所有 application。

### 阶段 2：把 Application 从配置包扩展为 package

目标：

- 现有 YAML app 继续可用。
- 新增 package manifest 概念。
- 支持配置包、WASM 包、Hybrid 包的统一描述。

### 阶段 3：GenUI Runtime v0

目标：

- Application 可以声明 UI schema / UI intent。
- Web UI 成为 Macaca Shell，而不是唯一聊天界面。
- trace overlay 与权限提示由系统统一提供。

### 阶段 4：Plugin Contract v0

目标：

- Gateway、Memory、Driver、Skill、MCP 都能通过 plugin manifest 注册。
- 支持热安装/热卸载的最小 lifecycle。

### 阶段 5：Store / Entitlement v0

目标：

- package signature
- license check
- subscription entitlement
- encrypted skill loading
- paid application install guard

### 阶段 6：A2A / Payment v0

目标：

- agent identity
- quote / intent / approval / receipt
- off-chain payment adapter
- future on-chain adapter extension point

### 阶段 7：Optional Web3 Node Module

目标：

- 用户选择安装。
- 不安装不影响基础系统。
- 安装后提供 wallet、signing、transaction、chain query。

### 阶段 8：Optional EVM / DApp Module

目标：

- 基于 Substrate/EVM 成熟方案。
- 支持 AI + Web3 application。
- 支持 agent-to-agent on-chain settlement。

## 16. 架构判断准则

以后新增能力时，用以下准则判断：

1. 如果能力是所有 application 都绕不开的基础机制，才可能进入 kernel。
2. 如果能力可以被第三方替换，必须是 system service 或 plugin。
3. 如果能力涉及外部平台，优先 gateway/plugin，不进核心。
4. 如果能力涉及 UI，必须支持 GenUI，不把 chat 当唯一界面。
5. 如果能力涉及二进制或第三方代码，必须走 package、signature、permission、sandbox。
6. 如果能力涉及付费，必须走 entitlement、metering、receipt、trace。
7. 如果能力涉及 agent 花钱，必须走 budget、approval、policy。
8. 如果能力涉及 crypto，必须是 optional module。
9. 如果能力涉及 EVM，基于 Substrate/EVM 生态，不自研底层链。
10. 如果能力未来可能跨进程或远程调用，不能只设计为 `Arc<T>` 直接调用。
11. 如果能力由 Application 定义业务逻辑，不能写入 Macaca 核心。
12. 如果能力是平台治理能力，必须有审计与回滚。

## 17. 结论

OpenHarmony 给 Macaca 的最大启发不是某个具体内核实现，而是“操作系统式边界”：

- 内核小而稳定。
- 系统能力服务化。
- 应用通过公开标准运行。
- 驱动、网关、插件、技能、MCP 都可扩展。
- 商店、签名、授权、订阅、防破解支撑商业生态。
- GenUI 让软件拥有自由 UI，而不是被聊天界面绑定。
- Web3、EVM、A2A 支付作为可选系统模块进入生态，而不是污染基础系统。

因此 Macaca 的长期目标应明确为：

> 一个微内核 Agent OS，提供统一身份、调度、IPC、权限、trace、资源和 package runtime；通过系统服务和插件承载 LLM、Memory、Driver、Skill、MCP、Gateway、Store、Payment、Web3、EVM；通过 WASM Application ABI 和 GenUI 支持第三方开发者构建 GUI 或 headless 软件，并通过商店和订阅形成可持续生态。

后续重构必须服务这个目标：把 `macaca-web` 和 `macaca-kernel` 中不属于核心原语的能力逐步服务化，把 Application 从 demo 配置升级为标准化软件包，把 Plugin/Store/GenUI/Web3 作为架构一等公民预留，而不是后期外挂。
