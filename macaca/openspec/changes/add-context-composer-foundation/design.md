# Design: Context Composer 集成基座

## Context

当前 Macaca 的上下文来源包括 agent/profile prompt、session history、working memory、tool schema、skill catalog、MCP capability、trace event、长期记忆召回和 knowledge artifacts。已有 `macaca-context` 可插拔 context engine 提供了基础门面和报告，但下一步需要解决“多个来源如何以统一、可审计、可替换的方式进入模型请求”。

设计上不能让 `macaca-context` 依赖具体 Milvus、具体 MCP server、具体 skill source 或 application 代码。所有来源必须通过 provider adapter 输出标准 candidate，由 composer 统一做预算、排序、trust boundary、cache boundary 和 report。

## Goals / Non-Goals

Goals:

- 建立上下文候选项、provider、composer、plan、compiled context 的最小通用模型。
- 保证上层 runtime/framework 只调用 facade，不知道具体 provider。
- 支持用户替换 composer 或替换某一类 provider。
- 所有上下文注入可审计、可诊断、可降级。
- 避免 dynamic/untrusted context 污染 stable system prefix 或 canonical transcript。

Non-Goals:

- 不实现 profile 文件 provider。
- 不实现 active vector memory provider。
- 不实现 skills/MCP capability provider。
- 不冻结外部 RPC/WASM provider 协议。
- 不删除旧 prompt 构造路径。

## Decisions

### Decision 1: Candidate 是所有 provider 的唯一输出模型

`ContextCandidate` 使用窄字段表达来源、范围、信任、预算和内容：

- `source_id`
- `kind`
- `scope`
- `priority`
- `trust`
- `cache_class`
- `target`
- `content`
- `budget`
- `diagnostics`

理由：provider 不应直接拼 prompt，也不应修改 transcript。candidate 是 Anti-Corruption Layer，所有外部或内部来源必须先被规整为统一值对象。

### Decision 2: Provider 使用 Chain of Responsibility

provider 按 stage 运行，例如 stable profile、capability index、active recall、runtime dynamic、diagnostics。每个 provider 只负责贡献候选项和诊断，不能决定最终 prompt。

理由：这让新增来源成为添加 provider，而不是修改 runtime loop。排序和裁剪集中在 composer/policy，避免面条代码。

### Decision 3: ContextPlan 使用 Builder

`ContextPlanBuilder` 负责校验 candidate、应用预算策略、生成 selected/skipped decisions，并输出不可变 `ContextPlan`。

理由：上下文计划是多步构建结果，Builder 可以把校验、排序、去重、截断、诊断收敛在一个流程里，便于测试和回滚。

### Decision 4: CompiledContext 使用 Composite

最终上下文由 sections/tree 表示，而不是单个字符串。每个 section 带 source、stability、trust、target、token estimate 和 render metadata。

理由：stable/dynamic split、prompt cache、trace UI 和审计都需要结构化上下文，而不是事后解析字符串。

### Decision 5: Policy 全部使用 Strategy

预算、排序、去重、截断、redaction、render、trust promotion 均通过可替换策略实现，默认策略保守、确定性、低开销。

理由：Macaca 是基础设施，不应把唯一策略硬编码进 core；但也不能过度设计成远程协议。先以 in-process traits 作为稳定扩展点。

## Risks / Trade-offs

- Risk: 抽象过宽导致 framework 化而没有实际价值。Mitigation: 首版字段保持最小，具体 provider 后续按提案增量加入。
- Risk: composer 额外开销影响每次模型调用。Mitigation: 默认 provider 空集/legacy 路径应接近零开销，stable section 可缓存。
- Risk: provider 顺序影响模型行为。Mitigation: stage 和 priority 必须确定性排序，并写入 report。
- Risk: 用户 provider 输出不安全内容。Mitigation: candidate 必须经过 trust、budget、redaction 和 render policy。

## Migration Plan

1. 在 `macaca-context` 内增加 composer 相关值对象和 traits。
2. 增加 default composer/facade，空 provider 时保持 legacy 等价。
3. 将 runtime/framework 模型请求前入口改为调用 facade。
4. 标记被替代的直接 prompt 拼接入口 deprecated，但不删除。
5. 后续提案逐步注册 profile、memory、skills/MCP 和 governance providers。
