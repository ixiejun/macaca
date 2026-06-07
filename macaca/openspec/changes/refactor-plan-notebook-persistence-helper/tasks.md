## 1. Implementation

- [x] 1.1 在 `loop_manager.rs` 中新增 PlanNotebook 持久化 helper，统一执行 scope -> load -> mutate -> save
- [x] 1.2 将 `planner_notebook_mark_decomposition` 切换为 helper，保持 notebook 内容不变
- [x] 1.3 将 `planner_notebook_mark_review` 切换为 helper，保持 notebook 内容不变
- [x] 1.4 保持 helper 仅在 web 层，不下沉到 `macaca-framework/src/plan.rs`

## 2. Tests

- [x] 2.1 添加或更新 unit test，覆盖 decomposition notebook 写入内容保持不变
- [x] 2.2 添加或更新 unit test，覆盖 review notebook 写入内容保持不变
- [x] 2.3 添加或更新 unit test，覆盖 planner session scope fallback 行为保持不变

## 3. Verification

- [x] 3.1 运行 `openspec validate refactor-plan-notebook-persistence-helper --strict`
- [x] 3.2 运行 PlanNotebook helper 相关单元测试
- [x] 3.3 运行 `cargo check -p macaca-web`
- [x] 3.4 运行 GitNexus `detect_changes(scope=staged)` 并确认影响范围符合本次局部重构
