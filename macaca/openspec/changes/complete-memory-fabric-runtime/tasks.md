# Tasks

## 1. OpenSpec Truth Alignment

- [x] 1.1 确认 `complete-memory-fabric-runtime` change id 唯一，且未与其他 active memory/context runtime 提案重复定义同一行为。
- [x] 1.2 创建 `proposal.md`，说明完成六阶段 memory fabric 的原因、范围和兼容性。
- [x] 1.3 创建 `design.md`，覆盖 runtime boundary、provider runtime、embedding/query、knowledge/governance、production migration。
- [x] 1.4 创建本 `tasks.md`，覆盖计划 Task 1-9。
- [x] 1.5 创建 `specs/macaca-memory-fabric-runtime/spec.md`，覆盖所有行为需求。
- [x] 1.6 修正 `add-memory-provider-runtime/tasks.md` 中 MCP live client 未实现却被标记完成的条目。
- [x] 1.7 确认 `add-memory-vector-backend-topology/tasks.md` 的 conformance harness 仍未误标完成。
- [x] 1.8 确认 `add-knowledge-digest-context-provider/tasks.md` 的 redaction/report/e2e/GitNexus 项仍未误标完成。
- [x] 1.9 运行所有相关 OpenSpec strict validation。

## 2. Runtime Facade Boundary

- [x] 2.1 对 `MemoryFacade`、`MemoryProviderRuntime`、`DefaultActiveRecallStrategy`、`KnowledgeCompiler` 运行 GitNexus upstream impact analysis。
- [x] 2.2 新增 runtime facade composition failing tests。
- [x] 2.3 创建 `macaca-memory/src/runtime/facade.rs`，定义 `MemoryRuntimeFacade` 和 `ComposedMemoryRuntime`。
- [x] 2.4 创建 `macaca-memory/src/runtime/status.rs`，定义 `MemoryRuntimeStatus`。
- [x] 2.5 创建 `macaca-memory/src/runtime/builder.rs`，用 already-built components 构建 runtime。
- [x] 2.6 创建 `macaca-memory/src/runtime/mod.rs` 并导出 runtime API。
- [x] 2.7 更新 `macaca-memory/src/lib.rs` re-export。
- [x] 2.8 运行 `cargo fmt`、`cargo test -p macaca-memory runtime:: --lib`、`cargo test -p macaca-memory --lib`。

## 3. Provider Runtime and MCP Truthful Capability

- [x] 3.1 对 `MemoryProviderRuntime`、`McpMemoryProvider`、`BuiltinMemoryProviderFactory` 运行 GitNexus upstream impact analysis。
- [x] 3.2 增加 MCP unwired status 和 provider profile slots tests。
- [x] 3.3 修改 MCP provider status，未接 live client 时不得报告 healthy store/search capability。
- [x] 3.4 增加 `MemoryMcpClient` trait 和 `McpMemoryProvider::with_client(...)` seam。
- [x] 3.5 实现 live client schema mapping：remember/search/get/delete 到配置的 MCP tools。
- [x] 3.6 缺失 required MCP tool 时返回 capability error 并记录 status diagnostic。
- [x] 3.7 运行 `cargo fmt`、`cargo test -p macaca-memory providers:: --lib`、`openspec validate add-memory-provider-runtime --strict`。

## 4. Embedding Registry and Decorators

- [x] 4.1 对 `EmbeddingProvider`、`CachedEmbeddingProvider`、`DashScopeEmbedding` 运行 GitNexus upstream impact analysis。
- [x] 4.2 新增 registry、timeout、retry tests。
- [x] 4.3 创建 `embedding_registry.rs`，定义 `EmbeddingProviderFactory` 和 `EmbeddingProviderRegistry`。
- [x] 4.4 创建 `embedding_decorators.rs`，实现 timeout、retry、metrics，并保留 existing cache。
- [x] 4.5 更新 `lib.rs` exports。
- [x] 4.6 运行 embedding 相关 cargo tests。

## 5. Vector Backend Conformance and Query Pipeline

- [x] 5.1 对 `VectorMemoryBackend`、`TopologyVectorMemoryBackend`、`VectorQueryStrategy` 运行 GitNexus upstream impact analysis。
- [x] 5.2 创建 `vector_conformance.rs`，提供 reusable backend conformance harness。
- [x] 5.3 将现有 private conformance checks 迁移/复用到 harness。
- [x] 5.4 创建 `query_pipeline.rs`，定义 `MemoryQueryMode`、`MemoryQueryPipeline`、`MemoryQueryPipelineResult`。
- [x] 5.5 实现 keyword/vector/hybrid/filtered strategy，支持 embedding/vector failure keyword fallback。
- [x] 5.6 默认 runtime 行为保持不变，除非显式配置新 pipeline。
- [x] 5.7 运行 vector/query 相关 cargo tests 和 `openspec validate add-memory-vector-backend-topology --strict`。

## 6. Knowledge Compiler, Wiki Digest, and Project Decision Artifacts

- [x] 6.1 对 `KnowledgeCompiler`、`WorkspaceKnowledgeDigestCapability`、`KnowledgeDigestContextProvider` 运行 GitNexus upstream impact analysis。
- [x] 6.2 增加 contradiction detection 和 exact evidence id tests。
- [x] 6.3 定义 `ContradictionStrategy`，默认支持 deterministic negation pairs。
- [x] 6.4 扩展 artifacts：ProjectDecisionLogArtifact、WikiDigestArtifact、CitationArtifact。
- [x] 6.5 artifact 默认只包含 bounded/redacted text 和 evidence ids。
- [x] 6.6 增加 structured digest report 不泄漏敏感全文和 redacted rendering tests。
- [x] 6.7 运行 governance/knowledge digest 相关 cargo tests 与 OpenSpec validations。

## 7. Governance Runtime and Provider Migration

- [x] 7.1 对 `GovernedMemoryFacade`、`MemorySnapshotStore`、`MemoryPromotionPolicy` 运行 GitNexus upstream impact analysis。
- [x] 7.2 创建 `governance/runtime.rs`，定义 durable `MemoryGovernanceJournal` trait 和 in-memory implementation。
- [x] 7.3 接入 snapshot/replay，不新增数据库依赖。
- [x] 7.4 增加 `MemoryCandidateCapturePolicy`，默认捕获 explicit user memory 和 agent summaries，拒绝空内容。
- [x] 7.5 创建 `governance/migration.rs`，定义 migration plan/status 和 copy/verify/rollback checkpoint。
- [x] 7.6 失败 verification 必须保持 source provider authoritative，并写 audit event。
- [x] 7.7 增加 `MemoryCompactionStrategy` seam，默认返回空 candidates 并记录 `compaction_disabled`。
- [x] 7.8 运行 governance/snapshot 相关 cargo tests。

## 8. Migrate macaca-web Production Consumers

- [x] 8.1 对 `ContextReportingChatModel`、`WorkspaceMemoryRecallSource`、`WorkspaceMemorySearchTool`、`WorkspaceKnowledgeDigestCapability` 运行 GitNexus upstream impact analysis，并在 HIGH/CRITICAL 时先告知用户。
- [x] 8.2 增加 web runtime adapter tests。
- [x] 8.3 创建 `macaca-web/src/memory_runtime.rs`，定义 `WebMemoryRuntime` over `Arc<dyn MemoryRuntimeFacade>`。
- [x] 8.4 更新 `AppState`，增加 `memory_runtime`，保留 `workspace_memory` 为 legacy builtin backing store。
- [x] 8.5 在 web bootstrap 初始化 runtime facade over builtin adapter。
- [x] 8.6 迁移 memory search/get/forget tools 到 runtime adapter。
- [x] 8.7 迁移 active recall source 和 knowledge digest capability 到 runtime facade。
- [x] 8.8 扫描确认 direct `TestMemoryManager` recall 仅剩 legacy backing store、tests 或 compatibility adapters。
- [x] 8.9 运行 macaca-web memory/context tests 和 focused cargo check。

## 9. End-to-End Verification and GitNexus

- [x] 9.1 运行 `cargo test -p macaca-memory --lib`。
- [x] 9.2 运行 `cargo test -p macaca-context --lib`。
- [x] 9.3 运行 `cargo test -p macaca-web --lib`。
- [x] 9.4 运行 focused `cargo check -p macaca-memory -p macaca-context -p macaca-web -p macaca-framework -p macaca-kernel`。
- [x] 9.5 运行所有相关 OpenSpec strict validation。
- [x] 9.6 扫描 `not wired to a live client yet`、direct `workspace_memory.recall`、direct `TestMemoryManager` 使用。
- [x] 9.7 扫描 runtime status/provider runtime 路径是否存在。
- [x] 9.8 运行 `gitnexus_detect_changes()`，确认影响范围符合 memory/runtime/context/web migration scope。
- [x] 9.9 当前未要求重启前后端；手工 E2E 门保留为后续按需验证项，自动 E2E 已覆盖 session recall、context report、memory_search/get/forget、refresh/no duplicate injection 的当前实现路径。
