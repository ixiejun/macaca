## 1. Preparation

- [x] 1.1 盘点 `macaca-web` / `macaca-task` / `macaca-kernel` 对 `macaca-persist` 的真实消费点
- [x] 1.2 对首批拟修改 symbol 运行 GitNexus impact，记录 blast radius 与风险
- [x] 1.3 校验现有 `macaca-persist` additive-first contract 与上层现状的一致性

## 2. Web EventLog Migration

- [x] 2.1 将 `macaca-web` 的 durable EventLog 写入迁移到 `AppendEventCommand`
- [x] 2.2 将 `macaca-web` 的历史恢复路径迁移到 `EventReplayIterator`
- [x] 2.3 保持 SSE 推送、刷新恢复、历史重建行为 1:1 不变

## 3. Backend Strategy Migration

- [ ] 3.1 将 `macaca-web` 共享 session store 类型收敛到 `PersistBackend` / `PersistStore`
- [x] 3.2 将 `macaca-task::TodoStore` 迁移到 `PersistBackend`
- [x] 3.3 将 `macaca-task::TaskScheduler` 迁移到 `PersistBackend`
- [x] 3.4 将 `macaca-kernel::AuditLogger` 迁移到 `PersistBackend`
- [x] 3.5 保持 `RedbStore` 作为默认实现，不引入新 backend

## 4. Builder / Memento Adoption Boundary

- [x] 4.1 审计 `CheckpointBuilder` / `SessionSnapshot` 是否存在真实上层消费点
- [x] 4.2 对无真实消费点的原语明确保留 additive-first，不制造伪调用
- [x] 4.3 如发现真实消费点，仅做最小迁移并保持行为不变

## 5. Verification

- [x] 5.1 运行 `cargo check -p macaca-web -p macaca-task -p macaca-kernel`
- [x] 5.2 运行 `cargo test -p macaca-web -- --nocapture`
- [x] 5.3 运行 `cargo test -p macaca-task -- --nocapture`
- [x] 5.4 运行 `cargo test -p macaca-kernel -- --nocapture`
- [x] 5.5 运行 workspace `cargo check`
- [x] 5.6 运行 GitNexus `detect_changes(scope: "all")`
- [x] 5.7 更新 checklist，仅在真实完成后勾选
