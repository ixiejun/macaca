# 即时委托 vs 项目级任务：LLM 驱动的智能路由设计

## 核心原则

**不做硬编码分类，不加额外分类步骤。** Coordinator 同时拥有两套工具，LLM 根据任务性质自然选择。

```
用户输入
    │
    ▼
Coordinator (LLM)
    │
    ├─ 判断为简单任务 → delegate_task (推模型，即时执行)
    │   例: "写个hello world" / "查看文件内容" / "修个typo"
    │
    └─ 判断为复杂项目 → create_goal (拉模型，Todo系统接管)
        例: "开发用户认证系统" / "重构数据库层" / "搭建CI/CD"
```

## 为什么不需要额外分类器

1. **LLM 天然擅长判断任务复杂度** — 这本质上是一个语义理解问题
2. **工具选择就是分类结果** — LLM 选择 `delegate_task` 还是 `create_goal`，隐式完成了分类
3. **避免双重 LLM 调用** — 不需要先分类再执行，一步到位
4. **灵活性** — LLM 可以综合考虑上下文、历史对话、项目状态来做判断

## Coordinator 的 System Prompt 引导

在 Coordinator 的 persona 中添加路由决策指南：

```markdown
## 任务路由决策

你拥有两种任务执行模式，根据任务性质选择：

### 即时委托 (delegate_task)
适用于：
- 单步或少步即可完成的任务
- 不需要多 agent 协作的任务
- 不需要质量验证的快速操作
- 明确知道由哪个 agent 执行

例如：写一个函数、修复一个 bug、查询信息、读写文件

### 项目级任务 (create_goal)
适用于：
- 需要分解为多个子任务的复杂工作
- 需要多个 agent 协作（后端+前端+架构）
- 需要质量验证和审查的重要交付
- 需要依赖管理（任务 B 依赖任务 A 完成）
- 预计需要多轮迭代才能完成

例如：开发一个完整功能、重构系统架构、搭建项目框架

### 判断标准
不要纠结于精确分类。一般规则：
- 如果你能用一句话描述完要做的事 → delegate_task
- 如果你需要列出步骤才能说清楚 → create_goal
- 不确定时，偏向 create_goal（宁可多验证，不要漏检查）
```

## Coordinator 的完整工具集

```
# 即时执行工具（现有）
- delegate_task      → 直接委托给指定 agent 执行
- get_task_result    → 获取委托结果
- list_agents        → 列出可用 agent
- shell              → 直接执行命令
- file_read / file_write → 直接文件操作

# 项目管理工具（新增）
- create_goal        → 提交高层目标，触发 Plan 循环
- create_todo        → 创建任务并分派给指定 agent
- review_todo        → 审查 agent 提交的完成任务
- check_todo_progress → 检查整体进度

# 自用工具（新增）
- claim_task         → 从自己的面板领取任务
- start_task         → 开始执行任务
- submit_task_for_review → 提交审查
- list_my_tasks      → 查看自己的任务面板
```

## 两种模式的完整流程对比

### 即时委托流程（现有，不变）

```
用户: "用Go写个hello world"
Coordinator:
  1. thinking → 简单任务，直接委托
  2. delegate_task(agent="backend", prompt="写一个Go hello world API")
  3. 等待 backend 完成（Fork-Join）
  4. 收到结果，汇总给用户
  5. 结束

耗时: 1-3 分钟
人工干预: 0
```

### 项目级任务流程（新增）

```
用户: "开发一个完整的用户认证系统，包括注册、登录、JWT、OAuth"
Coordinator:
  1. thinking → 复杂项目，需要分解
  2. create_goal(description="开发完整的用户认证系统...")
  3. 告知用户 "已创建项目目标，正在分解任务..."

Plan 循环自动接管:
  4. PlanLoop 检测到新 goal
  5. 唤醒 Coordinator 进行分解 →
     create_todo(agent="architect", title="设计认证架构", priority=9)
     create_todo(agent="backend", title="实现注册API", depends_on=[architect_task])
     create_todo(agent="backend", title="实现JWT中间件", depends_on=[architect_task])
     create_todo(agent="frontend", title="实现登录页面", depends_on=[backend_tasks])
     ...

Worker Agent 自主执行:
  6. Architect agent 心跳 → 发现面板有任务 → claim → 执行 → submit_for_review
  7. PlanLoop 唤醒 Coordinator review → 通过 → 解锁下游任务
  8. Backend agent 心跳 → 发现任务解锁 → claim → 执行 → submit
  9. ...循环直到所有任务完成

  10. PlanLoop: 所有任务完成 → 唤醒 Coordinator 总结
  11. Coordinator 汇总给用户

耗时: 10-60 分钟（自动）
人工干预: 0（除非 task failed 需要人工介入）
```

## 边界情况处理

### 中间复杂度的任务

```
用户: "给API加上分页功能"
```

这个任务处于中间地带。LLM 可能选择：
- `delegate_task` — 如果觉得一个 backend agent 就能搞定
- `create_goal` — 如果觉得需要修改多个文件、加测试、改前端

**两种选择都是合理的。** 这正是让 LLM 判断的优势——它能根据当前项目上下文（已有多少代码、API 结构复不复杂）做出更好的决定。

### 用户明确指定模式

```
用户: "帮我规划一下如何实现分页功能"  → 明确想要规划 → create_goal
用户: "直接给API加个分页"           → 明确想要快速执行 → delegate_task
```

LLM 从用户措辞中自然理解意图，无需额外规则。

### 项目进行中用户追加需求

```
# 项目正在进行中（Todo 面板有活跃任务）
用户: "对了，还要加上邮件验证功能"
```

Coordinator 应该：
1. 检查当前项目状态（`check_todo_progress`）
2. 追加新任务到现有项目（`create_todo`）
3. 而不是创建新 goal

这也是 LLM 自然能做到的——它看到上下文中有进行中的项目，就会选择追加而非新建。

## 与现有架构的集成

### 不需要改动的部分

- `delegate_task` 工具和 Fork-Join 流程**完全不变**
- `ExecutionQueue` 和 `worker_loop` 继续服务即时委托
- SSE 实时流、session 持久化等基础设施不变

### 需要改动的部分

1. **Coordinator persona** — 添加路由决策指南
2. **Tool 注册** — Coordinator 获得 create_goal/create_todo/review_todo/check_todo_progress
3. **PlanLoop 消费者** — 将 PlanEvent 转化为 Coordinator 的 agent 执行调用

### 统一的用户体验

从用户角度，体验完全一致：
- 始终在同一个 main thread 对话
- 简单任务秒回结果
- 复杂任务自动规划执行，定期汇报进度
- 无需用户选择"模式"

区别仅在内部：
- 简单任务走 delegate → fork-join → 同步返回
- 复杂任务走 goal → decompose → todo boards → async review → 异步完成

---

*设计时间：2026-03-22*
