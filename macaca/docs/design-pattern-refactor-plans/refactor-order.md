# Macaca Agent OS 渐进式重构顺序表

本文档基于两类事实来安排重构顺序：

1. [README.md](./README.md) 中定义的总体约束
2. workspace 内各 crate 的直接依赖关系与主要消费关系

目标不是给出“唯一正确”的理论拓扑排序，而是给出一份适合实际落地的、低风险的、可渐进迁移的执行顺序。核心原则是：

- 先重构基础抽象，再迁移主要消费方。
- 先收敛被多个 crate 复用的 contract，再收敛最外层编排与交付入口。
- 每一阶段都必须能独立编译、测试、回滚，不能把多个高风险层级绑在同一轮里一起改。

## 1. 直接依赖关系摘要

以下只列 workspace 内部依赖，用于说明顺序，不展开第三方库：

| crate | 直接依赖 |
| --- | --- |
| `macaca-proto` | - |
| `macaca-llm` | `macaca-proto` |
| `macaca-memory` | `macaca-proto` |
| `macaca-persist` | `macaca-proto` |
| `macaca-ipc` | `macaca-proto` |
| `macaca-task` | `macaca-llm`, `macaca-persist`, `macaca-proto` |
| `macaca-tools` | `macaca-persist`, `macaca-proto`, `macaca-task` |
| `macaca-driver` | `macaca-proto`, `macaca-tools` |
| `macaca-skill` | `macaca-proto`, `macaca-tools` |
| `macaca-agent` | `macaca-llm`, `macaca-proto`, `macaca-tools` |
| `macaca-framework` | `macaca-agent`, `macaca-llm`, `macaca-persist`, `macaca-proto`, `macaca-tools` |
| `macaca-runtime` | `macaca-llm`, `macaca-proto`, `macaca-tools` |
| `macaca-runtime-host` | `macaca-framework`, `macaca-proto`, `macaca-skill` |
| `macaca-kernel` | `macaca-agent`, `macaca-ipc`, `macaca-llm`, `macaca-memory`, `macaca-persist`, `macaca-proto`, `macaca-sdk`, `macaca-task`, `macaca-tools` |
| `macaca-sdk` | `macaca-agent`, `macaca-kernel`, `macaca-llm`, `macaca-proto`, `macaca-tools` |
| `macaca-app` | `macaca-agent`, `macaca-kernel`, `macaca-llm`, `macaca-proto`, `macaca-sdk`, `macaca-tools` |
| `macaca-gateway` | `macaca-proto` |
| `macaca-web` | `macaca-agent`, `macaca-app`, `macaca-driver`, `macaca-framework`, `macaca-kernel`, `macaca-llm`, `macaca-persist`, `macaca-proto`, `macaca-runtime`, `macaca-runtime-host`, `macaca-sdk`, `macaca-skill`, `macaca-task`, `macaca-tools` |
| `macaca-cli` | `macaca-agent`, `macaca-app`, `macaca-gateway`, `macaca-kernel`, `macaca-llm`, `macaca-proto`, `macaca-tools`, `macaca-web` |
| `macaca-integration-tests` | 几乎依赖全部核心 crate，仅作为验收层 |

## 2. 主要消费关系摘要

仅看 Cargo 依赖还不够，真正决定迁移顺序的是“谁定义抽象，谁消费抽象”。

| 生产者 / 被消费方 | 主要向谁提供能力 | 为什么必须先稳定 |
| --- | --- | --- |
| `macaca-proto` | 全部 crate | 定义消息、事件、任务、trace 基本类型，任何上层重构都要复用它 |
| `macaca-llm` | `task`、`agent`、`kernel`、`runtime`、`web` | provider/model/router 是多处横切能力，抽象不稳定会放大后续迁移成本 |
| `macaca-persist` | `task`、`tools`、`kernel`、`framework`、`web` | session、event、checkpoint、task board 都依赖持久化 contract |
| `macaca-task` | `tools`、`kernel`、`web` | TodoBoard / PlanLoop / ReviewLoop 是自主运行主链路的底座 |
| `macaca-tools` | `agent`、`driver`、`skill`、`framework`、`web` | tool middleware、policy、driver tool、skill tool 都从这里展开 |
| `macaca-agent` | `framework`、`kernel`、`sdk`、`app`、`web`、`cli` | agent 生命周期、builder、capability 是上层构建入口的核心抽象 |
| `macaca-framework` | `runtime-host`、`web`，未来也服务 `cli` / `gateway` | traced agent construction、MCP runtime、tool bridge 的统一入口 |
| `macaca-app` | `web`、`cli`、部分 `task`/`framework` 消费语义 | application manifest、workflow prompt、tool policy、entry agent 语义源头 |
| `macaca-kernel` | `sdk`、`app`、`web`、`cli` | loop、executor、session、resume 协调中枢，变更必须建立在底层抽象稳定之后 |
| `macaca-web` / `macaca-cli` | 用户最终入口 | 必须最后收口，否则前面每层变化都会反复穿透到入口层 |

## 3. 推荐重构总顺序

### 阶段 0：基线与约束

适用 crate：

- `macaca-integration-tests`
- 文档与 OpenSpec 本身

目标：

- 为每个核心 crate 落盘设计模式重构方案
- 为每个实际实施切片准备 OpenSpec proposal
- 保持现有端到端测试任务可跑，作为后续渐进重构的回归基线

说明：

- 这一阶段不是先改业务代码，而是先把“怎么安全改”固定住。
- 当前这一步已经在进行中，`docs/design-pattern-refactor-plans/` 就属于这个阶段的产物。
- Route C 阶段 0 的治理产物已经独立落盘：
  - [`../agent-os-microkernel-boundaries.md`](../agent-os-microkernel-boundaries.md)
  - [`../route-c-regression-matrix.md`](../route-c-regression-matrix.md)
  - [`../route-c-phase-template.md`](../route-c-phase-template.md)
  - [`../route-c-architecture-governance.md`](../route-c-architecture-governance.md)

### 阶段 1：最底层稳定 contract

顺序：

1. `macaca-proto`
2. `macaca-persist`
3. `macaca-memory`
4. `macaca-ipc`
5. `macaca-llm`
6. `macaca-gateway`

为什么先做这一组：

- 这些 crate 几乎不消费上层业务语义，却被大量上层 crate 依赖。
- 这里适合优先引入 `Adapter`、`Facade`、`Strategy`、`Proxy`、`Visitor`、`Builder` 等模式，把协议层和 provider 层先变成稳定原语。
- 如果这一层不先稳住，后面 `task`、`agent`、`framework` 的抽象会被迫反复返工。

进入条件：

- 现有类型、事件、存储和 provider contract 已经能被测试覆盖

退出条件：

- 低层 API 语义清晰，向上暴露的是稳定 contract，而不是零散 helper 或硬编码 if/else

### 阶段 2：自主运行基础设施层

顺序：

1. `macaca-task`
2. `macaca-tools`
3. `macaca-driver`
4. `macaca-skill`
5. `macaca-runtime`

为什么这样排：

- `task` 先于 `tools`，因为 tool policy、driver tool、skill tool 都会读写 task / review / todo 语义。
- `tools` 先于 `driver` / `skill`，因为 driver 和 skill 本质上都在向工具系统注册能力。
- `runtime` 放在这组后段，因为它消费工具和 LLM，但不应该反过来定义它们的 contract。

重点模式：

- `macaca-task`：`Mediator`、`State`、`Command`、`Memento`
- `macaca-tools`：`Chain of Responsibility`、`Decorator`、`Command`
- `macaca-driver`：`Bridge`、`Adapter`、`Factory`
- `macaca-skill`：`Composite`、`Factory`、`Facade`
- `macaca-runtime`：`Template Method`、`Observer`

退出条件：

- TodoBoard / PlanLoop / ReviewLoop / Tool middleware / Driver runtime / Skill runtime 的 contract 已稳定
- skill、driver、tool、task 的组合不再要求上层入口写专有逻辑

### 阶段 3：Agent 与 Framework 原语层

顺序：

1. `macaca-agent`
2. `macaca-framework`
3. `macaca-runtime-host`

为什么这样排：

- `macaca-agent` 定义 agent lifecycle、builder、capability、services，是框架层之上的直接基础。
- `macaca-framework` 负责把 agent、tool、trace、MCP、runtime 组装成可复用原语，所以必须建立在 agent 层重构完成之后。
- `macaca-runtime-host` 是 runtime + framework + skill 的宿主封装，应当最后接住前两者的稳定抽象。

当前状态：

- `macaca-agent` 已完成第一轮基于设计模式的核心重构，并已提交
- `macaca-framework` 的 agent construction 正在向新 `macaca-agent` 抽象迁移
- `runtime-host` 尚未系统迁移，应排在 `framework` 收口之后

重点模式：

- `macaca-agent`：`Builder`、`Facade`、`Null Object`、`State`、`Composite`
- `macaca-framework`：`Abstract Factory`、`Decorator`、`Bridge`、`Facade`
- `macaca-runtime-host`：`Facade`、`Bridge`、`Factory`

退出条件：

- traced agent construction、MCP runtime、skill-backed runtime 统一走 framework primitive
- 上层不再手工拼装 agent/tool/trace hook/runtime glue

### 阶段 4：应用语义与系统协调层

顺序：

1. `macaca-kernel`
2. `macaca-sdk`
3. `macaca-app`

为什么不是先做 `app`：

- `app` 虽然是应用语义源头，但它直接依赖 `kernel` 和 `sdk`。
- 如果 `kernel` 中 session / executor / loops / resume 的协调面还没有收敛，`app` 很容易把不稳定的系统细节再次包装并扩散给 `web` 和 `cli`。
- `sdk` 是连接 kernel 与 app 的中间层，适合在 kernel 稳定后先做一层 adapter/facade 收口，再让 `app` 基于新接口重构。

当前状态：

- `macaca-app` 已完成一轮设计模式重构
- `macaca-web` 等主要消费方正在迁移到新的 `macaca-app` 抽象
- `kernel` / `sdk` 还没有完成同等级别的系统化渐进重构，因此整个应用语义链条尚未完全闭环

重点模式：

- `macaca-kernel`：`Mediator`、`Observer`、`State`、`Memento`
- `macaca-sdk`：`Facade`、`Adapter`
- `macaca-app`：`Factory`、`Builder`、`Strategy`、`Composite`

退出条件：

- session / trace / resume / executor / app runtime / workflow prompt / entry agent / tool policy 都有明确的 contract 边界
- 上层入口不再自行理解 application manifest 细节

### 阶段 5：最终交付入口层

顺序：

1. `macaca-web`
2. `macaca-cli`

为什么最后做：

- 这两个 crate 依赖最广，最容易把底层不稳定抽象重新“拉平”成入口层硬编码。
- 只有当前面几层 contract 基本稳定后，web/cli 才能真正变成薄入口，而不是继续承载系统核心逻辑。

当前状态：

- `macaca-web` 已经开始迁移到 `macaca-app`、`macaca-framework`、`macaca-agent` 的新抽象上
- 但它仍然是当前最重、最复杂、风险最高的消费方
- `macaca-cli` 适合在 `web` 迁移经验稳定后再做相同思路的瘦身

重点模式：

- `Facade`
- `Adapter`
- `Decorator`
- `Observer`

退出条件：

- web/cli 只负责入口适配、参数转换、结果展示
- 主业务编排不再长期停留在入口层

### 阶段 6：全链路迁移与验收

顺序：

1. `macaca-integration-tests`
2. 全仓联调与回归

目标：

- 用 integration tests 和真实 session 流验证“重构后行为 1:1 保持”
- 覆盖 session trace、resume、goal create、plan decomposition、worker claim、review、coordinator resume、driver、skill、MCP、SSE/EventLog 恢复链路

## 4. 推荐执行表

| 阶段 | 先后顺序 | crate | 角色定位 | 推荐动作 |
| --- | --- | --- | --- | --- |
| 0 | 0-1 | `macaca-integration-tests` | 回归基线 | 先补足测试场景，再开始大规模迁移 |
| 1 | 1-1 | `macaca-proto` | 全局类型底座 | 先稳事件/消息/任务/trace contract |
| 1 | 1-2 | `macaca-persist` | 持久化底座 | 抽象 event/session/checkpoint/review 存储 contract |
| 1 | 1-3 | `macaca-memory` | 记忆底座 | 收敛 memory provider 和存取 facade |
| 1 | 1-4 | `macaca-ipc` | 进程通信底座 | 解耦 transport 与 message contract |
| 1 | 1-5 | `macaca-llm` | 模型/provider 底座 | 统一 provider/model/router/strategy |
| 1 | 1-6 | `macaca-gateway` | 外围协议入口 | 收敛 API facade，不反向污染内核 |
| 2 | 2-1 | `macaca-task` | 自主运行任务账本 | 先稳 TodoBoard / PlanLoop / ReviewLoop contract |
| 2 | 2-2 | `macaca-tools` | 工具执行总线 | 收敛 tool middleware / policy / command |
| 2 | 2-3 | `macaca-driver` | 外部执行驱动 | 解耦 driver ABI / runtime / trace bridge |
| 2 | 2-4 | `macaca-skill` | skill runtime | 标准化 skill manifest / tool export / runtime bridge |
| 2 | 2-5 | `macaca-runtime` | 通用执行循环 | 收敛 agentic loop 模板方法 |
| 3 | 3-1 | `macaca-agent` | Agent 原语 | 已完成第一轮重构，继续作为迁移基座 |
| 3 | 3-2 | `macaca-framework` | 组装与追踪原语 | 统一 traced build / MCP / runtime primitive |
| 3 | 3-3 | `macaca-runtime-host` | 通用宿主层 | 将 skill/runtime glue 收到 facade 后面 |
| 4 | 4-1 | `macaca-kernel` | 系统协调中枢 | 收敛 executor/session/loop/resume 协调面 |
| 4 | 4-2 | `macaca-sdk` | kernel 对外适配层 | 为 app/web/cli 提供稳定 facade |
| 4 | 4-3 | `macaca-app` | 应用语义层 | 已完成第一轮重构，继续迁移主要消费方 |
| 5 | 5-1 | `macaca-web` | 主交付入口 | 最后变薄，迁走剩余 orchestration/hardcode |
| 5 | 5-2 | `macaca-cli` | 次交付入口 | 沿用 web 的迁移模式做薄入口 |
| 6 | 6-1 | `macaca-integration-tests` | 全链路验收 | 统一验证所有迁移行为未退化 |

## 5. 已完成与当前推荐下一步

### 已完成或已启动

- `macaca-agent`
  - 已完成核心设计模式重构
  - 已提交 git commit
- `macaca-app`
  - 已完成核心设计模式重构
  - 主要消费方迁移已启动
- `macaca-framework`
  - 正在向新的 agent construction primitive 迁移
- `macaca-web`
  - 正在迁移到 `macaca-app` / `macaca-framework` / `macaca-agent` 新抽象

### 当前最合理的总顺序建议

从“尽量减少返工”和“让既有工作形成闭环”的角度，建议后续优先级如下：

1. 先把 `macaca-framework` 收口完成
2. 再补 `macaca-runtime-host`
3. 然后系统化推进 `macaca-kernel`
4. 再做 `macaca-sdk`
5. 再继续推进 `macaca-app` 消费方迁移闭环
6. 最后收口 `macaca-web` 和 `macaca-cli`

原因：

- `macaca-agent` 已经完成，最应该接住它的是 `macaca-framework`
- `macaca-app` 已经重构，如果 `framework` / `kernel` / `sdk` 不跟上，`web` 里仍会保留大量胶水代码
- `web` 现在虽然已经在迁，但它本质上应该是“最后变薄”的对象，而不应继续成为长期承载核心编排逻辑的地方

## 6. 使用方式

后续每做一个 crate 的渐进式重构或迁移，都建议先回答四个问题：

1. 这个 crate 在上表里属于哪个阶段
2. 它依赖的下层 crate 是否已经基本稳定
3. 它的主要消费方是否已经准备好迁移
4. 本轮是“重构生产者”还是“迁移消费者”

只有这四个问题都回答清楚，切片才容易做小、做稳、做完。
