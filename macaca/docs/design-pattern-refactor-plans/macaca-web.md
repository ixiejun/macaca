# macaca-web 设计模式渐进式重构计划

## 当前职责

`macaca-web` 是 Agent OS 的 HTTP/SSE/Web UI 后端，负责 server bootstrap、chat v2、session stream、event persistence、framework runner、framework toolkit、loop manager、routes 和状态管理。它目前承担了大量跨层协调逻辑，是最需要小步拆分的 crate。

重点对象：

- `start_server` / server bootstrap。
- `chat_orchestrator`。
- `framework_runner` / `framework_toolkit`。
- `loop_manager`。
- `event_persistence` / `sse` / `session`。
- routes。

## 适用模式

| 位置 | 现状风险 | 设计模式 | 渐进目标 |
| --- | --- | --- | --- |
| server bootstrap | `start_server` 负责配置、状态、路由、loops、skills/MCP，过大 | Builder + Facade | `WebServerBuilder` + `WebRuntimeFacade` |
| chat_v2 orchestration | coordinator、pause/resume、SSE、EventLog、task loops 混合 | Mediator | `ChatSessionMediator` |
| framework runner | agent 构建、toolkit、trace hook、workspace 混合 | Abstract Factory | `TracedAgentFactory` |
| event persistence | SSE payload、EventLog、trace steps 转换重复 | Observer + Visitor | `TraceEventSink` + event visitor |
| session recovery | refresh 后历史加载、增量推送需要一致 | Memento | `SessionReplayState` |
| routes | handler 直接操作复杂 state | Command + Facade | route command 调用 service facade |

## 小步重构计划

1. 第一切片：把 `start_server` 中纯装配步骤抽成私有 helper，不改变调用。
2. 第二切片：新增 `WebServerBuilder`，先只封装现有参数。
3. 第三切片：将 chat_v2 中 executor event forwarder 抽成 `TraceEventForwarder`，统一 SSE + EventLog。
4. 第四切片：将 `framework_runner` 中 traced agent 构建固化为唯一入口，删除无 trace 入口的调用机会。
5. 第五切片：`loop_manager` 中 worker/planner/coordinator 状态更新改走 framework-level status sink。
6. 第六切片：前端历史恢复和实时增量使用同一 event normalization 规则，后端提供稳定 event id/cursor。

## 示例代码片段

### WebServerBuilder

```rust
pub struct WebServerBuilder {
    config: WebConfig,
    app_state: Option<Arc<AppState>>,
    routes: Vec<RouteModule>,
}

impl WebServerBuilder {
    pub async fn build(self) -> Result<WebServer, WebError> {
        let state = self.app_state.unwrap_or_else(|| Arc::new(AppState::from_config(&self.config)));
        let router = self.build_router(Arc::clone(&state))?;
        Ok(WebServer { state, router })
    }
}
```

### TraceEventForwarder

```rust
pub struct TraceEventForwarder {
    event_log: Arc<EventLog>,
    sse: Arc<SessionSseHub>,
    normalizer: TraceEventNormalizer,
}

impl TraceEventForwarder {
    pub async fn forward(&self, session_id: &str, event: ExecutorEvent) {
        let normalized = self.normalizer.normalize(event);
        self.event_log.append_normalized(session_id, &normalized).await;
        self.sse.publish(session_id, normalized).await;
    }
}
```

### ChatSessionMediator

```rust
pub struct ChatSessionMediator {
    sessions: Arc<SessionRegistry>,
    task_space: Arc<TaskSpace>,
    agent_factory: Arc<dyn TracedAgentFactory>,
    trace_forwarder: Arc<TraceEventForwarder>,
}

impl ChatSessionMediator {
    pub async fn handle_user_message(&self, input: ChatInput) -> Result<ChatOutput, ChatError> {
        let session = self.sessions.open_or_resume(input.session_id).await?;
        let coordinator = self.agent_factory.coordinator(&session).await?;
        coordinator.reply(input.message).await
    }
}
```

## 验证策略

- 每次拆 `macaca-web` 只能移动一个 helper，并保留 route/API 行为 snapshot。
- 重点 regression：新建 session 不刷新也能实时看到 trace；刷新后历史 event 正确加载；增量 event 不重复。
- `FULLSTACK-AUTODEV` 和 `NEWSROOM-AUTOWRITER` 都要作为 smoke test，防止只对开发类应用有效。
- 任何涉及 `loop_manager` 和 `chat_orchestrator` 的改动必须先跑 GitNexus impact，因为它们处于核心用户链路。

