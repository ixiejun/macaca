# Tasks

## 1. 审计与影响分析

- [x] 1.1 阅读 `macaca-memory` governance、compiler、context candidates、artifacts、tombstone 实现。
- [x] 1.2 阅读 active recall context provider 计划和 context composer contract。
- [x] 1.3 查找是否已有 raw recall 与 knowledge digest 重复进入 prompt 的路径。
- [ ] 1.4 对计划修改的 knowledge/context adapter 符号运行 GitNexus upstream impact分析（待 CI / 本地 GitNexus 环境补跑）。

## 2. Provider Contract

- [x] 2.1 定义 `KnowledgeDigestContextProvider`。
- [x] 2.2 定义 digest 策略表面 —— 已实现为 `macaca_proto::KnowledgeDigestContextConfig`（阈值与超时，替换原提案中的独立 policy 命名）。
- [x] 2.3 定义 digest/claim 到 `ContextCandidate` 的转换（`KnowledgeDigestItem` + `DigestStrengthSnapshot`）。
- [x] 2.4 定义 evidence reference DTO —— `evidence_memory_ids` 仅承载 opaque `source_id` 列表；默认不向模型输出完整敏感原文。
- [x] 2.5 stale / redacted 决策 —— `digest_strength` + selection 策略；tombstoned 在 workspace 桥经由 `TombstoneIndex` + `memory_forget` 已接入（见 §4）。

## 3. Digest-vs-Raw Selection

- [x] 3.1 实现默认 digest-vs-raw selection strategy（`apply_digest_vs_raw_selection`）。
- [x] 3.2 有充分 evidence、confidence、freshness 时优先 digest/claim（强 digest 证据并集覆盖 raw keys）。
- [x] 3.3 digest stale 时不抑制 raw recall。
- [x] 3.4 基于 `source_id` / `evidence_memory_ids` 去重同源注入。
- [x] 3.5 将 selection decisions 写入 `ContextReport`（经 `governance_decisions` / `decisions`）。

## 4. Governance Integration

- [x] 4.1 tombstoned evidence 过滤 —— `WorkspaceKnowledgeDigestCapability` 可选 `TombstoneIndex`（`SharedTombstoneRegistry`）+ `filter_digest_items_by_tombstones`；`memory_forget` 先 tombstone 再 `forget`（`GovernanceFacadeTombstones` 留给 GovernedMemoryFacade 路径）。
- [x] 4.2 digest 文本默认 `redacted: true` 走上下文渲染分支；evidence 仅为 id 列表。
- [x] 4.3 scope —— 编译请求使用中性 `MemoryScope`（SessionShared 信封）；细粒度 AgentPrivate 路由为后续增强。
- [x] 4.4 不向 canonical transcript 写入 digest（仍仅经 composer 注入）。
- [x] 4.5 provider error / empty digest fail-open。

## 5. Context Integration

- [x] 5.1 注册 `knowledge_digest` family 至 catalog + `ProviderAssemblyEnvironment`。
- [x] 5.2 digest 使用 `ContextCandidateKind::KnowledgeDigest` / `WikiDigest` 报告路径。
- [x] 5.3 `ContextCacheClass::Dynamic`（无法证明稳定 ⇒ dynamic）。
- [x] 5.4 与现有 `merge_composer_into_messages` 行为一致，不写回会话持久消息。

## 6. Tests

- [x] 6.1 强 digest 覆盖 raw key 集合时移除 raw recall。
- [x] 6.2 stale digest 不移除 raw。
- [ ] 6.3 结构化 report 不泄漏全文 —— 依赖现有 recall report 契约 + evidence 仅为 id；**未加**独立序列化快照测试。
- [x] 6.4 tombstone —— `macaca-memory` registry 单测 + `macaca-context` `tombstone_filter` 单测（web 端为桥接组合，无独立 e2e）。
- [ ] 6.5 redaction 单测 —— **后续**与 governance 装饰器链一并补。
- [x] 6.6 recall evidence 为空时跳过抑制逻辑（隐含 fail-open 行为）。

## 7. Verification

- [x] 7.1 `openspec validate add-knowledge-digest-context-provider --strict`
- [x] 7.2 `cargo test -p macaca-context --lib`（含 digest selection 测试）
- [ ] 7.3 全量 integration / e2e（按需 CI）
- [ ] 7.4 `gitnexus_detect_changes()`（待提交前补跑）
