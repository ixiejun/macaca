# Change: 增加长期向量记忆后端拓扑

## Why

Macaca 当前长期记忆向量化默认采用 Milvus，并已经形成关键隔离模型：一个 application 对应一个向量 database，该 database 下每个 agent 对应一个 collection。这个模型应上升为供应商无关的 `VectorMemoryBackend` contract，而不是仅作为 Milvus 实现细节。

Macaca 不绑定 Milvus 供应商，但默认实现应继续使用 Milvus。用户可以替换为支持等价 application database + agent collection 拓扑语义的其他向量数据库或远程向量服务。

## What Changes

- 在 `macaca-memory` 单 crate 内增加 `vector/` 模块组织，不新增额外 crate。
- 定义 `VectorMemoryBackend`、`VectorMemoryTopology`、database/collection handle、collection schema、vector record、vector hit。
- 将 Milvus 作为默认 `VectorMemoryBackend` 实现。
- 强制默认拓扑：`application_id -> database`，`agent_id/agent_name -> collection`。
- 定义 session/project shared vector collection 的命名和路由原则。
- 为替代 backend 定义 conformance requirements 和 contract tests。
- 保持现有 `VectorStore` trait 兼容；`VectorMemoryBackend` 是更高层拓扑 contract。

## Impact

- Affected specs: `macaca-memory-vector-backend`
- Affected code:
  - `macaca/crates/macaca-memory/src/vector/`
  - 现有 `vector.rs` 的兼容 re-export 或渐进目录化
  - `macaca/crates/macaca-memory/src/core/scope.rs`
- Compatibility:
  - Milvus 仍是默认长期向量记忆实现。
  - 现有 `VectorStore` API 不删除。
  - 非 Milvus backend 必须证明等价拓扑能力。
