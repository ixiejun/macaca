## 1. Implementation

- [x] 1.1 在 `loop_manager.rs` 中新增 planner framework 调用模式/配置，用于区分 decomposition、review、follow-up
- [x] 1.2 在 `loop_manager.rs` 中新增 helper，统一处理 planner Working -> framework builder -> reply -> completed/failed -> Idle 模板
- [x] 1.3 将 `PlanEvent::GoalReady` decomposition 分支切换为 helper，保持 prompt、goal id、goal context 和日志行为不变
- [x] 1.4 将 `PlanEvent::ReviewNeeded` review 分支切换为 helper，保持 `build_worker_agent`、task id 和日志行为不变
- [x] 1.5 将 `GoalEvaluation::NeedsMoreWork` follow-up 分支切换为 helper，保持 prompt、goal id、goal context 和日志行为不变

## 2. Tests

- [x] 2.1 添加或更新 unit test，覆盖 decomposition/follow-up 使用 goal-aware traced builder 配置
- [x] 2.2 添加或更新 unit test，覆盖 review 继续使用 worker traced builder 配置
- [x] 2.3 添加或更新 unit test，覆盖三类 planner 调用的成功/失败日志文案保持不变

## 3. Verification

- [x] 3.1 运行 `openspec validate refactor-planner-framework-call-helper --strict`
- [x] 3.2 运行 planner framework helper 相关单元测试
- [x] 3.3 运行 `cargo check -p macaca-web`
- [x] 3.4 运行 GitNexus `detect_changes(scope=staged)` 并确认影响范围符合本次局部重构
