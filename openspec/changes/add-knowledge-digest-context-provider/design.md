# Design: Knowledge Digest Context Provider

## Context

`macaca-memory` 已有 candidate、promotion、audit、tombstone、knowledge compiler、compiled digest candidates 和 artifacts。Phase 6 的目标不是重做这些底层能力，而是让 context composer 能消费治理后的知识层。

OpenClaw 的 wiki/active-memory 经验表明，长期自治系统需要把原始片段提升为 claim/evidence 结构，减少重复、冲突和过期上下文。Macaca 需要在不强耦合 memory 与 context 的前提下，把治理后的知识摘要作为更高质量的 context source。

## Goals / Non-Goals

Goals:

- 将 compiled digest、claims 和 artifacts 适配为 context candidates。
- 在有充分 evidence 时优先 digest/claim，减少 raw recall 噪声。
- 保留 evidence 可追溯性，但默认不暴露完整敏感原文。
- 遵守 tombstone、redaction、scope、freshness 和 confidence。
- 支持可替换 digest-vs-raw selection strategy。

Non-Goals:

- 不重新实现 memory candidate/promotion/governance。
- 不实现新的 knowledge compiler。
- 不把 digest 写入 canonical transcript。
- 不把 memory governance 逻辑放进 runtime/framework。
- 不硬编码 Milvus、application name、workflow name 或业务名称。

## Decisions

### Decision 1: Provider 使用 Adapter 读取 governance 输出

`KnowledgeDigestContextProvider` 只依赖 memory governance/knowledge capability 的窄接口或 DTO，例如 compiled digest candidates、claims 和 artifacts。它不直接读取 vector backend，不执行 promotion，也不修改 memory store。

理由：保持 context 与 memory governance 解耦，符合 Ports and Adapters。

### Decision 2: Digest-vs-raw 选择使用 Strategy

默认选择策略考虑：

- evidence coverage
- confidence
- freshness
- conflict/supersedes
- tombstone state
- scope visibility
- budget
- raw recall score

当 digest 有足够 evidence 且未过期时，优先 digest；当 digest stale 或 confidence 不足时，允许 fresh raw recall 进入动态上下文。

### Decision 3: Redaction 和 audit 使用 Decorator

provider 输出必须经过 redaction/audit decorator。report 可显示 evidence ids、hash、source labels 和 artifact ids，但默认不显示完整 source memory content。

### Decision 4: Digest context 仍是 request-only

knowledge digest 是高质量上下文，但它仍是模型请求上下文，不应写回 canonical transcript。是否晋升或更新 knowledge layer 由 memory governance workflow 决定。

## Risks / Trade-offs

- Risk: stale digest 压过 fresh recall。Mitigation: freshness/confidence/recency strategy，stale decision 进入 report。
- Risk: evidence id 泄漏敏感上下文。Mitigation: evidence 默认以 id/hash/source label 表达，完整内容需要显式 debug。
- Risk: digest 与 raw recall 重复注入。Mitigation: source/evidence 去重和 digest-vs-raw selection。
- Risk: context provider 绕过 tombstone。Mitigation: provider 必须通过 governance capability 或 tombstone-aware adapter。

## Migration Plan

1. 定义 knowledge digest context provider 和 selection policy。
2. 将 existing compiled digest candidates 适配到 `ContextCandidate`。
3. 与 active recall provider 的 raw candidates 做去重/优先级选择。
4. 把 evidence、redaction、tombstone、stale decisions 写入 `ContextReport`。
5. 添加 tests 覆盖 Phase 6 的全部控制点。
