## 1. Data Model Changes (Phase 1)

- [x] 1.1 在 `macaca-proto/src/types.rs` 的 `TodoItem` 中添加 `sequence_number: u32` 字段，默认值 0
- [x] 1.2 在 `macaca-proto/src/types.rs` 中添加 `AgentTaskRef` 枚举（AllTasks / SpecificTask）
- [x] 1.3 在 `macaca-task/src/decompose.rs` 的 `DecomposedTask` 中添加 `sequence: u32` 和 `depends_on_agents: Vec<AgentTaskRef>` 字段
- [x] 1.4 更新 `TodoItem` 的 serde 反序列化，处理缺少 `sequence_number` 的旧数据（默认 0）
- [x] 1.5 编写单元测试：TodoItem 序列化/反序列化兼容性（含新旧格式）

## 2. 序号分配逻辑 (Phase 2)

- [x] 2.1 在 `macaca-task/src/todo_board.rs` 的 `TaskSpace.create_and_assign()` 中实现自动序号分配：查询当前 agent+session 最大序号，新任务从 max+1 开始
- [x] 2.2 批量创建时按 `DecomposedTask.sequence` 排序后分配递增序号
- [x] 2.3 在 `TodoStore` 中添加 `get_max_sequence_number(app_id, session_id, agent)` 查询方法
- [x] 2.4 编写单元测试：批量创建序号分配、追加任务序号递增

## 3. 顺序执行逻辑 (Phase 3)

- [x] 3.1 修改 `TaskBoard.claim_next()` 排序逻辑：按 `sequence_number` 升序取最小的 Pending 任务
- [x] 3.2 实现阻塞语义：如果最小序号任务是 Blocked/InProgress/Assigned，则 claim_next() 返回 None
- [x] 3.3 修改 `macaca-task/src/queue.rs` 的 `TaskQueue` 排序：从 BinaryHeap 改为按序号排序
- [x] 3.4 修改 `macaca-kernel/src/executor/queue.rs` 的 `ExecutionQueue` 排序策略
- [x] 3.5 确保 `WorkerLoop` 在当前任务完成前不会 claim 下一个任务
- [x] 3.6 编写单元测试：顺序 claim、Blocked 阻塞后续、Failed 后行为

## 4. 依赖管理增强 (Phase 4)

- [x] 4.1 实现 `detect_cycles(tasks: &[TodoItem])` 函数（DFS 环检测）
- [x] 4.2 在 `TaskSpace.create_and_assign()` 中调用环检测，发现环时返回错误
- [x] 4.3 增强 `unblock_dependents()`：Failed/Cancelled 依赖时 emit AnomalyDetected 而非解除阻塞
- [x] 4.4 实现跨 agent 依赖解析：`AgentTaskRef::AllTasks` → 展开为该 agent 所有 TaskId
- [x] 4.5 实现跨 agent 依赖解析：`AgentTaskRef::SpecificTask` → 按 title 匹配 TaskId
- [x] 4.6 实现 `skip_task()` API：Pending/Blocked → Cancelled + 触发依赖重评估
- [x] 4.7 编写单元测试：环检测、跨 agent 依赖解析、skip_task、unblock 增强

## 5. LLM Decomposer 增强 (Phase 5)

- [x] 5.1 修改 `LlmDecomposer` 的 system prompt，引导 LLM 输出 `sequence` 和 `depends_on_agents` 字段
- [x] 5.2 更新 `DecomposedTask` JSON 解析逻辑，支持新字段（向后兼容旧格式）
- [x] 5.3 在 decompose 结果处理中，将 `depends_on_agents` 解析为具体 TaskId
- [x] 5.4 编写集成测试：模拟 LLM 输出 → 验证分解后的任务序号和依赖关系

## 6. 数据迁移 (Phase 6)

- [x] 6.1 在 `TodoStore` 初始化时实现旧数据迁移：按 `created_at` 排序分配 sequence_number
- [x] 6.2 迁移只执行一次（用 meta key 标记已迁移）
- [x] 6.3 编写迁移测试：模拟旧格式数据 → 迁移 → 验证序号

## 7. 工具层更新 (Phase 7)

- [x] 7.1 更新 `DelegateTaskTool` schema：移除 `priority` 参数，添加 `sequence` 参数（可选，默认追加到末尾）
- [x] 7.2 更新 delegate_callback 签名，传递 sequence 而非 priority
- [x] 7.3 编写集成测试：通过工具创建任务 → 验证序号分配

## 8. 后端 API 排序 (Phase 8)

- [x] 8.1 修改 `macaca-web/src/routes.rs` 的 `list_todos` 返回结果按 `sequence_number` 升序排序
- [x] 8.2 修改 `list_agent_todos` 返回结果按 `sequence_number` 升序排序
- [x] 8.3 确保 `TodoItem` 的 JSON 序列化包含 `sequence_number` 字段

## 9. 前端 Task 面板改造 (Phase 9)

- [x] 9.1 更新 `frontend/lib/types.ts` 的 `TodoItem` 接口，新增 `sequence_number: number` 字段
- [x] 9.2 将 `TaskCard` 组件中的 `PriorityBadge` 替换为 `SequenceBadge`，显示 `#N` 格式的序号
- [x] 9.3 `AgentGroup` 组件内的任务按 `sequence_number` 升序排列
- [x] 9.4 Blocked 状态的 `TaskCard` 增加依赖提示（锁图标 + 被阻塞原因）
- [x] 9.5 移除 `PRIORITY_CONFIG` 及 `PriorityBadge` 组件（不再使用）
- [x] 9.6 前端构建验证：`npm run build` 通过

## 10. 验证与文档 (Phase 10)

- [x] 10.1 全链路集成测试：create_goal → decompose → sequential execution → dependency unblock → completion
- [x] 10.2 cargo check + cargo test 全 workspace 通过
- [x] 10.3 前端 + 后端联调验证：任务面板按序号正确显示
- [x] 10.4 更新 CLAUDE.md 中相关文档（如有必要）
