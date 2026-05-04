## Context

`macaca-web` currently contains several files above the project target of 500 lines:

- `loop_manager.rs`
- `session.rs`
- `framework_runner.rs`
- `routes.rs`
- `framework_toolkit.rs`
- `lib.rs`
- `chat_orchestrator.rs`

Existing changes have already split the original route surface, grouped `AppState`, migrated goal execution toward framework primitives, and moved many lower-layer consumer paths to facades. This proposal should not repeat those changes. It should define web-side primitives that allow future splits to be reviewable and behavior-preserving.

## Goals

- Make web server startup explicit through Builder + Facade primitives.
- Keep `start_server(port)` available but compatibility-only.
- Add additive primitives for event forwarding, session replay, chat mediation, traced agent construction, route commands, and status sinks.
- Preserve all HTTP routes, SSE payload shapes, EventLog writes, app discovery behavior, and task loop startup behavior in the first implementation.
- Keep every implementation slice small enough to review and revert.

## Non-Goals

- Do not remove `start_server`.
- Do not change frontend API routes or response schemas.
- Do not rewrite `post_chat_v2` or `ensure_plan_and_worker_loops` in the same first implementation slice.
- Do not introduce application-specific workflow, app, driver, or agent names.
- Do not move lower-layer semantics back into web.

## Design Decisions

### Builder + Facade

Add `WebServerBuilder` and `WebRuntimeFacade` as the canonical server startup path.

`start_server(port)` remains present for CLI/backward compatibility, but it becomes:

```rust
#[deprecated(note = "Use WebServerBuilder::new().port(port).serve() instead")]
pub async fn start_server(port: u16) -> MacacaResult<()> {
    WebServerBuilder::new().port(port).serve().await
}
```

This marks the old interface for migration discovery while preserving behavior.

### Bootstrap Helper Extraction

The builder should initially reuse private helper functions that preserve the current startup order:

1. load config
2. apply MCP process environment
3. build LLM router and kernel
4. discover and start apps
5. load skills and executable tools
6. load driver runtime
7. assemble orchestration tools
8. initialize persistence
9. build `AppState`
10. register executors and start hook consumer
11. build router
12. bind and serve

The first implementation may keep most helper bodies in `lib.rs` to reduce file churn. Later changes can split them into dedicated modules.

### Event Forwarding Primitives

Add `TraceEventForwarder` and a minimal `TraceEventNormalizer` as additive primitives. They should initially delegate to existing EventLog/SSE payload behavior or remain unused until a later migration. The goal is to establish the pattern boundary without changing live event behavior.

### Session Replay Primitive

Add a `SessionReplayState` struct that represents replay cursor/state. The first implementation should not change how `session.rs` reconstructs traces; it only introduces the type and tests for basic cursor semantics.

### Chat Session Mediator

Add a `ChatSessionMediator` shell that owns an `Arc<AppState>` and can later absorb session open/resume, coordinator construction, and trace forwarding. The first implementation should not replace `post_chat_v2`.

### Route Command Primitive

Add a small `RouteCommand` trait for future handler-to-service migration. Do not migrate existing routes in this change unless a pure adapter can be added without behavior change.

### Deprecation Policy

Deprecated Rust interfaces must remain available. New callers should use the pattern primitive. For this change:

- `start_server` is deprecated and delegates to `WebServerBuilder`.
- Any old helper replaced by a new primitive should be deprecated only if it is public or crate-visible and still needed for compatibility.
- Do not delete old handlers.

## Risks

- `start_server` has broad blast radius because it constructs all runtime state.
- `post_chat_v2` and `ensure_plan_and_worker_loops` are core user paths; this change should only add shells for those future migrations.
- Builder extraction can accidentally change startup order; keep helper calls ordered and validate app discovery.
- Deprecated `start_server` can warn inside callers. Allow deprecated usage only at compatibility call sites if needed.

## Validation

- `openspec validate refactor-macaca-web-patterns --strict`
- `cargo fmt --all`
- `cargo check -p macaca-web`
- Smoke start backend from `macaca/` and verify `/api/status` and `/api/apps` if feasible.
