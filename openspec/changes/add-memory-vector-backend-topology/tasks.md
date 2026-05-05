## 1. Preparation

- [x] 1.1 阅读当前 `vector.rs`、`isolated.rs`、`backend.rs`、`store.rs`。
- [x] 1.2 对 `MilvusStore`、`VectorStore`、`IsolatedMemoryManager` 运行 GitNexus upstream impact analysis。
- [x] 1.3 运行 baseline `cargo test -p macaca-memory`。

## 2. Vector module structure

- [x] 2.1 新增 `macaca-memory/src/vector/mod.rs` 或渐进保留 `vector.rs` 并挂载子模块。
- [x] 2.2 新增 `VectorMemoryTopology`、database handle、collection handle。
- [x] 2.3 新增 `VectorCollectionSchema`、`VectorMemoryRecord`、`VectorMemoryHit`。
- [x] 2.4 新增 `VectorMemoryBackend` trait。

## 3. Milvus backend

- [x] 3.1 将现有 Milvus 实现包装为默认 `MilvusVectorMemoryBackend`。
- [x] 3.2 实现 `application_id -> database` mapping。
- [x] 3.3 实现 `agent_id/agent_name -> collection` mapping。
- [x] 3.4 实现 session/project shared collection mapping。
- [x] 3.5 增加 topology sanitization，避免非法 database/collection 名称。

## 4. Contract tests

- [x] 4.1 添加 application database mapping 测试。
- [x] 4.2 添加 agent collection isolation 测试。
- [x] 4.3 添加 session shared collection 不混入 agent private collection 的测试。
- [ ] 4.4 添加替代 backend conformance test harness。
- [x] 4.5 添加 vector record provenance fields 测试。

## 5. Compatibility

- [x] 5.1 保留现有 `VectorStore` trait。
- [x] 5.2 保留现有 `MilvusStore` 可构造路径或提供兼容 re-export。
- [x] 5.3 确认现有 `MemoryManager` / `IsolatedMemoryManager` 测试继续通过。

## 6. Verification

- [x] 6.1 运行 `cargo fmt`。
- [x] 6.2 运行 `cargo test -p macaca-memory`。
- [ ] 6.3 运行 `cargo check -p macaca-memory -p macaca-kernel -p macaca-agent -p macaca-framework -p macaca-web`。
- [ ] 6.4 运行 `openspec validate add-memory-vector-backend-topology --strict`。
- [ ] 6.5 运行 `gitnexus_detect_changes()`。
