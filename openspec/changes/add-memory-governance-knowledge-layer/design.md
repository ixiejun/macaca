## Context

OpenClaw memory-wiki 展示了 claim/evidence/freshness/contradiction 价值。Hermes/Supermemory 展示了 auto capture 与 provider extraction 的价值，但自动写入必须谨慎。Macaca 需要 memory governance 作为长期自治基础。

## Goals

- 自动捕获先进入 candidate layer，不直接污染长期记忆。
- 用户显式记忆可写入高置信长期层。
- 支持 promotion policy 可替换。
- 支持 audit log、tombstone、deletion propagation。
- 支持 knowledge compiler，将原始记忆编译成 claim/evidence/decision/constraint。
- 支持 artifacts/report/wiki 输出。

## Non-Goals

- 不要求首轮实现复杂 LLM dreaming。
- 不要求首轮实现完整 Obsidian wiki。
- 不默认公开完整 memory content。
- 不新增额外 crate。

## Decisions

### Decision 1: Candidate layer separates capture from commitment

自动捕获、agent 自我总结、tool observation、delegation observation 默认写入 candidate。

显式用户“记住”或明确 policy 才可直接写长期高置信记忆。

### Decision 2: Promotion policy is replaceable

Promotion 根据：

- source confidence
- recurrence
- explicitness
- recency/freshness
- agent/session visibility
- conflict status
- privacy policy

决定是否晋升到 agent private、session shared、application shared、user scoped 或 knowledge layer。

### Decision 3: Deletion uses tombstone and propagation

删除不只是从本地 store 移除。需要：

- tombstone 防止重建索引时复活。
- provider delete propagation。
- vector delete。
- artifact update。
- audit event。

### Decision 4: Knowledge layer compiles structured memory

Knowledge compiler 输出：

- claim
- evidence references
- decision
- constraint
- preference
- freshness
- contradiction/conflict group
- supersedes/revoked

Context engine 可优先消费 compiled digest，再按需回溯 evidence。

### Decision 5: Artifacts are public summaries, not raw leak

Artifacts 可以包括：

- markdown memory report
- project decision log
- wiki digest
- governance audit summary

默认不输出 secrets 或完整敏感原文。

## Risks / Trade-offs

- Risk: 自动候选过多。
  - Mitigation: filters、rate limit、dedupe、promotion thresholds。
- Risk: knowledge compiler 过度抽象丢失证据。
  - Mitigation: 每条 compiled item 必须带 evidence references。
- Risk: 删除 propagation 不完整。
  - Mitigation: tombstone + provider result diagnostics + retry queue。

## Migration Plan

1. 定义 governance DTO。
2. 实现 candidate store。
3. 实现 default promotion policy。
4. 实现 audit/tombstone。
5. 实现 knowledge compiler trait 和 default no-op/local compiler。
6. 实现 artifact provider。
7. 接入 context digest source。
