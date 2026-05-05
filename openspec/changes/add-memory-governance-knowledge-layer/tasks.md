## 1. Preparation

- [ ] 1.1 阅读现有 snapshot/file/session/vector memory 代码。
- [ ] 1.2 阅读 context report/artifact 相关代码。
- [ ] 1.3 对计划修改的 memory facade/governance/artifacts 符号运行 GitNexus upstream impact analysis。

## 2. Governance model

- [ ] 2.1 新增 `governance/mod.rs`。
- [ ] 2.2 定义 `MemoryCandidate`、`CandidateSource`、`CandidateDecision`。
- [ ] 2.3 定义 `PromotionPolicy`、`PromotionDecision`。
- [ ] 2.4 定义 `MemoryAuditEvent`、`MemoryTombstone`。
- [ ] 2.5 定义 deletion propagation result DTO。

## 3. Candidate and promotion

- [ ] 3.1 实现 candidate store 接口。
- [ ] 3.2 实现 default conservative promotion policy。
- [ ] 3.3 支持 agent private candidate。
- [ ] 3.4 支持 session shared candidate。
- [ ] 3.5 用户显式记忆直接写高置信长期层。

## 4. Deletion and audit

- [ ] 4.1 删除时写 tombstone。
- [ ] 4.2 删除传播到 file/session/vector/provider。
- [ ] 4.3 删除失败记录 diagnostics 和 retryable event。
- [ ] 4.4 所有 write/promote/delete 记录 audit event。

## 5. Knowledge layer

- [ ] 5.1 新增 `KnowledgeCompileCapability`。
- [ ] 5.2 定义 claim/evidence/decision/constraint/preference/freshness/conflict model。
- [ ] 5.3 实现 default compiler skeleton 或 no-op compiler。
- [ ] 5.4 将 compiled digest 作为 context source 候选。

## 6. Artifacts

- [ ] 6.1 新增 `artifacts/mod.rs`。
- [ ] 6.2 实现 memory report artifact。
- [ ] 6.3 实现 project decision log artifact。
- [ ] 6.4 实现 wiki digest artifact skeleton。
- [ ] 6.5 默认 artifact 不输出完整敏感原文。

## 7. Tests

- [ ] 7.1 candidate 不直接污染长期记忆测试。
- [ ] 7.2 promotion policy 测试。
- [ ] 7.3 tombstone 防复活测试。
- [ ] 7.4 delete propagation diagnostics 测试。
- [ ] 7.5 claim/evidence conflict model 测试。
- [ ] 7.6 artifact redaction 测试。

## 8. Verification

- [ ] 8.1 运行 `cargo fmt`。
- [ ] 8.2 运行 `cargo test -p macaca-memory`。
- [ ] 8.3 运行相关上层 cargo check。
- [ ] 8.4 运行 `openspec validate add-memory-governance-knowledge-layer --strict`。
- [ ] 8.5 运行 `gitnexus_detect_changes()`。
