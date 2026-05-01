## 1. Spec And Baseline

- [x] 1.1 创建 `refactor-macaca-task-patterns` proposal / design / tasks / delta spec
- [x] 1.2 盘点 `macaca-task` 当前 public API 与上层调用点，明确 deprecated 迁移范围
- [x] 1.3 为将修改的核心 symbol 运行 GitNexus impact，并记录风险

## 2. Lifecycle Policy

- [x] 2.1 为 `TaskBoard` / `TaskSpace` 当前状态流转补齐表驱动回归测试
- [x] 2.2 新增 `TodoLifecyclePolicy` 与默认实现
- [x] 2.3 将 claim/start/review/retry/skip/fail 相关状态转移收口到 policy
- [x] 2.4 保留旧 public task action 入口，标记 `deprecated` 并委托到新 canonical API
- [x] 2.5 迁移仓库内已知旧入口调用面到新 lifecycle API

## 3. Dependency Resolver

- [x] 3.1 新增 `TaskDependencyResolver` 与默认实现
- [x] 3.2 将 `create_and_assign` 的 blocked 判定收口到 resolver
- [x] 3.3 将 `claim_next` 的 parent goal gating 收口到 resolver
- [x] 3.4 将 completed 后 dependents 解锁收口到 resolver
- [x] 3.5 保留旧 public dependency-sensitive 入口，标记 `deprecated` 并委托到新 canonical API

## 4. Loop Templates

- [x] 4.1 将 `PlanLoop::run` 拆成显式 template step，保持 `PlanEvent` 不变
- [x] 4.2 提供新的 canonical constructor / loop API，并将旧 constructor 标记 `deprecated`
- [x] 4.3 将 `WorkerLoop::run` 拆成显式 template step，保持 `WorkerEvent` 不变
- [x] 4.4 提供新的 canonical constructor / loop API，并将旧 constructor 标记 `deprecated`
- [x] 4.5 迁移仓库内 `PlanLoop::new` / `WorkerLoop::new` 的已知调用面

## 5. Verification

- [x] 5.1 运行 `openspec validate refactor-macaca-task-patterns --strict`
- [x] 5.2 运行 `cargo test -p macaca-task -- --nocapture`
- [x] 5.3 运行 `cargo check -p macaca-task`
- [x] 5.4 运行 `cargo check -p macaca-tools -p macaca-web -p macaca-integration-tests`
- [x] 5.5 运行 workspace `cargo check`
- [x] 5.6 运行 `gitnexus_detect_changes(scope: "all")`
- [x] 5.7 更新 tasks.md，确认所有任务与真实完成状态一致
