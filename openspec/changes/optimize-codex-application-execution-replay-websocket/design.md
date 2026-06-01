## Context

Macaca's application execution platform already persists authoritative execution facts through `service.application_execution` and the shared EventLog. The remaining UI refinement is transport ordering and transport kind: a refresh should first ask the backend for durable replay/current-state, then subscribe for future rows. The WebSocket stream must be an Observer projection over durable EventLog notifications, not a new source of truth.

## Design Decisions

- Use the **Observer** pattern for realtime delivery: WebSocket subscribers observe EventLog append notifications and receive only rows that have already been persisted.
- Use the **Memento** pattern for refresh recovery: replay/current-state reconstruct the UI cache before any realtime subscription is opened.
- Use the **Adapter** pattern in `macaca-web`: the Web route adapts HTTP/WebSocket framing to EventLog queries and does not construct providers, append execution events, or interpret Codex task behavior.
- Keep Codex behavior in `apps/codex-wasm-workbench`: the UI submits a generic start command with app-owned task input and renders generic protocol events.

## Non-Goals

- No new application execution provider strategy.
- No Codex-specific branch in kernel, SDK, runtime-host generic services, or Web shell routes.
- No replacement of the existing replay/current-state/control API.
- No raw prompt/provider payload persistence; existing sanitized event rules still apply.

## Trace And Audit

Every WebSocket subscription requires an explicit `trace_id` and logs a bounded route-open event with application id, session id, cursor, and trace id. Delivered messages carry durable EventLog coordinates (`session_id`, `seq`) so replay and audit can verify the same facts after browser refresh or disconnect.
