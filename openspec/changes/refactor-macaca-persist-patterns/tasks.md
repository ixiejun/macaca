## 1. Preparation

- [x] 1.1 盘点 `macaca-persist` 中 session / event / checkpoint / backend 的现有公开入口
- [x] 1.2 对首批拟修改 symbol 运行 GitNexus impact，记录 blast radius
- [x] 1.3 确认现有 crate 测试覆盖 replay / checkpoint / session store 的基线

## 2. Event Replay Iterator

- [x] 2.1 为 event log 新增 additive-first 的 `EventReplayIterator`
- [x] 2.2 保留现有读取接口，并在内部兼容复用 iterator 原语
- [x] 2.3 补 replay 顺序与 cursor 行为测试

## 3. Append Event Command

- [x] 3.1 新增 `AppendEventCommand`
- [x] 3.2 将内部 event append 流程迁到 command object
- [x] 3.3 保持现有外部追加行为与 payload 1:1 不变

## 4. Checkpoint Builder

- [x] 4.1 新增 `CheckpointBuilder`
- [x] 4.2 保留旧 checkpoint 创建入口并兼容 builder
- [x] 4.3 补 checkpoint builder 等价性测试

## 5. Backend Strategy Boundary

- [x] 5.1 抽出 `PersistBackend` trait 或等价 backend strategy 边界
- [x] 5.2 让 `RedbStore` 成为默认实现并通过 trait 接入
- [x] 5.3 不在本轮引入新的 backend

## 6. Session Snapshot / Memento

- [x] 6.1 新增 session snapshot / memento 原语
- [x] 6.2 明确 session metadata、event cursor、checkpoint 引用的快照语义
- [x] 6.3 不改变现有上层恢复语义

## 7. Verification

- [x] 7.1 运行 `cargo check -p macaca-persist`
- [x] 7.2 运行 `cargo test -p macaca-persist -- --nocapture`
- [x] 7.3 运行 workspace `cargo check`
- [x] 7.4 如实现范围需要，补最小 replay / refresh-restore 契约验证
- [x] 7.5 更新 checklist，仅在真实完成后勾选
