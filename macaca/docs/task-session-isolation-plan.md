# Task Session 隔离实施方案

> 日期：2026-04-01
> 状态：待确认

---

## 问题

当前 Task/Goal 按 `app_id` 隔离，同一 app 的所有 session 共享同一套任务。Session A 创建的 goal/task 在 Session B 中也可见，新建 session 看到旧 session 的残留任务。

## 推荐方案：共享 Loop + 数据层 Session 隔离（Option B）

**核心思路**：`session_id` 通过**数据**（TodoItem/TodoGoal 字段 + storage key）隔离，**不改变** PlanLoop/WorkerLoop 拓扑。

- PlanLoop 仍然 per-app（一个 app 一个 PlanLoop）
- WorkerLoop 仍然 per-app（一个 agent 一个 WorkerLoop）
- 但 TaskSpace/TaskBoard 的查询按 session_id 过滤
- 每个 session 的 coordinator 创建的 goal 自动携带 session_id
- PlanLoop 弹出 goal 后，分解的 tasks 继承该 goal 的 session_id
- WorkerLoop claim task 时，task 已经带有 session_id

**优势**：
- 不增加后台任务数量（N_sessions 个 session 共享同一套 loops）
- Scheduler 创建的 app 级任务用 `session_id: None`，自然不属于任何 session
- 旧数据 `session_id: None`，不影响新 session

### Key Schema 变化

```
当前:
  todo/{app_id}/{agent}/{task_id}
  goal/{app_id}/{goal_id}

改为:
  todo/{app_id}/{session_id}/{agent}/{task_id}   (session_id 有值时)
  todo/{app_id}/_global_/{agent}/{task_id}        (scheduler 等无 session 场景)
  goal/{app_id}/{session_id}/{goal_id}
  goal/{app_id}/_global_/{goal_id}
```

---

## 实施阶段

### Phase 1: 数据模型 — TodoItem/TodoGoal 加 session_id (0.5h)

| 文件 | 修改 |
|------|------|
| `macaca-proto/src/types.rs` | TodoItem 加 `#[serde(default)] pub session_id: Option<String>`，TodoGoal 同理 |
| `TodoItem::new()` | 增加 `session_id` 参数 |
| `TodoGoal::new()` | 增加 `session_id` 参数 |

### Phase 2: TodoStore — Session-aware key + 查询 (2h)

| 文件 | 修改 |
|------|------|
| `macaca-task/src/todo_store.rs` | Key helpers 加入 session_id 维度 |
| 同上 | 新增 `list_all_todos_for_session(app_id, session_id)` |
| 同上 | 新增 `list_goals_for_session(app_id, session_id)` |
| 同上 | 新增 `pop_pending_goal_for_session(app_id, session_id)` |
| 同上 | `save_todo` 根据 item.session_id 选择 key format |
| 同上 | `rollback_in_progress` 支持 session 过滤 |

### Phase 3: TaskBoard/TaskSpace 加 session_id (1.5h)

| 文件 | 修改 |
|------|------|
| `macaca-task/src/todo_board.rs` | `TaskBoard::new(app_id, agent, session_id, store)` |
| 同上 | `TaskSpace::new(app_id, session_id, store)` |
| 同上 | 所有方法通过 session_id 过滤 |
| 同上 | `create_and_assign` 设 `item.session_id = self.session_id` |
| 同上 | `push_goal` 设 `goal.session_id = self.session_id` |

### Phase 4: PlanLoop/WorkerLoop 透明传递 (0.5h)

不需要结构改动。PlanLoop/WorkerLoop 的 TaskSpace/TaskBoard 已经携带 session_id。

| 关注点 | 处理 |
|--------|------|
| PlanLoop 的 TaskSpace | `ensure_plan_and_worker_loops` 中用 `session_id: None`（扫描全部 session 的 goals） |
| WorkerLoop 的 TaskBoard | 同上，用 `session_id: None`（claim 全部 session 的 tasks） |
| 分解出的 tasks | 继承 goal 的 session_id（通过 TaskSpace.create_and_assign） |

### Phase 5: Tool 构建 + REST API 传 session_id (3h)

| 文件 | 修改 |
|------|------|
| `routes.rs` run_agentic_stream | `TaskSpace::new(app_id, Some(session_id), store)` |
| `routes.rs` ensure_plan_and_worker_loops | PlanLoop/WorkerLoop 用 `session_id: None` |
| `routes.rs` list_todos/get_progress/list_goals | 接受 `?session_id=X` 查询参数 |
| `routes.rs` create_goal HTTP | 传递 session_id |
| `agent_runner.rs` build_agent_toolset | TaskBoard/TaskSpace 构造传入 session_id |
| `macaca-tools/src/todo.rs` | 无需改动（tools 通过 space/board 间接获得 session 作用域） |

### Phase 6: 前端 (1h)

| 文件 | 修改 |
|------|------|
| `api.ts` | `fetchTodos(appId, sessionId?)` 加查询参数 |
| `types.ts` | `TodoItem` 加 `session_id?: string` |
| `TaskBoardModal.tsx` | 接受 `sessionId` prop |
| `AgentPanel.tsx` | 传递当前 sessionId 给 TaskBoardModal |

### Phase 7: 兼容性 (0h)

- 旧数据 `session_id: None`（`#[serde(default)]`），反序列化无报错
- 旧 session 的 tasks 通过 `list_all_todos(app_id)` 仍可查看（无 session 过滤）
- 新 session 查看时只看到自己的 tasks
- Scheduler 创建的 tasks 用 `session_id: None`，不干扰任何 session

---

## 关键决策

| 决策 | 选择 | 理由 |
|------|------|------|
| PlanLoop/WorkerLoop 拓扑 | 共享（per-app） | 避免后台任务爆炸 |
| session_id 类型 | `Option<String>` | 兼容旧数据 + scheduler |
| PlanLoop 作用域 | `session_id: None` | 需要跨 session 扫描 goals |
| WorkerLoop 作用域 | `session_id: None` | 需要跨 session claim tasks |
| 数据层隔离 | Key schema 加 session_id | TodoItem 自身也携带 session_id |
| REST API | 可选 `?session_id=X` 查询参数 | 兼容旧 API |

## 风险

| 风险 | 缓解 |
|------|------|
| PlanLoop 跨 session 扫描时混淆 | goal 和 task 都带 session_id，分解时继承 |
| WorkerLoop claim 跨 session task | task 自带 session_id，完成后回写正确的 key |
| 旧数据无 session_id | `Option<String>` + `#[serde(default)]` 自动兼容 |
| 大量 session 导致 key 爆炸 | RedbStore 前缀扫描性能足够，可加 TTL 清理 |

## 总工作量

约 **8.5 小时**，修改 ~10 个文件，~280 行代码变更。

---

*设计完成：2026-04-01*
