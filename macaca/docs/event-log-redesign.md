# Event Trace 持久化架构重设计

> 日期：2026-03-30
> 状态：设计完成，待实现

---

## 一、核心问题

当前架构是 **SSE-First**（SSE 优先）：事件产生后注入 SSE channel 发给浏览器，持久化只是副作用。

```
当前: Event → SSE Channel → Browser (primary)
                  ↓ (side effect)
            Memory Collector → Periodic Save → DB
```

这个架构的根本假设是"浏览器始终在线"，与 Macaca 作为 7×24 自主运行系统的定位完全矛盾。

### 根因分解

1. **没有 Event Store 抽象**：事件直接注入 SSE channel，DB 中存储的只是快照/摘要
2. **多写入路径无协调**：5 条独立写入路径操作不同 key，语义重叠但不一致
3. **Monolithic Session 存储**：整个 session 作为大 JSON blob 存取，每次 read-modify-write
4. **in-memory 和 persistent 状态的二元性**：`sessions` (HashMap) 和 `session_store` (RedbStore) 同步不完整

---

## 二、当前架构图

```
┌──────────────────────────────────────────────────────────────────────┐
│                        事件产生层                                     │
│                                                                      │
│  Coordinator (run_agentic_stream)        Executor Worker Loop        │
│  ├─ thinking, tool_call, tool_result     ├─ TaskStarted              │
│  ├─ assistant, content, done             ├─ AgentEvent (thinking,    │
│  ├─ loop_paused, loop_resumed            │   tool_call, tool_result, │
│  ├─ fork_created, stopped                │   assistant, cc_trace,    │
│  └─ cc_trace                             │   completed)              │
│                                          ├─ TaskCompleted/Failed     │
│  PlanLoop / WorkerLoop                   └─ HookEvent (fork_*)       │
│  └─ plan_decision events                                             │
└─────────────────────┬──────────────────────────┬─────────────────────┘
                      │                          │
                      ▼                          ▼
┌─────────────────────────────┐   ┌──────────────────────────────────┐
│  mpsc::Sender<SSE Event>    │   │  broadcast::Sender<ExecutorEvent>│
│  (tx — coordinator专用)      │   │  (event_broadcast — 多订阅者)     │
│                             │   │                                  │
│  tx → bridge_task           │   │  订阅者1: SSE stream (浏览器)      │
│       ↓                     │   │  订阅者2: event_collector_handle  │
│  sse_tx (Arc<RwLock<>>)     │   │    (独立于SSE的trace收集器)        │
│  (hot-swappable on refresh) │   │                                  │
│       ↓                     │   │  ⚠ Lagged → 事件静默丢弃          │
│  stream_rx → SSE stream     │   │                                  │
└──────────┬──────────────────┘   └────────────┬─────────────────────┘
           │                                   │
           ▼                                   ▼
┌──────────────────────────────────────────────────────────────────────┐
│                      内存收集层                                       │
│                                                                      │
│  AgentTraceCollector (纯内存 HashMap)                                 │
│  ├─ traces: RwLock<HashMap<agent, Vec<AgentTrace>>>                  │
│  └─ task_to_agent: RwLock<HashMap<task_id, agent>>                   │
│                                                                      │
│  ⚠ 两个写入者同时操作同一个 collector:                                  │
│    1. event_collector_handle (独立于SSE, 来自broadcast订阅)            │
│    2. SSE stream 内的 collector_for_stream                            │
│                                                                      │
│  trace_steps / cc_trace_steps (Vec, 仅在 run_agentic_stream 栈上)    │
│  ⚠ 进程崩溃 = 全部丢失                                                │
└──────────────────────────┬───────────────────────────────────────────┘
                           │
                           ▼
┌──────────────────────────────────────────────────────────────────────┐
│                      持久化层 (5条独立写入路径)                         │
│                                                                      │
│  路径1: periodic_saver (每1秒)                                       │
│    collector.get_all() → save_agent_traces()                         │
│    → key: "agent_traces/{session_id}"                                │
│    ⚠ 全量覆盖写，不是追加                                              │
│                                                                      │
│  路径2: persist_running_snapshot (每次tool_call/tool_result)          │
│    → key: "session/{session_id}" (read-modify-write)                 │
│    ⚠ 不写 agent_traces (避免竞态)                                     │
│    ⚠ 但写 trace_steps/cc_trace_steps                                 │
│                                                                      │
│  路径3: 最终完成时的 session save                                     │
│    → key: "session/{session_id}" (完整覆盖)                           │
│    ⚠ 与 periodic_saver 的时序竞态                                     │
│                                                                      │
│  路径4: save_plan_decision (每次PlanEvent)                            │
│    → key: "plan_decisions/{app_id}" (read-append-write)              │
│    ⚠ 按 app_id 聚合，不按 session_id                                  │
│                                                                      │
│  路径5: update_session_realtime (每次ExecutorEvent)                   │
│    → key: "session/{session_id}" (仅更新status)                      │
│    ⚠ 不写 traces，仅状态字段                                          │
│                                                                      │
│  所有路径 → RedbStore.set() (spawn_blocking, 每次独立事务)            │
└──────────────────────────────────────────────────────────────────────┘
```

---

## 三、已识别的 9 个数据丢失场景

### 场景 A: broadcast channel Lagged（高）

- **位置**: `routes.rs:1549`, `routes.rs:2217-2219`
- **机制**: `broadcast` channel 容量 4096。消费者处理跟不上时，`Lagged(n)` 表示 n 条事件永久丢弃。
- **影响**: SSE stream 和 `event_collector_handle` 中丢失的事件永远无法恢复。在高频 tool_call 场景（如 claude_code_execute 内部大量步骤）中真实发生。

### 场景 B: 浏览器刷新时 SSE stream 事件窗口丢失（中）

- **位置**: `routes.rs:1506-1516`（hot-swap）
- **机制**: 旧 SSE stream 关闭到新 stream 接收之间有时间窗口。在 hot-swap 完成前由 coordinator 发送到旧 `tx` 的事件丢失。
- **影响**: 几毫秒窗口，但理论上可能丢失正好在刷新时产生的事件。

### 场景 C: Coordinator 主线程 trace_steps 仅在内存中（高）

- **位置**: `routes.rs:2632-2633`
- **机制**: `trace_steps` 和 `cc_trace_steps` 是函数栈上的 `Vec`。`persist_running_snapshot` 是 fire-and-forget（`tokio::spawn`），不等待完成。
- **影响**: 进程崩溃 = 最近一次 snapshot 之后的 trace_steps 全部丢失。

### 场景 D: AgentTraceCollector 双写竞态（高）

- **位置**: `routes.rs:1791`（event_collector_handle 写入）, `routes.rs:2158-2183`（SSE stream 内写入）
- **机制**: 同一个 `Arc<AgentTraceCollector>` 被两个异步任务同时操作，从同一个 broadcast channel 的不同 Receiver 读取。
- **影响**: AgentTrace 中可能有重复的 steps，或某些 steps 在一个 consumer 中 lag 丢失而在另一个中正常处理。

### 场景 E: periodic_saver 全量覆盖写（低）

- **位置**: `routes.rs:1777-1786`, `routes.rs:938-951`
- **机制**: `save_agent_traces` 是 `store.set(key, data)` 全量覆盖。当前 collector 只增不清所以等效追加，但未来修改可能导致数据丢失。

### 场景 F: plan_decision 按 app_id 聚合（中）

- **位置**: `routes.rs:982-996`
- **机制**: 存储在 `plan_decisions/{app_id}` 下，所有 session 看到同一个 app 的全部 decisions。无法区分属于哪次对话，列表无限增长。

### 场景 G: Session 最终保存竞态（低）

- **位置**: `routes.rs:1881-1986`, `routes.rs:754`（SESSION_LOCKS）
- **机制**: 当前分离存储设计（session key vs agent_traces key）避免了最严重竞态。

### 场景 H: 浏览器刷新后 SSE 不可重放（高）

- **位置**: `routes.rs:1483-1567`
- **机制**: 刷新后只能接收新事件，之前的 thinking/tool_call/tool_result 序列全部丢失。`get_session_by_id` 可从 DB 加载 StoredTurn 但不是 SSE 重放。
- **影响**: 用户体验核心问题。

### 场景 I: broadcast_to_app_sessions try_send 失败（低）

- **位置**: `routes.rs:3238`
- **机制**: `try_send` 非阻塞，channel 满时 plan_decision 事件丢弃。

---

## 四、新架构设计

### 核心原则

- ALL events 在产生时**立即持久化**（不是 SSE 的副作用）
- 持久化**独立于任何客户端连接**
- 前端从**持久化存储**拉取数据（不从 SSE 内存）
- SSE 降级为**通知机制**（"有新事件了"），不是数据源
- 浏览器刷新 = 从 DB 重新拉取（完整，零数据丢失）

### 架构图

```
┌──────────────────────────────────────────────────────────────────────┐
│                        事件产生层 (不变)                               │
│                                                                      │
│  Coordinator          Executor Worker        PlanLoop/WorkerLoop     │
│  ├─ thinking          ├─ TaskStarted         ├─ GoalReady            │
│  ├─ tool_call         ├─ AgentEvent          ├─ ReviewNeeded         │
│  ├─ tool_result       ├─ TaskCompleted       ├─ AllTasksDone         │
│  ├─ content/done      ├─ TaskFailed          └─ AnomalyDetected     │
│  └─ loop_paused/      └─ HookEvent                                   │
│     resumed/fork                                                     │
└────────────┬──────────────────┬──────────────────────┬───────────────┘
             │                  │                      │
             ▼                  ▼                      ▼
┌──────────────────────────────────────────────────────────────────────┐
│                   EventLog (新增，单一写入点)                          │
│                                                                      │
│  pub struct EventLog {                                               │
│      store: Arc<RedbStore>,                                          │
│      seq_counters: DashMap<String, AtomicU64>,  // per-session seq   │
│      notify: broadcast::Sender<(String, u64)>,  // (session_id, seq)│
│  }                                                                   │
│                                                                      │
│  impl EventLog {                                                     │
│      async fn append(&self, session_id, event) -> u64;  // 返回 seq │
│      async fn query(&self, session_id, since, limit) -> Vec<Event>; │
│      async fn latest_seq(&self, session_id) -> u64;                 │
│  }                                                                   │
│                                                                      │
│  Storage: events/{session_id}/{seq:08d} → EventEntry JSON            │
│           (append-only, 无 read-modify-write)                        │
│                                                                      │
│  ⚡ append() 内部: seq++ → store.set() → notify.send()              │
│     (持久化在通知之前，保证 at-least-once)                             │
└────────────────────────────────┬─────────────────────────────────────┘
                                 │ notify: (session_id, latest_seq)
                                 ▼
┌──────────────────────────────────────────────────────────────────────┐
│                   SSE 通知层 (精简)                                    │
│                                                                      │
│  SSE stream per session:                                             │
│    收到 notify → 发送 { event: "update", seq: N }                     │
│                                                                      │
│  前端收到 update → fetch GET /api/sessions/{id}/events?since=last    │
│    → 渲染新事件                                                       │
│                                                                      │
│  浏览器刷新 = fetch GET /api/sessions/{id}/events?since=0            │
│    → 完整重放所有事件                                                  │
│    → 然后订阅 SSE 获取后续通知                                         │
└──────────────────────────────────────────────────────────────────────┘
```

### EventEntry 数据结构

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEntry {
    pub seq: u64,
    pub timestamp: DateTime<Utc>,
    pub session_id: String,
    pub event_type: String,        // "thinking", "tool_call", "delegated_tool_call", "plan_decision", ...
    pub source: String,            // "coordinator", "executor:backend", "plan_loop"
    pub payload: serde_json::Value, // 事件具体数据
}
```

### Storage Key Schema

```
events/{session_id}/{seq:08d}  →  EventEntry JSON  (append-only)
```

- 每个事件独立一个 key，永不 read-modify-write
- `seq` 是 per-session 单调递增序列号（AtomicU64）
- RedbStore `list_keys("events/{session_id}/")` 可高效前缀扫描

### 新增 REST API

```
GET /api/sessions/{id}/events?since={seq}&limit={n}
  → EventLog.query(session_id, since_seq, limit)
  → 返回 Vec<EventEntry>
```

---

## 五、详细实施计划

### Phase 0: 紧急修复双写竞态 (0.5天)

| 步骤 | 文件 | 修改内容 |
|------|------|---------|
| 0.1 | `routes.rs:2158-2183` | 删除 SSE stream 中对 `collector_for_stream` 的写入，仅保留 `convert_executor_event_to_sse` 转发 |
| 0.2 | `routes.rs:2109` | 删除 `collector_for_stream` 的创建（不再需要，`event_collector_handle` 是唯一写入者） |

### Phase 1: EventLog 核心实现 (1天)

| 步骤 | 文件 | 修改内容 |
|------|------|---------|
| 1.1 | `macaca-persist/src/event_log.rs` (新) | 实现 EventLog struct: append, query, latest_seq |
| 1.2 | `macaca-persist/src/lib.rs` | 新增 `pub mod event_log; pub use event_log::EventLog;` |
| 1.3 | `macaca-proto/src/types.rs` | 新增 `EventEntry` 结构体 |
| 1.4 | `macaca-persist/src/event_log.rs` | 单元测试：append, query, ordering, concurrent writes |

### Phase 2: 接入 EventLog 到所有事件产生点 (1天)

| 步骤 | 文件 | 修改内容 |
|------|------|---------|
| 2.1 | `macaca-web/src/state.rs` | AppState 新增 `event_log: Arc<EventLog>` |
| 2.2 | `macaca-web/src/lib.rs` | 初始化 EventLog |
| 2.3 | `routes.rs` run_agentic_stream | 每个 `tx.send(thinking/tool_call/...)` 之前先 `event_log.append()` |
| 2.4 | `routes.rs` SSE stream executor events | 每个 `convert_executor_event_to_sse` 之前先 `event_log.append()` |
| 2.5 | `routes.rs` plan_decision | `save_plan_decision` 改为 `event_log.append()` |
| 2.6 | `routes.rs` | 新增 `get_session_events` handler |
| 2.7 | `macaca-web/src/lib.rs` | 新增路由 `GET /api/sessions/:id/events` |

### Phase 3: 消除冗余持久化路径 (0.5天)

| 步骤 | 文件 | 修改内容 |
|------|------|---------|
| 3.1 | `routes.rs` | 删除 `AgentTraceCollector` 及其所有使用 |
| 3.2 | `routes.rs` | 删除 `periodic_saver` (1s interval task) |
| 3.3 | `routes.rs` | 删除 `persist_running_snapshot` |
| 3.4 | `routes.rs` | 删除 `update_session_realtime` |
| 3.5 | `routes.rs` | 删除 `save_agent_traces` / `load_agent_traces` |
| 3.6 | `routes.rs` `get_session_by_id` | 从 EventLog 重建 turns |

### Phase 4: SSE 通知化 + 前端改造 (0.5天)

| 步骤 | 文件 | 修改内容 |
|------|------|---------|
| 4.1 | `routes.rs` post_chat SSE stream | 简化为：EventLog 通知 → 转发 `{ event: "update", seq: N }` |
| 4.2 | `routes.rs` stream_session_events | 同上简化 |
| 4.3 | 前端 `api.ts` | 收到 "update" → fetch /api/sessions/{id}/events?since=last |
| 4.4 | 前端 `page.tsx` | 从 event list 重建 turns 用于渲染 |

---

## 六、迁移策略

采用 **双写渐进迁移**，每步可独立回滚：

**Step 1**: 新增 EventLog + 新 API，保持旧路径不变。每个事件同时写入 EventLog 和旧路径。验证数据一致性。

**Step 2**: 前端新增"从 EventLog 加载"能力（与旧 SSE 并存，feature flag 切换）。验证前端渲染正确。

**Step 3**: 关闭旧 SSE 数据通道，切换到 EventLog + 通知模式。保留 `get_session_by_id` 作为兼容层。

**Step 4**: 删除旧代码（AgentTraceCollector, periodic_saver, persist_running_snapshot 等）。

---

## 七、权衡

| 方案 | 优点 | 缺点 |
|------|------|------|
| **EventLog + SSE 通知化** (推荐) | 零数据丢失；刷新完美恢复；消除所有竞态；可审计可重放 | 前端需改造；增加 DB 写入；需清理策略 |
| **仅修复当前架构** | 改动最小；不需前端改造 | 不解决根本问题；进程崩溃仍丢数据 |
| **WAL + SSE 不变** | 保持前端兼容 | 复杂度高；WAL 和 SSE 一致性维护困难 |

---

## 八、关键源码引用

| 组件 | 位置 | 问题 |
|------|------|------|
| AgentTraceCollector 双写 | `routes.rs:1791` + `routes.rs:2158-2183` | 两个消费者写同一个 collector |
| persist_running_snapshot | `routes.rs:740-812` | read-modify-write 竞态 |
| save_agent_traces | `routes.rs:938-951` | 全量覆盖写 |
| plan_decision 聚合 | `routes.rs:982-996` | 按 app_id 而非 session 隔离 |
| SSE 不可重放 | `routes.rs:1483-1567` | 刷新后历史丢失 |
| periodic_saver | `routes.rs:1777-1786` | 依赖内存 collector |
| broadcast Lagged | `routes.rs:2217-2219` | 事件静默丢弃 |
| broadcast 容量 | `app_executor.rs:280` | 4096 容量限制 |

---

*设计完成：2026-03-30*
