# Task Todo 管理系统：实现全自主 7×24 Agent OS 的核心架构

## 一、设计哲学

当前所有 Agent 框架（包括 OpenClaw）本质上是**被动系统**：人类发起任务 → Agent 执行 → 返回结果。即使有定时调度，Agent 仍然是"被告知做什么"。

本设计的核心转变：**Agent 主动从任务面板领取工作，完成后自主汇报并领取下一个**。Plan Agent 扮演"项目经理"角色，负责分解目标、分派任务、验证质量、智能调度。

```
传统模式: 人类 → 任务 → Agent → 结果 → 人类
本设计:   人类 → 目标 → Plan Agent → 分解 → 任务面板 → Agent 自主领取/执行/汇报 → Plan Agent 验证 → 循环
```

---

## 二、核心概念

### 2.1 三层隔离模型

```
┌─────────────────────────────────────────────────────────────┐
│                    System Level                              │
│                                                              │
│  ┌──────────────────────┐  ┌──────────────────────┐         │
│  │   Application A       │  │   Application B       │        │
│  │                        │  │                        │       │
│  │  ┌─────────────────┐  │  │  ┌─────────────────┐  │       │
│  │  │ Plan Agent /     │  │  │  │ Plan Agent /     │  │       │
│  │  │ Coordinator      │  │  │  │ Coordinator      │  │       │
│  │  │ (可访问所有面板)  │  │  │  │ (可访问所有面板)  │  │       │
│  │  └────────┬─────────┘  │  │  └────────┬─────────┘  │      │
│  │           │             │  │           │             │     │
│  │  ┌────────┼─────────┐  │  │  ┌────────┼─────────┐  │     │
│  │  │        │         │  │  │  │        │         │  │     │
│  │  ▼        ▼         ▼  │  │  ▼        ▼         ▼  │     │
│  │ [Backend] [Frontend] [Arch] │ [Agent1] [Agent2] [Agent3]│  │
│  │  面板      面板      面板│  │  面板     面板     面板  │  │
│  │  (隔离)   (隔离)   (隔离)│  │  (隔离)  (隔离)  (隔离)  │  │
│  └──────────────────────┘  └──────────────────────┘         │
│                                                              │
│  Application A 的 Agent 无法访问 Application B 的任何面板     │
└─────────────────────────────────────────────────────────────┘
```

**隔离规则：**
- Application 间完全隔离，不可跨 Application 访问
- 同 Application 内 Agent 间隔离，Agent 只能访问自己的面板
- Plan Agent / Coordinator 拥有本 Application 内所有面板的读写权限
- 系统级管理员可跨 Application 查看（审计用途）

### 2.2 Task 生命周期

```
                    Plan Agent 创建
                         │
                         ▼
                    ┌──────────┐
                    │  PENDING  │ ← 待领取（在 Agent 面板中按优先级排列）
                    └────┬─────┘
                         │ Agent 领取
                         ▼
                    ┌──────────┐
                    │ ASSIGNED  │ ← 已分配（Agent 确认领取）
                    └────┬─────┘
                         │ Agent 开始执行
                         ▼
                    ┌──────────────┐
                    │ IN_PROGRESS   │ ← 执行中（Agent 定期更新进度）
                    └────┬─────────┘
                         │ Agent 完成
                         ▼
                    ┌────────────────┐
                    │ PENDING_REVIEW │ ← 待审查（Agent 提交完成总结）
                    └────┬───────────┘
                         │ Plan Agent 验证
                    ┌────┴────┐
                    ▼         ▼
            ┌───────────┐ ┌──────────────────┐
            │ COMPLETED  │ │ NEEDS_OPTIMIZATION│ ← 需优化（附优化建议）
            └───────────┘ └────────┬─────────┘
                                   │ Agent 重新执行
                                   ▼
                              ┌──────────────┐
                              │ IN_PROGRESS   │ ← 再次执行（带优化上下文）
                              └──────────────┘
```

**额外状态：**
- `BLOCKED` — 被其他任务阻塞（依赖未完成）
- `CANCELLED` — Plan Agent 取消（需求变更或不再需要）
- `FAILED` — 多次优化仍无法完成，需要人工干预

### 2.3 角色与职责

#### Plan Agent / Coordinator

| 职责 | 说明 |
|------|------|
| 目标分解 | 接收高层目标，分解为可执行的子任务 DAG |
| 任务分派 | 根据 Agent 能力，将任务放入对应 Agent 的面板 |
| 质量验证 | 对 Agent 提交的 `PENDING_REVIEW` 任务进行验证 |
| 进度监控 | 定期检查各 Agent 面板的任务完成情况 |
| 智能调度 | 所有任务完成后，决定是否生成新任务 |
| 优化反馈 | 验证未通过时，标记 `NEEDS_OPTIMIZATION` 并给出建议 |

#### Worker Agent（Backend / Frontend / Architect 等）

| 职责 | 说明 |
|------|------|
| 任务领取 | 完成当前任务后，从面板按优先级领取下一个 |
| 任务执行 | 自主执行任务，使用可用工具 |
| 进度上报 | 执行中定期更新进度信息 |
| 完成汇报 | 完成后标记 `PENDING_REVIEW`，提交完成总结 |
| 空闲轮询 | 无任务时，按心跳间隔检查面板是否有新任务 |
| 优化执行 | 收到 `NEEDS_OPTIMIZATION` 后，根据建议重新执行 |

---

## 三、数据模型

### 3.1 Task

```rust
pub struct Task {
    pub id: TaskId,
    pub application_id: ApplicationId,
    pub assigned_agent: AgentName,        // 分配给哪个 Agent
    pub created_by: AgentName,            // 创建者（通常是 Plan Agent）

    // 任务内容
    pub title: String,                     // 简短标题
    pub description: String,               // 详细描述
    pub acceptance_criteria: Vec<String>,   // 验收标准
    pub context: Option<String>,           // 上下文（来自父任务或前序任务）

    // 状态
    pub status: TaskStatus,
    pub priority: u8,                      // 0-10，越大越紧急
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deadline: Option<DateTime<Utc>>,

    // 依赖
    pub depends_on: Vec<TaskId>,           // 前置依赖
    pub blocks: Vec<TaskId>,               // 阻塞的下游任务
    pub parent_task: Option<TaskId>,       // 父任务（DAG 结构）
    pub subtasks: Vec<TaskId>,             // 子任务

    // 执行记录
    pub progress_updates: Vec<ProgressUpdate>,
    pub completion_summary: Option<String>,       // Agent 完成时的总结
    pub review_result: Option<ReviewResult>,      // Plan Agent 的审查结果
    pub optimization_suggestions: Option<String>, // 优化建议
    pub attempt_count: u32,                       // 尝试次数
    pub max_attempts: u32,                        // 最大尝试次数（默认 3）

    // 元数据
    pub tags: Vec<String>,
    pub estimated_effort: Option<Duration>,
    pub actual_effort: Option<Duration>,
}

pub enum TaskStatus {
    Pending,            // 待领取
    Assigned,           // 已分配
    InProgress,         // 执行中
    PendingReview,      // 待审查
    NeedsOptimization,  // 需优化
    Completed,          // 已完成（验证通过）
    Blocked,            // 被阻塞
    Cancelled,          // 已取消
    Failed,             // 失败（需人工干预）
}

pub struct ReviewResult {
    pub passed: bool,
    pub feedback: String,
    pub verified_criteria: Vec<(String, bool)>,  // 每条验收标准的验证结果
    pub reviewed_at: DateTime<Utc>,
}

pub struct ProgressUpdate {
    pub timestamp: DateTime<Utc>,
    pub message: String,
    pub percentage: Option<u8>,  // 0-100
}
```

### 3.2 TaskBoard（Agent 的任务面板）

```rust
pub struct TaskBoard {
    pub application_id: ApplicationId,
    pub agent_name: AgentName,
    pub tasks: Vec<Task>,  // 按优先级排序
}

impl TaskBoard {
    /// Agent 领取最高优先级的 Pending 任务
    pub fn claim_next(&mut self) -> Option<&mut Task>;

    /// 获取所有指定状态的任务
    pub fn tasks_by_status(&self, status: TaskStatus) -> Vec<&Task>;

    /// Agent 标记当前任务为待审查
    pub fn submit_for_review(&mut self, task_id: TaskId, summary: String);

    /// 检查是否有可领取的任务
    pub fn has_pending_tasks(&self) -> bool;

    /// 获取当前正在执行的任务
    pub fn current_task(&self) -> Option<&Task>;
}
```

### 3.3 TaskSpace（Application 的任务空间）

```rust
pub struct TaskSpace {
    pub application_id: ApplicationId,
    pub boards: HashMap<AgentName, TaskBoard>,  // 每个 Agent 一个面板
    pub goal_queue: Vec<Goal>,                  // 高层目标队列
}

impl TaskSpace {
    /// Plan Agent: 分解目标为任务并分派
    pub fn decompose_and_assign(&mut self, goal: Goal, plan_agent: &PlanAgent);

    /// Plan Agent: 获取所有待审查的任务
    pub fn pending_reviews(&self) -> Vec<&Task>;

    /// Plan Agent: 审查任务
    pub fn review_task(&mut self, task_id: TaskId, result: ReviewResult);

    /// Plan Agent: 检查整体进度
    pub fn overall_progress(&self) -> ProgressSummary;

    /// Plan Agent: 决定是否需要生成新任务
    pub fn needs_new_tasks(&self) -> bool;
}
```

---

## 四、核心流程

### 4.1 Plan Agent 调度循环

```
┌──────────────────────────────────────────────────────────┐
│                  Plan Agent 主循环                         │
│                                                           │
│  loop {                                                   │
│    // 1. 检查是否有新的高层目标                               │
│    if let Some(goal) = goal_queue.pop() {                 │
│      tasks = llm_decompose(goal);        // LLM 分解       │
│      assign_to_agents(tasks);            // 分派到面板      │
│    }                                                      │
│                                                           │
│    // 2. 审查待审查的任务                                    │
│    for task in pending_reviews() {                        │
│      result = llm_verify(task);          // LLM 验证       │
│      if result.passed {                                   │
│        mark_completed(task);                              │
│        unblock_dependents(task);         // 解锁下游        │
│      } else {                                             │
│        mark_needs_optimization(task, suggestions);        │
│      }                                                    │
│    }                                                      │
│                                                           │
│    // 3. 检查整体进度，智能决策                               │
│    if all_tasks_done() {                                  │
│      if needs_further_work() {           // LLM 判断       │
│        new_tasks = plan_next_phase();    // 生成新任务      │
│        assign_to_agents(new_tasks);                       │
│      } else {                                             │
│        report_completion();              // 汇报完成        │
│        enter_idle();                                      │
│      }                                                    │
│    }                                                      │
│                                                           │
│    // 4. 检查异常（超时、失败、阻塞）                         │
│    handle_anomalies();                                    │
│                                                           │
│    sleep(check_interval);  // 默认 30 秒                   │
│  }                                                        │
└──────────────────────────────────────────────────────────┘
```

### 4.2 Worker Agent 执行循环

```
┌──────────────────────────────────────────────────────────┐
│                Worker Agent 主循环                         │
│                                                           │
│  loop {                                                   │
│    // 1. 检查面板是否有可领取的任务                           │
│    if let Some(task) = board.claim_next() {               │
│      task.status = InProgress;                            │
│                                                           │
│      // 2. 执行任务                                        │
│      result = execute_task(task);                         │
│                                                           │
│      // 3. 定期上报进度（执行中）                             │
│      task.progress_updates.push(update);                  │
│                                                           │
│      // 4. 完成后提交审查                                   │
│      if result.success {                                  │
│        task.status = PendingReview;                       │
│        task.completion_summary = summarize(result);       │
│      } else if task.attempt_count < max_attempts {        │
│        task.status = NeedsOptimization;                   │
│        task.optimization_suggestions = diagnose(error);   │
│      } else {                                             │
│        task.status = Failed;  // 需要人工干预               │
│      }                                                    │
│                                                           │
│      continue;  // 立即尝试领取下一个任务                    │
│    }                                                      │
│                                                           │
│    // 5. 无任务时，检查 NEEDS_OPTIMIZATION 的任务            │
│    if let Some(task) = board.needs_optimization_tasks() { │
│      task.status = InProgress;                            │
│      task.attempt_count += 1;                             │
│      // 带着优化建议重新执行                                 │
│      execute_with_context(task, task.optimization_suggestions);│
│      continue;                                            │
│    }                                                      │
│                                                           │
│    // 6. 真正空闲 — 等待心跳间隔后再检查                     │
│    sleep(heartbeat_interval);  // 默认 10 秒               │
│  }                                                        │
└──────────────────────────────────────────────────────────┘
```

### 4.3 端到端流程示例

```
人类输入: "为 Macaca 开发一个用户认证系统"

1. Goal 入队 → TaskSpace.goal_queue

2. Plan Agent 唤醒，分解目标:
   ├── Task A: [Architect] 设计认证系统架构（JWT/Session/OAuth）     优先级: 9
   ├── Task B: [Backend]   实现用户注册/登录 API                     优先级: 8, 依赖 A
   ├── Task C: [Backend]   实现 JWT Token 签发和验证中间件            优先级: 8, 依赖 A
   ├── Task D: [Frontend]  实现登录/注册页面                         优先级: 7, 依赖 B,C
   └── Task E: [Backend]   编写认证系统集成测试                      优先级: 6, 依赖 B,C

3. Plan Agent 分派:
   Architect 面板: [A]
   Backend 面板:   [B(blocked), C(blocked), E(blocked)]
   Frontend 面板:  [D(blocked)]

4. Architect Agent 领取 Task A，执行，提交审查
   Plan Agent 验证 → 通过 → 解锁 B, C

5. Backend Agent 领取 Task B（优先级最高），执行，提交审查
   同时 Backend 面板中 Task C 也变为 Pending
   Plan Agent 验证 B → 需要优化（缺少密码强度检查）
   Backend Agent 领取 Task C 执行...
   Backend Agent 收到 B 的优化建议，重新执行...

6. 当 B, C 都完成 → D 解锁 → Frontend Agent 领取
   当 B, C 都完成 → E 解锁 → Backend Agent 领取

7. 所有任务完成后，Plan Agent 评估:
   "认证系统基础完成，但缺少 OAuth2 第三方登录支持"
   → 决定生成新任务 F, G...

8. 最终所有任务完成，Plan Agent 汇报:
   "用户认证系统开发完成，包含注册、登录、JWT、OAuth2、集成测试"
```

---

## 五、持久化设计

### 5.1 存储 Key 设计

```
task_space/{app_id}/board/{agent_name}/tasks     → Vec<Task> (JSON)
task_space/{app_id}/goals                         → Vec<Goal> (JSON)
task_space/{app_id}/completed_tasks               → Vec<Task> (归档)
task_space/{app_id}/metrics                       → TaskMetrics (统计)
```

### 5.2 持久化策略

- **任务状态变更时立即持久化**（非定时），确保任何时刻重启都不丢任务
- Task 使用独立 key（`task/{task_id}`），避免大 JSON 的读写竞争
- 已完成任务定期归档到 `completed_tasks`，保持活跃面板精简
- 使用已有的 `RedbStore`，无需引入新存储

### 5.3 跨重启恢复

```
进程重启后:
1. 从 RedbStore 加载所有 TaskSpace
2. 恢复每个 Agent 的 TaskBoard
3. IN_PROGRESS 的任务回滚到 PENDING（Agent 需要重新领取）
4. PENDING_REVIEW 的任务保持不变（等待 Plan Agent 审查）
5. 重启 Plan Agent 调度循环
6. 重启 Worker Agent 执行循环
```

---

## 六、与现有 Macaca 架构的集成

### 6.1 新增模块

```
macaca-task-board/
  src/
    task.rs         → Task, TaskStatus, ReviewResult 数据模型
    board.rs        → TaskBoard (Agent 面板)
    space.rs        → TaskSpace (Application 任务空间)
    plan_agent.rs   → Plan Agent 调度循环
    worker_loop.rs  → Worker Agent 自主执行循环
    persistence.rs  → RedbStore 持久化
    tools.rs        → 暴露给 Agent 的 tool：claim_task, submit_review, update_progress
```

### 6.2 与现有组件的关系

| 现有组件 | 集成方式 |
|----------|----------|
| `ApplicationExecutor` | 每个 app 创建时初始化对应的 `TaskSpace` |
| `AgentRunner` | Worker 循环替换为 TaskBoard-driven 模式 |
| `ForkManager` | delegate_task 可以自动在目标 Agent 面板创建 Task |
| `HookConsumer` | Task 状态变更触发 hook 事件 |
| `RedbStore` | 复用已有的 KV 存储 |
| Web UI | 新增 Task Board 可视化面板 |

### 6.3 新增 Agent Tools

```rust
// 暴露给 Worker Agent 的工具
fn claim_task() -> Task;           // 从面板领取最高优先级任务
fn update_progress(msg: String);   // 更新当前任务进度
fn submit_for_review(summary: String); // 提交任务审查
fn list_my_tasks() -> Vec<Task>;   // 查看我的面板

// 暴露给 Plan Agent 的工具
fn create_task(agent: String, task: TaskDef) -> TaskId;  // 创建并分派任务
fn review_task(task_id: TaskId, result: ReviewResult);   // 审查任务
fn check_progress() -> ProgressSummary;                  // 检查整体进度
fn reassign_task(task_id: TaskId, new_agent: String);    // 重新分派
```

---

## 七、关键设计决策

### 7.1 为什么不用现有的 ExecutionQueue？

| 维度 | ExecutionQueue | TaskBoard |
|------|---------------|-----------|
| 触发方式 | 被动（人类/coordinator push） | 主动（Agent pull） |
| 生命周期 | 一次性执行 | 多次尝试 + 审查循环 |
| 质量控制 | 无验证 | Plan Agent 验证 |
| 持久化 | 纯内存 | RedbStore |
| 智能调度 | 无 | 依赖 DAG + 优先级 + 智能决策 |

TaskBoard 是 ExecutionQueue 的进化，但不替代它。ExecutionQueue 仍用于实时的 delegate_task 场景，TaskBoard 用于长期自主运行的任务管理。

### 7.2 Plan Agent vs Coordinator 的区别

| 维度 | Coordinator（现有） | Plan Agent（新增） |
|------|---------------------|-------------------|
| 角色 | 任务路由器 | 项目经理 |
| 决策 | "这个任务谁能做" | "还需要做什么 + 做得好不好" |
| 生命周期 | 一个请求内 | 跨请求持续运行 |
| 主动性 | 被动（等待人类输入） | 主动（自主生成和调度任务） |

Plan Agent 可以是 Coordinator 的增强，也可以是独立角色。建议先增强 Coordinator，后期可独立。

### 7.3 心跳与轮询频率

| 角色 | 检查间隔 | 检查内容 |
|------|----------|----------|
| Plan Agent | 30 秒 | 新目标、待审查任务、异常检测 |
| Worker Agent | 10 秒 | 新任务（空闲时）、优化任务 |
| 进度上报 | 60 秒 | 当前任务执行进度 |
| 死任务检测 | 5 分钟 | IN_PROGRESS 超过阈值的任务 |

---

## 八、与 autonomous-agent-os-analysis.md 的关系

本设计对应分析报告中的：
- **P3-13：LLM-based 任务分解器** → Plan Agent 的 `llm_decompose(goal)`
- **P3-14：自主目标生成 + Plan-Verify 循环** → Plan Agent 的调度循环 + 审查机制
- **K8s 声明式期望状态** → Goal 即期望状态，TaskBoard 是控制器
- **OpenFang Hands** → Worker Agent 的自主执行循环
- **Linux systemd** → Plan Agent 的死任务检测 + Worker 心跳

---

*设计时间：2026-03-22*
