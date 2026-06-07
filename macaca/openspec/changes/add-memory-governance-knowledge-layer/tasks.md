## 1. Preparation

- [x] 1.1 阅读现有 snapshot/file/session/vector memory 代码。
- [x] 1.2 阅读 context report/artifact 相关代码。
- [x] 1.3 对计划修改的 memory facade/governance/artifacts 符号运行 GitNexus upstream impact analysis。

## 2. Governance model

- [x] 2.1 新增 `governance/mod.rs`。
- [x] 2.2 定义 `MemoryCandidate`、`CandidateSource`、`CandidateDecision`。
- [x] 2.3 定义 `PromotionPolicy`、`PromotionDecision`。
- [x] 2.4 定义 `MemoryAuditEvent`、`MemoryTombstone`。
- [x] 2.5 定义 deletion propagation result DTO。

## 3. Candidate and promotion

- [x] 3.1 实现 candidate store 接口。
- [x] 3.2 实现 default conservative promotion policy。
- [x] 3.3 支持 agent private candidate。
- [x] 3.4 支持 session shared candidate。
- [x] 3.5 用户显式记忆直接写高置信长期层。

## 4. Deletion and audit

- [x] 4.1 删除时写 tombstone。
- [x] 4.2 删除传播到 file/session/vector/provider。
- [x] 4.3 删除失败记录 diagnostics 和 retryable event。
- [x] 4.4 所有 write/promote/delete 记录 audit event。

## 5. Knowledge layer

- [x] 5.1 新增 `KnowledgeCompileCapability`。
- [x] 5.2 定义 claim/evidence/decision/constraint/preference/freshness/conflict model。
- [x] 5.3 实现 default compiler skeleton 或 no-op compiler。
- [x] 5.4 将 compiled digest 作为 context source 候选。

## 6. Artifacts

- [x] 6.1 新增 `artifacts/mod.rs`。
- [x] 6.2 实现 memory report artifact。
- [x] 6.3 实现 project decision log artifact。
- [x] 6.4 实现 wiki digest artifact skeleton。
- [x] 6.5 默认 artifact 不输出完整敏感原文。

## 7. Tests

- [x] 7.1 candidate 不直接污染长期记忆测试。
- [x] 7.2 promotion policy 测试。
- [x] 7.3 tombstone 防复活测试。
- [x] 7.4 delete propagation diagnostics 测试。
- [x] 7.5 claim/evidence conflict model 测试。
- [x] 7.6 artifact redaction 测试。

## 8. Verification

- [x] 8.1 运行 `cargo fmt`。
- [x] 8.2 运行 `cargo test -p macaca-memory`。
- [x] 8.3 运行相关上层 cargo check。
- [x] 8.4 运行 `openspec validate add-memory-governance-knowledge-layer --strict`。
- [x] 8.5 运行 `gitnexus_detect_changes()`。

Note: Rebuilt the GitNexus index for `/Users/quantum/Code/dev/agent` with
`npx gitnexus analyze . --force --skip-agents-md`, then ran `npx gitnexus
impact -r agent --depth 2` for:
- `Struct:macaca/crates/macaca-memory/src/governance/facade.rs:GovernedMemoryFacade`
- `Struct:macaca/crates/macaca-memory/src/governance/candidate.rs:MemoryCandidate`
- `Struct:macaca/crates/macaca-memory/src/governance/compiler.rs:KnowledgeCompiler`

All three impact checks returned `risk: LOW` and `impactedCount: 0`. A follow-up
`npx gitnexus detect-changes -s all -r agent` reported `Changes: 3 files, 21
symbols` and `Risk level: low`.
