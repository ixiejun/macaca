## 1. PlanLoop ReviewNeeded 去重 (P0)

- [x] 1.1 在 `PlanLoop::run()` 中添加 `reviewed_tasks: HashSet<TaskId>` 状态
- [x] 1.2 ReviewNeeded emit 前检查 HashSet，只 emit 新任务
- [x] 1.3 每个心跳周期开始时，清理已不再是 PendingReview 的旧记录
- [x] 1.4 编写单元测试：同一任务不重复 emit

## 2. Worker 提交后唤醒 PlanLoop (P1)

- [x] 2.1 在 WorkerLoop consumer 的 `submit_for_review` 调用后，调用 PlanLoopWaker::wake()
- [x] 2.2 确保 PlanLoopWaker 在 WorkerLoop consumer 作用域可用

## 3. Review 完成后唤醒 WorkerLoop (P1)

- [x] 3.1 在 PlanEvent consumer 的 ReviewNeeded delegate 完成后，调用 WorkerLoopWaker::wake()
- [x] 3.2 确保 review 通过时被 unblock 的任务能立即被 WorkerLoop claim

## 4. Review 事件广播 (P1)

- [x] 4.1 PlanEvent consumer 中，planner 完成 review delegate 后，广播 `task_reviewed` SSE 事件
- [x] 4.2 同时写入 EventLog（通过 broadcast_to_app_sessions 持久化）
- [x] 4.3 事件包含：task_id, agent, title, decision_type="task_reviewed"

## 5. 验证 (P2)

- [x] 5.1 cargo check + cargo test 全 workspace 通过
- [ ] 5.2 手动测试：提交 review → planner 只审核一次 → 后续任务立即启动
- [ ] 5.3 前端刷新后 review 事件正确加载
