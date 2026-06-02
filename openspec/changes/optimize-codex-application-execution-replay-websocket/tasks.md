## 1. Specification And Boundary Alignment

- [x] 1.1 Add delta spec for replay-first WebSocket consumption over durable EventLog.
- [x] 1.2 Validate the OpenSpec change strictly before implementation completion.

## 2. Backend Thin Adapter

- [x] 2.1 Add a WebSocket route in `macaca-web` for application execution event increments.
- [x] 2.2 Reuse the existing service-owned EventLog query filters and durable stream payload.
- [x] 2.3 Add route tests proving the WebSocket adapter remains side-effect free and query-bounded.

## 3. Codex Workbench App UI

- [x] 3.1 Change production UI refresh/start flow to replay persisted events before opening realtime transport.
- [x] 3.2 Replace production `EventSource` consumption with WebSocket incremental updates.
- [x] 3.3 Keep browser LLM/tool execution debug-only and outside production execution.

## 4. Verification

- [x] 4.1 Run focused UI tests.
- [x] 4.2 Run focused Web shell tests.
- [x] 4.3 Run OpenSpec strict validation and repository diff checks.
- [x] 4.4 Run GitNexus change detection before commit.
- [x] 4.5 Send a real backend task to `codex-wasm-workbench` and verify EventLog replay/current-state plus WebSocket delivery after subscriber disconnect.
