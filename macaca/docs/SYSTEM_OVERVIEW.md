# Macaca System Overview

> **Canonical system-definition document.** 这份文档回答：Macaca 是什么、解决什么问题、核心模块如何协作，以及任务如何在系统里执行。
>
> 当前实现的具体问题、优先级风险与重构行动请看 [`SYSTEM_AUDIT.md`](./SYSTEM_AUDIT.md)。
>
> 更长篇、混合了历史推演与架构草稿的材料保留在 [`../ARCHITECTURE-v2.md`](../ARCHITECTURE-v2.md) 中，但它不是 canonical 来源。

<a id="what-macaca-is"></a>
## 1. What Macaca Is

Macaca 是一个 **Agent 操作系统（Agent OS）**。它不是单一 Agent，也不是单个工作流脚本，而是一套用于：
- 发现与运行 Application
- 注册、调度与协调 Agent
- 将 Driver / Tool 能力统一暴露给 Agent
- 通过 Web / Gateway / 持久化 / 审计能力把执行链路运行起来并留下证据

更准确地说，它的目标形态不是“等待用户来点一下按钮”的被动式聊天系统，而是一个 **7×24 小时持续运行的全自动 Agent 执行系统**：系统应能围绕目标自主规划、自主执行、在需要时自我唤醒，并只在真正必要时才把人类拉回环路。

类比关系：
- Linux 管理进程 → Macaca 管理 Agent
- Linux 运行应用程序 → Macaca 运行 Application
- Linux 暴露系统调用 → Macaca 暴露 Agent 能力接口与工具系统

<a id="what-problem-it-solves"></a>
## 2. What Problem It Solves

Macaca 试图解决的是：**如何把多个 Agent、工具、驱动、工作流和状态追踪整合成一个可以长期运行、可扩展、可审计、可被真实用户使用的系统底座**。

它要避免的不是“Agent 能不能调用一次工具”，而是以下系统性问题：
- Agent 只能做一次性对话，无法长期运行和协调
- Agent 缺少持续运行、自我唤醒和主动推进任务的机制，只能等人类反复触发
- Application 的入口、能力和工作流缺少统一运行时
- 工具、驱动、Gateway、记忆、审计等能力彼此割裂
- 执行链路没有清晰状态、事件与审计证据
- 系统目标不清时，重构只能围绕局部代码味道而不是围绕系统原则进行

<a id="target-system-qualities"></a>
## 3. Target System Qualities

Macaca 的目标质量不是“能跑 demo”即可，而是尽量朝着一个真正可长期演化的 Agent OS 靠拢。目标质量包括：

- **可维护性**：避免巨型文件、God Object、重复类型、重复执行逻辑、散落硬编码
- **自主性 / 主动性**：系统应能 7×24 小时围绕目标自动规划、自动执行、自我唤醒，而不是主要依赖人类轮询式推动
- **可扩展性**：Application、Driver、Gateway、LLM Provider、记忆层都应可插拔
- **可观测性**：任务执行、状态变化、工具调用、审计事件都可追踪
- **可配置性**：入口 Agent、Workflow、能力边界应尽量从 manifest / config / app 定义读取，而不是写死在源码里
- **执行一致性**：无论是即时委派还是目标级执行，系统都应保留一致的任务/状态/审计语义
- **真实使用导向**：不是玩具项目，而是面向真实任务和真实使用体验的底座

<a id="core-design-principles"></a>
## 4. Core Design Principles

### <a id="principle-1-bounded-module-responsibility"></a>Principle 1 — Bounded module responsibility
模块边界必须清晰：路由层不应吞掉业务编排，状态容器不应变成 God Object，单个文件不应承载跨层职责。

### <a id="principle-2-config-driven-entry-and-orchestration"></a>Principle 2 — Config-driven entry and orchestration
入口 Agent、编排入口和工作流应尽量由 app manifest / config 决定，而不是依赖硬编码的 `coordinator` 假设。

### <a id="principle-3-observable-end-to-end-execution"></a>Principle 3 — Observable end-to-end execution
从用户请求进入系统，到 Agent 调度、工具调用、任务拆解、状态更新、事件/审计写入，整条链路都应可追踪。

### <a id="principle-4-shared-protocol-and-task-primitives"></a>Principle 4 — Shared protocol and task primitives
跨 crate 的核心类型、任务原语和执行语义应尽量共享，而不是重复定义、重复实现。

### <a id="principle-5-pluggable-capabilities-and-platform-surfaces"></a>Principle 5 — Pluggable capabilities and platform surfaces
Driver、Gateway、记忆、MCP、LLM Provider 等都应作为能力面存在，可以接入、替换、扩展，而不是被固定在单一路径里。

<a id="module-map"></a>
## 5. Module Map

| 模块 | 系统角色 | 核心职责 |
|---|---|---|
| `macaca-web` | 入口与运行承载层 | 提供 HTTP / SSE / chat / goals API，桥接 session、事件与执行生命周期 |
| `macaca-kernel` | Agent 内核 | 注册 Agent、执行 Agent、管理 fork / executor / 审计 / 告警 |
| `macaca-runtime` | Agentic 运行时 | 驱动 AgenticLoop、LLM 调用、工具执行、循环控制 |
| `macaca-task` | 任务系统 | TodoBoard、PlanLoop、WorkerLoop、目标拆解与 review 流程 |
| `macaca-app` | Application 运行时 | 发现应用、加载 manifest/persona/workflow、启动应用 |
| `macaca-tools` | 工具系统 | 聚合内置工具、编排工具与任务工具 |
| `macaca-driver` | 驱动层 | 把 shell、文件系统、Claude Code 等能力暴露为可调用接口 |
| `macaca-llm` | 模型抽象层 | 对接多 provider、路由、降级、成本/速率控制 |
| `macaca-persist` | 持久化层 | Redb KV、会话存储、EventLog 等持久化能力 |
| `macaca-proto` | 协议与共享类型 | 统一配置、错误、共享类型、编排协议 |
| `macaca-gateway` | 外部入口扩展 | Telegram / Discord / 其他 IM 的适配入口 |
| `macaca-memory` / `macaca-ipc` / `macaca-mcp` | 可插拔能力层 | 分别承载记忆、进程间通信、MCP 协议支持 |

<a id="task-execution-chain"></a>
## 6. Task Execution Chain

### Current — 当前系统里已经存在的执行路径

Macaca 当前至少有两条可见执行路径：

1. **即时委派路径**  
   `User -> post_chat -> Coordinator AgenticLoop -> delegate_task -> Fork-Join -> Worker AgenticLoop`

2. **目标级任务路径**  
   `User -> create_goal -> PlanLoop -> TodoStore -> WorkerLoop claim -> AgenticLoop execute -> PlanLoop review`

这两条路径说明系统已经不只是“调用一次 LLM”，而是在尝试形成：**用户请求 → 协调/拆解 → 执行 → review / resume / persistence** 的闭环。

### Intended — 目标中的一致执行语义

系统的目标不是让 chat 绕过 Agent，而是让请求进入统一的 Agent / Workflow 语义中：
- 用户请求进入入口 Agent
- 入口 Agent 负责判断直接回复还是触发工作流/任务系统
- 系统在目标尚未完成时应能依据事件、任务状态、调度条件或恢复信号继续推进，而不是默认退化成“等用户下一条消息”
- Agent 执行中产生状态更新、工具调用、事件和审计证据
- 结果通过 SSE / Web / Gateway 等界面对外暴露

### Planned — 已被识别但未完全落地的能力

以下能力已经在架构材料中被明确，但尚未完整融入主执行路径：
- 更完整的 WorkflowEngine 接入
- 记忆系统接入 Agent 执行路径
- Gateway 与 MCP 的系统级整合
- 更统一的任务/状态/事件语义

<a id="current-intended-planned-boundaries"></a>
## 7. Current vs Intended vs Planned Boundaries

| 类别 | 含义 | 在这份文档中的写法要求 |
|---|---|---|
| **Current** | 当前代码库里已经存在或已实现的事实 | 直接描述现状，可由代码或当前文档证明 |
| **Intended** | 系统希望长期保持的设计原则或执行语义 | 必须写清是设计目标/系统约束，而不是假装已完成 |
| **Planned** | 已识别但尚未完整接入/尚待重构的部分 | 必须明确标成 planned，不得混写成 current |

这条边界很重要，因为 `ARCHITECTURE-v2.md` 和旧版审计文档里混杂了 current / intended / planned 三种表达，容易让后续重构误把“目标”当“现状”。

<a id="deeper-references-and-audit"></a>
## 8. Links to Deeper References and Audit

- 当前实现审计与重构依据：[`SYSTEM_AUDIT.md`](./SYSTEM_AUDIT.md)
- Route C 微内核边界治理：[`agent-os-microkernel-boundaries.md`](./agent-os-microkernel-boundaries.md)
- Route C 回归矩阵：[`route-c-regression-matrix.md`](./route-c-regression-matrix.md)
- Route C 阶段实施模板：[`route-c-phase-template.md`](./route-c-phase-template.md)
- Route C 架构治理规则：[`route-c-architecture-governance.md`](./route-c-architecture-governance.md)
- 非 canonical 深度参考：[`../ARCHITECTURE-v2.md`](../ARCHITECTURE-v2.md)
- 已批准的澄清计划：[`../../.omx/plans/prd-system-audit-clarification.md`](../../.omx/plans/prd-system-audit-clarification.md)
- 验证矩阵：[`../../.omx/plans/test-spec-system-audit-clarification.md`](../../.omx/plans/test-spec-system-audit-clarification.md)

<a id="ecosystem-hardening"></a>
## 9. Ecosystem Hardening

Route C Phase 13 treats third-party development as an operating-system concern.
Applications, plugins, skills, MCP packages, GenUI surfaces, Store-submitted
packages, optional Web3 packages, and optional EVM/DApp packages must be
developable, packageable, certifiable, traceable, and debuggable without
modifying Macaca source code.

Developer guides:

- Application development: [`developer/application-development-guide.md`](./developer/application-development-guide.md)
- Plugin development: [`developer/plugin-development-guide.md`](./developer/plugin-development-guide.md)
- GenUI development: [`developer/genui-development-guide.md`](./developer/genui-development-guide.md)
- Store submission: [`developer/store-submission-guide.md`](./developer/store-submission-guide.md)
- Web3 and DApp development: [`developer/web3-dapp-development-guide.md`](./developer/web3-dapp-development-guide.md)

All ecosystem packages must pass compatibility certification before they are
considered installable. The checker reports `compatible`,
`compatible_with_warnings`, or `incompatible` with stable diagnostic codes,
trace/audit events, and actionable field paths. Optional Web3/EVM modules remain
optional; unavailable optional modules must degrade with structured warnings and
must not break normal applications.
