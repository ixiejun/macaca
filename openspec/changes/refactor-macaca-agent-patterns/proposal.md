# Change: 渐进式重构 macaca-agent 核心抽象

## Why

`macaca-agent` 是 Agent OS 的基础接口层，当前承担 Agent trait、基础 Agent、Agent 服务集合、状态机和 capability 描述等核心职责。随着 traced agent、driver、skill、MCP、memory、event sink 等能力持续接入，当前实现存在几个可预见的扩展压力：

- `AgentServices` 以多个可选服务聚合为主，调用侧容易出现重复 `Option` 判断，缺省行为不够统一。
- `BasicAgent` 构造会随着服务和 capability 增长而继续膨胀，容易演化成长参数构造或散落初始化。
- `AgentStateMachine` 的状态转移主要依赖枚举和 match，后续 pause/resume/fail/retry 等状态语义增加后会难以审计。
- Agent capability 未来会来自 persona、manifest、skill、driver、MCP、tool policy 等多个来源，简单字符串/列表合并会降低可追踪性。

本 change 依据：

- `macaca/docs/design-pattern-refactor-plans/README.md` 的全局渐进式重构约束
- `macaca/docs/design-pattern-refactor-plans/macaca-agent.md` 的 crate 级计划

目标是在行为 1:1 还原的前提下，把 `macaca-agent` 的核心抽象逐步收敛到更稳定的设计模式结构。

## What Changes

- 为 `AgentServices` 增加只读 Facade 访问方法，先保持内部字段与旧调用兼容。
- 引入 Null Object 缺省服务，例如 no-op event sink / memory service，减少调用侧 `Option` 分支。
- 增加 `BasicAgentBuilder`，旧 `BasicAgent::new` 保留并委托给 builder，避免一次性破坏调用侧。
- 为 `AgentStateMachine` 增加状态转移黄金测试，再抽出 `AgentLifecyclePolicy`，保证状态语义可审计。
- 为 Agent capability 引入可组合的 capability 表达，先保持现有对外 capability 输出不变。

## Non-Goals

- 不改变 `Agent` trait 的外部行为语义。
- 不改变 coordinator、planner、worker 的执行链路。
- 不改变 traced agent 构建入口。
- 不改变 SSE、EventLog、run_trace、AgentExecutionEvent 的 event 名称或 payload schema。
- 不在本 change 中迁移 `macaca-framework`、`macaca-web`、`macaca-task` 的 agent 构建逻辑。
- 不一次性删除旧构造函数；旧 API 只标记迁移方向并委托到新实现。
- 不改变 application manifest、allowed_tools、skill、driver 或 MCP 的配置语义。

## Impact

- Affected specs: `macaca-agent-core`
- Affected code:
  - `macaca/crates/macaca-agent/src/**`
  - 可能涉及调用侧测试 fixture，但不应要求业务 crate 改语义
- Expected risk: Medium
- Risk reason:
  - `macaca-agent` 是基础 crate，调用面广。
  - 但本 change 采用 additive-first 策略，先增加 facade/builder/policy，再逐步内部委托，避免破坏外部调用。
- Behavioral compatibility:
  - Agent execute/status/capability 行为保持不变。
  - 缺省服务从“没有服务”变成 no-op 服务时，不应产生额外副作用。
  - 状态转移结果必须与现有实现完全一致。
  - capability 对外展示必须与现有输出兼容。

## Rollout Strategy

本 change 必须按小切片推进：

1. 先补测试锁定现有行为。
2. 再添加新抽象，但不删除旧路径。
3. 让旧路径委托到新抽象。
4. 最后只在确认所有调用侧兼容后，逐步替换内部调用点。

每个切片都必须可以单独编译、单独回滚。

