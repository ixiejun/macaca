# API 与前端参考

## REST API 端点

### 聊天
| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/chat` | SSE 流 — 协调者 agentic loop 实时追踪 |
| POST | `/api/chat/stop` | 终止应用的所有进程（`{ "app_id": "uuid" }`） |

### 应用与 Agent
| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/status` | 系统状态（版本、agent 数、应用数、LLM 提供商） |
| GET | `/api/apps` | 列出所有应用 |
| GET | `/api/apps/{id}` | 应用详情 |
| GET | `/api/apps/{id}/agents` | 列出应用的 agent |
| GET | `/api/apps/{id}/agents/stream` | SSE — agent 状态更新 |

### 会话
| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/sessions` | 列出所有会话 |
| GET | `/api/sessions/{app_id}` | 应用的会话列表 |
| GET | `/api/sessions/detail/{id}` | 会话详情（含 turns + traces） |
| DELETE | `/api/sessions/detail/{id}` | 删除会话 |
| GET | `/api/sessions/stream/{id}` | SSE — 实时会话事件 |
| GET | `/api/sessions/{id}/events` | EventLog 条目（`?since=N&limit=M`） |

### 任务与目标
| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/apps/{id}/todos` | 列出任务（`?session_id=X`）— 按 sequence_number 排序 |
| GET | `/api/apps/{id}/todos/progress` | 进度摘要（各状态计数） |
| GET | `/api/apps/{id}/todos/{agent}` | Agent 的任务面板 |
| GET | `/api/apps/{id}/goals` | 列出目标（`?session_id=X`） |
| POST | `/api/apps/{id}/goals` | 创建目标（`{ "description": "..." }`） |

### 调度与技能
| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/apps/{id}/schedules` | 列出调度任务 |
| POST | `/api/apps/{id}/schedules` | 创建调度任务 |
| GET/DELETE | `/api/apps/{id}/schedules/{sid}` | 获取/删除调度 |
| PUT | `/api/apps/{id}/schedules/{sid}/toggle` | 启用/禁用 |
| GET | `/api/skills` | 列出可用技能 |

## SSE 事件类型

### 协调者事件
| 事件 | 数据 | 来源 |
|------|------|------|
| `thinking` | `{ iteration }` | 协调者 agentic loop |
| `tool_call` | `{ tool_name, tool_input }` | 协调者工具执行 |
| `tool_result` | `{ tool_name, output }` | 协调者工具结果 |
| `assistant` | `{ content }` | 中间助手文本 |
| `content` | `{ content }` | 最终响应 |
| `done` | `{ model, tokens, iterations, tools_used }` | 执行完成 |
| `error` | `{ error }` | 错误发生 |
| `stopped` | `{ reason }` | 用户取消 |

### 委派事件
| 事件 | 数据 |
|------|------|
| `fork_created` | `{ fork_id, session_id }` |
| `loop_paused` | `{ iteration, reason, session_id }` |
| `loop_resumed` | `{ session_id, task_id, success }` |
| `delegated_task_start` | `{ task_id, agent, agent_tab }` |
| `delegated_thinking` | `{ task_id, agent, event }` |
| `delegated_tool_call` | `{ task_id, agent, event }` |
| `delegated_tool_result` | `{ task_id, agent, event }` |
| `delegated_completed` | `{ task_id, agent, event }` |
| `delegated_task_complete` | `{ task_id, success, output }` |
| `delegated_task_error` | `{ task_id, error }` |

### 计划事件
| 事件 | 数据 |
|------|------|
| `plan_decision` | `{ decision_type, message, goal_id?, task_id?, agent? }` |

决策类型：`goal_ready`（目标就绪）、`review_needed`（需要审核）、`task_reviewed`（审核完成）、`goal_satisfied`（目标达成）、`goal_needs_work`（需要补充）、`goal_completed`（目标完成）、`anomaly`（异常检测）、`task_claimed`（任务认领）

## 前端组件目录

### 页面
| 组件 | 路径 | 用途 |
|------|------|------|
| 首页 | `app/page.tsx` | 应用发现网格 |
| 聊天 | `app/chat/[appId]/page.tsx` | 主工作区（~600 行） |

### 组件
| 组件 | 用途 | 关键属性 |
|------|------|----------|
| `AgentPanel` | Agent 状态面板 + TERMINATE 按钮 | `agents, appId, sessionId` |
| `ConversationTurn` | 渲染助手轮次含 trace 步骤 | `turn: ChatTurn` |
| `DelegatedAgentTrace` | 委派 agent 执行追踪 | `trace: DelegatedAgentTrace` |
| `TaskBoardModal` | 任务面板弹窗，含序号徽标 | `appId, sessionId, onClose` |
| `Sidebar` | 会话列表 + 新建会话 + 首页 | `sessions, onSelectSession, onNewChat` |
| `InputArea` | 聊天输入文本框 | `onSubmit, disabled` |
| `Message` | **死代码** — 已被 ConversationTurn 替代 | — |

### API 客户端（`lib/api.ts`）
13 个函数：`fetchStatus`、`fetchApps`、`fetchApp`、`fetchAppAgents`、`subscribeAgentStatus`、`fetchAppSessions`、`fetchSession`、`deleteSession`、`sendChat`、`subscribeSessionStream`、`stopChat`、`fetchTodos`、`fetchSessionEvents`

基础 URL：`NEXT_PUBLIC_API_BASE` 或 `window.location.hostname:3001`

### 类型定义（`lib/types.ts`）
关键接口：`TodoItem`（含 `sequence_number`）、`ChatTurn`、`DelegatedAgentTrace`、`PlanDecisionEvent`、`ChatStreamEvent`（30+ 事件类型联合）、`TodoStatus`（9 种状态）

## CSS 设计系统

### 主题："VS Code Dark+" 终端美学
- 背景：`#000`（纯黑）
- 主色调：`#7DFF9B`（绿色终端）
- 错误色：`#FF6B81`
- 紫色强调：`#C084FC`
- 字体：SFMono-Regular, monospace
- 全大写标签 + 字间距：`0.12em-0.18em`

### 核心 CSS 类
| 类名 | 用途 |
|------|------|
| `workspace-shell` | 根网格：288px 侧栏 + flex 内容，`height: 100dvh` |
| `workspace-sidebar` | 左侧栏，`overflow: hidden` |
| `workspace-main` | 主内容区，flex 列 |
| `workspace-body` | 网格：内容 + 320px agent 面板 |
| `workspace-conversation-scroll` | 可滚动消息区域 |
| `workspace-trace-block` | trace 步骤容器（thinking、工具调用） |
| `workspace-code-block` | 代码/工具输出块 |
| `workspace-assistant-summary` | 最终 OUTPUT SUMMARY 块 |
| `workspace-terminate-btn` | 红色 TERMINATE 按钮（含 hover/active 效果） |
| `workspace-agent-tabs` | Agent 标签栏（MAIN THREAD、BACKEND 等） |

### TaskBoard 样式
- `SequenceBadge`：`#N` 格式，等宽字体，绿色强调背景
- `StatusBadge`：按状态着色，可选脉冲动画
- Blocked 任务：红色边框 + 锁图标 + "WAITING ON DEPS" 文字
- Agent 分组：可折叠区段，内部按 sequence_number 排序

## EventLog 重建

页面刷新时，`get_session_by_id` 重建：
1. **Agent traces**：从 `delegated_*` EventLog 条目 → 每 agent 的 trace 数组
2. **协调者 traces**：从 `thinking`、`tool_call`、`tool_result` EventLog 条目 → trace_steps
3. **计划决策**：从独立的 plan_decision 存储

确保浏览器刷新不丢失数据 — EventLog 是唯一事实来源。
