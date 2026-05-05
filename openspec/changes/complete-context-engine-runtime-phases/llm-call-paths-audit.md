# LLM call paths — context report coverage (audit)

This note satisfies **complete-context-engine-runtime-phases** task **3.3** (audit or document non-applicable paths).

## Supported: durable `context_report` EventLog events

| Path | Mechanism | Persistence |
|------|-----------|---------------|
| Framework `ReActAgent` / `ContextReportingChatModel` | Wraps `ChatModel::chat`; runs `ContextRuntimeFacade::builtins` when `session_id` is present | Append-only `event_log` row with `event_type = "context_report"` on the session |

## Supported: runtime stream / trace (not EventLog `context_report`)

| Path | Mechanism | Persistence |
|------|-----------|---------------|
| `macaca-runtime` `AgenticLoop::run_iteration` | `ContextRuntimeFacade` + `DriverTrace` envelope with `trace.event_type = "context_report"` | Propagates via `AgentExecutionEvent` channel to consumers (e.g. SSE / executor). Debug log `runtime_context_report` is ancillary only. |

These paths are **runtime-observable** but **not** the same row shape as the web EventLog `context_report` API. Callers that require a single persistence format should prefer the framework web path or add an explicit EventLog bridge when a session id is available.

## Not applicable (by design): library / SDK style one-off calls

These entry points build **library-style** LLM calls without a Macaca chat session, persisted EventLog, or shared context facade. They are intentionally out of scope for automatic `context_report` persistence.

| Crate / module | Reason |
|----------------|--------|
| `macaca-sdk` (`builder.rs`) | Minimal embedded API; no session or event log |
| `macaca-agent` (`basic.rs`, `agent.rs`) | Thin agent helpers; no orchestrated Macaca session |
| `macaca-task` (`plan_loop.rs`, `decompose.rs`) | Internal planning/decomposition; optional future bridge only |
| `macaca-cli` (`commands.rs`) stub provider | Diagnostics / smoke; not a hosted session |
| `Kernel` test harness direct `llm.chat` | Unit / integration harness only |

**Migration / extension:** If a future feature needs reports here, inject `ContextRuntimeFacade` (or a decorator `LlmProvider`) and an `EventLog` / trace sink keyed by an explicit `session_id` or correlation id.

## Phase 5 notes

- Engine selection is **configuration-driven** (`ContextConfig`, app manifest `context`, inline agent `context_engine`); no branching on app or workflow names inside the engines.
- `context_engine_fallback` events are emitted when the primary engine fails assembly and the fallback engine is used.
