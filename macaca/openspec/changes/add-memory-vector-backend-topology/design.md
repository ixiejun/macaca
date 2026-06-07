## Context

现有 `IsolatedMemoryManager` 注释已经表达了目标隔离模型：

- 文件目录：`{base_path}/{app_id}/{agent_id}/{memory_id}.json`
- Milvus 多租户：Application → Database，Agent → Collection

本提案将这个实现约定提升为正式架构 contract。Milvus 是默认实现，其他后端必须映射同等语义。

## Goals

- 保留 Milvus 作为默认长期向量记忆 backend。
- 定义供应商无关的 `VectorMemoryBackend` contract。
- 保证 agent private vector memory 默认以 agent collection 隔离。
- 支持 session/project shared vector memory 的显式 collection。
- 支持替代 backend，但必须通过 topology conformance tests。

## Non-Goals

- 不实现所有第三方向量库。
- 不移除现有 `VectorStore` trait。
- 不强制所有 memory provider 必须使用向量后端。
- 不改变 embedding provider 的厂商选择。
- 不新增额外 crate。

## Decisions

### Decision 1: Milvus remains the default implementation

默认向量后端使用 Milvus。

默认拓扑：

```text
Application
  └── Milvus Database
        ├── Agent A Collection
        ├── Agent B Collection
        ├── session_<session_id> Collection
        └── project_<project_id> Collection
```

agent private recall 默认只搜索当前 agent collection。

session/project shared recall 必须通过 `MemoryVisibility::SessionShared` 显式路由到 session/project collection。

### Decision 2: `VectorMemoryBackend` sits above `VectorStore`

现有 `VectorStore` 是底层 upsert/search/delete 抽象，不表达 database/collection 拓扑。

新增 `VectorMemoryBackend` 表达：

- ensure application database
- ensure agent collection
- ensure shared collection
- upsert record
- search collection
- delete record
- rebuild / status / diagnostics 后续扩展

`VectorStore` 保留用于低层兼容和单 collection adapter。

### Decision 3: Replacement backend must prove topology equivalence

替代 backend 不必使用 Milvus 的 API 名称，但必须等价支持：

- application 隔离域。
- agent 隔离单元。
- session/project shared 隔离单元。
- scoped search 不跨 agent 泄漏。
- collection-level rebuild/delete/status。

Qdrant、LanceDB、remote vector backend 都需要 adapter 映射。

### Decision 4: Schema carries provenance

每条 vector memory record 至少携带：

- memory id
- scope
- visibility
- content
- vector
- metadata
- created_at / updated_at
- source/provenance
- optional confidence/freshness/conflict fields

这样 active recall、governance 和 trace 可以解释命中来源。

## Risks / Trade-offs

- Risk: 替代 backend 使用单 namespace，无法天然隔离。
  - Mitigation: 不允许作为默认 long-term vector backend，只能作为 supplement/remote RAG，除非 adapter 证明隔离。
- Risk: Milvus database API 与部署版本差异。
  - Mitigation: backend adapter 提供 diagnostics 和 fallback，contract 测试模拟 topology 行为。
- Risk: session shared collection 命名与 agent collection 冲突。
  - Mitigation: 使用 reserved prefix，例如 `session_`、`project_`，并进行 sanitization。

## Migration Plan

1. 新增 `vector/` 模块与 `VectorMemoryBackend` trait。
2. 包装现有 Milvus store 为默认 backend。
3. 增加 topology mapping helper。
4. 增加 conformance tests。
5. 保留现有 `vector.rs` re-export。
