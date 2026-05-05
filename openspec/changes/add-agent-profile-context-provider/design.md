# Design: Agent Profile Context Provider

## Context

OpenClaw 的 workspace bootstrap 文件提供了清晰的 agent 行为注入模型。Macaca 需要类似能力，但必须作为可插拔 provider，而不是 runtime 固定读取一组硬编码文件。文件名可以是默认约定，扫描 root、启用规则、优先级和预算都必须可配置。

## Goals / Non-Goals

Goals:

- 将 agent profile 文件作为 context candidates 注入。
- 保留固定文件名约定，但避免 application/business 硬编码。
- 文件加载安全、可预算、可诊断。
- 支持未来用户替换 profile source 或 policy。

Non-Goals:

- 不实现 profile 文件编辑 UI。
- 不把 `MEMORY.md` 自动写入 vector memory。
- 不把 `HEARTBEAT.md` 行为直接绑定到 scheduler。
- 不删除现有 system prompt/persona 字段。

## Decisions

### Decision 1: Profile provider 是 Adapter

`ProfileFileContextProvider` 把文件系统中的 profile 文件适配为 `ContextCandidate`。它不直接渲染最终 prompt，也不决定 stable prefix 之外的全局布局。

### Decision 2: 安全加载使用 Template Method

共享加载流程：

1. 解析 root 和候选文件。
2. realpath 校验，禁止越界。
3. 文件大小和编码检查。
4. 根据 file kind 应用 policy。
5. 生成 candidate 和 diagnostics。

各文件类型只覆盖优先级、默认 cache class、target 和预算。

### Decision 3: 优先级和注入策略使用 Strategy

默认策略遵循：

- 高：`AGENTS.md`、`SOUL.md`
- 中：`TOOLS.md`、`IDENTITY.md`
- 低：`USER.md`
- 动态/低：`HEARTBEAT.md`
- 种子/审计：`MEMORY.md`

用户可替换策略，但不能绕过安全加载和 report。

### Decision 4: MEMORY.md 不等同长期记忆

`MEMORY.md` 是 agent 对记忆系统的说明、种子或治理入口。长期向量记忆由 memory facade/provider 管理，profile provider 不直接写 collection。

## Risks / Trade-offs

- Risk: profile 文件过长导致 prompt 膨胀。Mitigation: per-kind budget 和截断决策写入 report。
- Risk: workspace 文件包含 prompt injection。Mitigation: profile root 可信度可配置，默认文件仍携带 source/trust metadata。
- Risk: `HEARTBEAT.md` 被错误注入普通请求。Mitigation: 默认低优先级并可按 heartbeat 阶段 target。
- Risk: 文件名约定被视为硬编码。Mitigation: 固定名称只是默认 provider 约定，provider/policy 可替换。

## Migration Plan

1. 在 context composer 基座上实现 profile provider。
2. 将现有 persona/profile prompt 来源映射为 candidates。
3. 保留旧 helper 并标记 deprecated。
4. 增加 report/API 诊断，显示 profile 文件是否被注入或跳过。
