# Design: Complete Memory Fabric Runtime

## Context

`docs/superpowers/specs/2026-05-06-complete-memory-system-phases-design.md` 审计结论是：Phase 1 和 Phase 4 基本可用，Phase 2、Phase 3、Phase 5、Phase 6 仍未完整落地。尤其是 MCP provider 未接入 live client、provider runtime 未成为 `macaca-web` 生产入口、embedding/index/vector 解耦不完整、query strategy stack 缺失、knowledge/wiki 只有基础 DTO、governance 多为 in-memory decorator。

本设计采用推荐的 Option B：通过可逆的增量 slice 完成 runtime contracts，再迁移生产消费者。现有工作行为必须保留，旧 API 保留为 compatibility adapters。

## Goals / Non-Goals

Goals:

- 让上层 crates 使用单一 `MemoryRuntimeFacade`。
- 让 provider runtime 从 profile/slot 构建真实 provider，并真实报告 capability。
- 将 embedding、vector backend、query pipeline 分层解耦。
- 让 knowledge compiler 产生可追溯 claims、conflicts、decision logs 和 wiki digest artifacts。
- 让 governance 具备 durable snapshot/replay、candidate capture、migration checkpoint 和 truthful compaction seam。
- 迁移 `macaca-web` active recall、memory tools、knowledge digest 到 facade-backed runtime。

Non-Goals:

- 不删除 legacy memory manager APIs。
- 不把整个 memory system 拆成多个新 crate。
- 不要求默认测试依赖外部服务。
- 不实现完整 autonomous dreaming；本变更只提供 truthful disabled seam。
- 不修改用户可见 trace event 语义，除非增加缺失 diagnostics。

## Runtime Boundary

`MemoryRuntimeFacade` 是上层 crate 的 canonical boundary。它包装现有 `MemoryFacade`、provider runtime、active recall、knowledge compiler 和 governance runtime。

目标 trait：

```rust
#[async_trait::async_trait]
pub trait MemoryRuntimeFacade: Send + Sync {
    async fn remember(&self, request: MemoryWriteRequest) -> MacacaResult<MemoryId>;
    async fn search(&self, request: MemorySearchRequest) -> MacacaResult<Vec<MemoryEntry>>;
    async fn active_recall(&self, request: ActiveRecallRequest) -> MacacaResult<ActiveRecallResult>;
    async fn compile_knowledge(&self, request: KnowledgeCompileRequest) -> MacacaResult<KnowledgeCompileResult>;
    async fn status(&self) -> MemoryRuntimeStatus;
}
```

`ComposedMemoryRuntime` 使用 Facade 模式隐藏 provider registry、routing、active recall、knowledge 和 governance 组合。legacy managers 通过 Adapter 接入。

## Provider Runtime

`MemoryProviderRuntime` 必须成为 factory-backed runtime，而不是 status wrapper。它按 profile config 解析以下 slot：

- agent private memory provider
- session shared memory provider
- embedding provider
- vector backend
- active recall provider
- knowledge compiler provider

MCP provider 必须满足二选一：

- 接入 live MCP client seam 并按配置映射 store/search/get/delete tools。
- 明确报告 unavailable/unsupported capability，不得声称 healthy store/search 后返回固定 `not wired` 错误。

## Embedding and Query Stack

Embedding provider 通过 `EmbeddingProviderRegistry` 选择。Decorator 链提供：

- cache
- timeout
- retry
- metrics

Query pipeline 使用 Strategy：

- keyword
- vector
- hybrid
- filtered
- rerank-compatible

当 embedding 或 vector backend 失败/缺失时，hybrid query 必须降级到 keyword fallback，并记录 diagnostics，不阻塞 agent run。

## Vector Backend Conformance

`VectorMemoryBackend` 合约必须有可复用 conformance harness，验证：

- AgentPrivate collection isolation。
- SessionShared/project shared collection 必须显式，不混入 agent private。
- topology/status 可报告。

至少 builtin 和 Milvus topology 实现需要通过该 harness；替代 backend 可复用同一测试。

## Knowledge and Governance

Knowledge compiler 输出：

- claims
- evidence references
- deterministic conflict groups
- project decision log artifacts
- wiki digest artifacts
- citation artifacts

默认 contradiction strategy 不调用 LLM，只处理确定性 negation patterns，例如 `X requires Y` vs `X does not require Y`。

Governance runtime 提供：

- durable audit journal trait。
- candidate capture policy。
- snapshot/replay integration。
- tombstone authoritative propagation。
- provider migration copy/verify/rollback checkpoints。
- compaction/dreaming seam，默认 truthful disabled 并记录 `compaction_disabled`。

## Production Migration

`macaca-web` 初始化 `WebMemoryRuntime` adapter，内部持有 `Arc<dyn MemoryRuntimeFacade>`。Active recall source、memory search/get/forget tools 和 workspace knowledge digest capability 必须通过该 adapter 访问记忆。

`workspace_memory` 和 `TestMemoryManager` 可以继续存在，但只作为 builtin backing store、legacy compatibility 或测试路径。

## Design Pattern Mapping

- Facade: `MemoryRuntimeFacade`。
- Adapter: legacy managers、remote provider、MCP provider、Milvus backend、web adapter。
- Strategy: provider selection、query pipeline、contradiction detection、promotion/migration policy。
- Decorator: timeout、retry、metrics、cache、redaction、circuit breaker。
- Abstract Factory: provider/backend/embedding factories。
- Chain of Responsibility: write governance pipeline 和 query pipeline。
- Memento: snapshot/restore 和 migration checkpoints。
- Observer: provider diagnostics/runtime memory events 进入 context report/trace。
- Proxy: remote/MCP/vector backend clients。

## Risks / Trade-offs

- Risk: `macaca-web` 迁移影响 live recall 和用户可见 context report。Mitigation: lower-crate tests 先通过，再迁移 web adapter；保留 legacy backing store。
- Risk: MCP provider 继续假阳性健康状态。Mitigation: spec 明确 truthful capability，测试覆盖 unwired/live seam。
- Risk: embedding/vector failure 阻塞 agent。Mitigation: keyword fallback、timeout、fail-open diagnostics。
- Risk: governance durable runtime 过度设计。Mitigation: 先提供 trait + in-memory snapshot/replay，不引入数据库依赖。
- Risk: digest 与 raw recall 重复注入。Mitigation: context selection 和 evidence/source 去重由 existing digest proposal 协同验证。

## Migration Plan

1. 写入本 OpenSpec，并修正 false-complete task 状态。
2. 新增 `MemoryRuntimeFacade` 和 runtime builder/status。
3. 完成 provider runtime 与 MCP truthful capability。
4. 完成 embedding registry/decorators。
5. 完成 vector conformance 和 query pipeline。
6. 完成 knowledge compiler/artifacts 增强。
7. 完成 governance runtime/migration/compaction seam。
8. 迁移 `macaca-web` production memory consumers。
9. 执行 focused tests、OpenSpec validation、direct access scans、GitNexus detect changes。
