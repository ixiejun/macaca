## 1. Preparation

- [x] 1.1 阅读 `macaca-memory` 当前 `store.rs`、`manager.rs`、`isolated.rs`、`facade.rs`、`backend.rs`、`query.rs`、`snapshot.rs`。
- [x] 1.2 对计划修改的 `MemoryManager`、`IsolatedMemoryManager`、`MemoryStore`、`MemoryRetriever` 运行 GitNexus upstream impact analysis。
- [x] 1.3 运行 baseline `cargo test -p macaca-memory`。

## 2. Core module

- [x] 2.1 新增 `macaca-memory/src/core/mod.rs`。
- [x] 2.2 新增 `MemoryScope`、`MemoryVisibility`、`MemoryIdentity`。
- [x] 2.3 新增 `MemoryWriteRequest`、`MemorySearchRequest`、`MemoryGetRequest`、`MemoryDeleteRequest`、`MemoryPrefetchRequest`、`MemoryStatusReport`。
- [x] 2.4 新增 `MemoryFacade` trait。
- [x] 2.5 新增 `MemoryRouter` 与默认 routing policy。
- [x] 2.6 新增 `MemoryProvider`、capability traits、lifecycle event DTO。

## 3. Builtin adapters

- [x] 3.1 将现有 `IsolatedMemoryManager` 包装为 `AgentPrivate` builtin adapter。
- [x] 3.2 将现有 `MemoryManager` 包装为 `SessionShared` / builtin adapter。
- [x] 3.3 新增默认 `MemoryFabricFacade`，聚合 router 与 adapters。
- [x] 3.4 更新 `lib.rs` re-export，保持旧 public API 可用。

## 4. Isolation tests

- [x] 4.1 添加 `AgentPrivate` 写入必须携带 application + agent scope 的测试。
- [x] 4.2 添加两个 agent 不能互读 private memory 的测试。
- [x] 4.3 添加同 session/project 授权 agent 可读取 `SessionShared` 的测试。
- [x] 4.4 添加 private memory 不会自动晋升 shared memory 的测试。

## 5. Compatibility

- [x] 5.1 确认旧 manager/store/vector/embedding traits 未删除。
- [x] 5.2 确认旧 facade 方法仍可编译。
- [x] 5.3 如新增替代入口，给后续迁移任务记录 deprecated 计划，不在本任务删除旧入口。

## 6. Verification

- [x] 6.1 运行 `cargo fmt`。
- [x] 6.2 运行 `cargo test -p macaca-memory`。
- [x] 6.3 运行 `cargo check -p macaca-memory -p macaca-kernel -p macaca-agent -p macaca-framework -p macaca-web`。
- [x] 6.4 运行 `openspec validate add-memory-fabric-core --strict`。
- [ ] 6.5 运行 `gitnexus_detect_changes()`。

Note: 本机 `gitnexus` CLI 支持 `status`、`impact`、`context` 等命令，但不支持 `detect_changes` 子命令；已用 `gitnexus status` 确认索引 up-to-date，并用限定范围 diff 检查本次修改文件。
