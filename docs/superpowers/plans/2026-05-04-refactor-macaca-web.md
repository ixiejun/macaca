# macaca-web Design Pattern Refactor Brainstorm and Plan

Date: 2026-05-04

## 1. Current Code Facts

This plan follows:

- `macaca/docs/design-pattern-refactor-plans/README.md`
- `macaca/docs/design-pattern-refactor-plans/refactor-order.md`
- `macaca/docs/design-pattern-refactor-plans/macaca-web.md`
- `macaca/docs/design_patterns.md`
- `openspec/AGENTS.md`

`macaca-web` is phase 5 in the global refactor order: the final delivery entry layer. It is intentionally last because it depends on almost every backend crate and currently absorbs unstable lower-layer glue.

Current module size snapshot:

- `loop_manager.rs`: 2730 lines
- `session.rs`: 1677 lines
- `framework_runner.rs`: 1527 lines
- `routes.rs`: 1185 lines
- `framework_toolkit.rs`: 1125 lines
- `lib.rs`: 758 lines
- `chat_orchestrator.rs`: 679 lines

This still violates the project preference that files stay under 500 lines, but the correct response is incremental extraction with behavior parity, not a large split.

Existing web-related refactor history:

- `refactor-core-architecture` split the original giant routes surface into modules and grouped `AppState`.
- `migrate-goal-pipeline-to-framework`, `migrate-hardcoded-orchestration-to-framework`, `refactor-trace-middleware-helpers`, `refactor-planner-framework-call-helper`, and `refactor-worker-execution-result-template` already cover significant `chat_orchestrator`, `framework_runner`, and `loop_manager` migration work.
- `migrate-runtime-host-consumers-to-facade-boundary`, `migrate-driver-consumers-to-runtime-primitives`, `migrate-skill-consumers-to-pattern-primitives`, `migrate-persist-consumers-to-pattern-primitives`, and `migrate-sdk-consumers-to-facade-spec` already migrated several web consumers to lower-layer facades.

Current high-risk facts:

- `start_server` still performs configuration loading, environment application, LLM/kernel creation, app discovery/startup, skill/executable skill loading, driver runtime loading, orchestration tool callback assembly, persistence setup, AppState construction, executor registration, hook consumer startup, router construction, and axum serving in one function.
- `start_server` is the safest next first slice because pure assembly helpers can be extracted without changing HTTP API, session, SSE, task, or framework execution behavior.
- `chat_orchestrator::post_chat_v2` and `loop_manager::ensure_plan_and_worker_loops` are core user paths and already covered by ongoing/past changes; touching them should be separate, after a narrower bootstrap refactor.

## 2. Superpowers Brainstorm

### Option A: Extract private server bootstrap helpers first

Move pure assembly blocks inside `start_server` into private helpers in `lib.rs` or a small new bootstrap module:

- load config and apply MCP environment
- build LLM router and kernel
- discover/start apps and collect app dirs/skill dirs/started app metadata
- load skill catalog and executable skill tools
- build orchestration tools
- build persistence state
- build router

Benefits:

- Matches the first slice in `macaca-web.md`.
- Behavior can remain 1:1 because helpers only move existing code.
- Reduces `start_server` complexity before introducing public builder types.
- Easier to review and revert than touching chat/session/loop behavior.

Risks:

- Helpers can become a new dump of parameters if grouped poorly.
- App startup is still high impact because it feeds the web UI and executor registry.
- Moving closures for delegate/get-result tools can break captured Arc lifetimes if not kept mechanical.

### Option B: Introduce `WebServerBuilder` immediately

Add a `WebServerBuilder` that owns config, port, state construction, router construction, and serve startup.

Benefits:

- Directly matches the target Builder pattern.
- Creates a clear future seam for tests and alternate web entrypoints.
- Can eventually make `start_server(port)` a thin facade.

Risks:

- Too much for a first slice because builder design forces naming and ownership choices before helper boundaries are clear.
- High chance of long parameter shuffling across `AppState`, router, app startup, and tool construction.
- If done before helper extraction, the builder may simply wrap the current giant function.

### Option C: Start with `TraceEventForwarder`

Extract a unified SSE/EventLog forwarder from chat/session/event persistence paths.

Benefits:

- Addresses real user-facing refresh/live-event consistency risk.
- Matches Observer + Visitor goals.
- Builds toward stable event normalization and cursor semantics.

Risks:

- Touches `post_chat_v2`, `session` stream/replay, `event_persistence`, and SSE semantics.
- Duplicate or missing events are easy to introduce and hard to spot without browser regression tests.
- Existing persist consumer migration already touched event replay; this should not be mixed with bootstrap refactor.

### Option D: Start with `ChatSessionMediator`

Introduce a mediator for chat v2 session open/resume, coordinator build, pause/resume, SSE, and EventLog.

Benefits:

- Targets the highest-complexity business path.
- Could eventually make HTTP handlers thin command adapters.

Risks:

- Very high blast radius across session lifecycle, coordinator trace, active sessions, cancellation, and task loops.
- Requires strong behavior snapshots for refresh recovery and live trace streams.
- Too risky as the first producer refactor slice for `macaca-web`.

### Option E: Continue splitting giant `loop_manager.rs`

Extract planner/worker/status sink modules or facade layers from `loop_manager.rs`.

Benefits:

- Directly attacks the largest file.
- Aligns with the project 500-line file rule.

Risks:

- `ensure_plan_and_worker_loops` is a critical user path.
- Several recent changes already targeted planner and worker helper extraction; another overlapping slice risks merge conflicts and behavior drift.
- File-size reduction alone is not enough if the extracted boundaries are not stable.

## 3. Recommendation

Choose Option A as the first macaca-web refactor slice, then Option B.

Rationale:

- It is the smallest behavior-preserving step toward the documented `WebServerBuilder + WebRuntimeFacade` target.
- It avoids touching the highest-risk chat/session/loop paths while current lower-layer migrations settle.
- It gives the future `WebServerBuilder` real internal components to compose instead of wrapping a monolithic `start_server`.
- It keeps the change generic and application-agnostic: no workflow names, app names, driver names, or business-specific logic should be introduced.

Recommended first change ID:

- `refactor-web-server-bootstrap-helpers`

Recommended second change ID:

- `refactor-web-server-builder-facade`

## 4. Risk Register

- Risk: `start_server` feeds app discovery, app runtime, executor registry, MCP, drivers, tools, persistence, SSE, and router setup.
  Control: First slice should only move code into private helpers and keep execution order identical.

- Risk: closures for `DelegateTaskTool` and `GetTaskResultTool` capture shared registry state.
  Control: Keep callback construction in one helper with the same captured `Arc<RwLock<Option<...>>>` and `delegate_session_id`.

- Risk: app discovery depends on current working directory.
  Control: Do not change app registry defaults or startup cwd assumptions in this refactor.

- Risk: current repository may have unrelated dirty web/app changes.
  Control: Before implementation, inspect `git status`; do not revert or mix unrelated edits.

- Risk: GitNexus impact will likely be HIGH/CRITICAL for `start_server`.
  Control: Run impact analysis before editing, report blast radius, and keep the first patch limited to helper extraction.

## 5. Write Plan

### Task 1: OpenSpec Proposal

Create `openspec/changes/refactor-web-server-bootstrap-helpers/`:

- `proposal.md`: explain why `start_server` needs private helper extraction before `WebServerBuilder`.
- `design.md`: document exact helper boundaries, execution-order preservation, and non-goals.
- `tasks.md`: track context checks, impact analysis, helper extraction, validation, and deprecated/hardcode scans.
- `specs/web-server-bootstrap/spec.md`: add requirements for behavior-preserving bootstrap helper extraction.

Validation:

```bash
openspec validate refactor-web-server-bootstrap-helpers --strict
```

### Task 2: Pre-Edit Impact Analysis

Run GitNexus checks:

```bash
npx gitnexus status
npx gitnexus impact --repo agent start_server
```

If stale:

```bash
npx gitnexus analyze
```

Report direct callers, affected processes, and risk before editing.

### Task 3: Extract Private Bootstrap Helpers

Modify `macaca/crates/macaca-web/src/lib.rs` only in the first implementation slice.

Suggested helper boundaries:

- `load_web_config() -> MacacaConfig`
- `apply_mcp_process_env(config: &MacacaConfig)`
- `build_llm_and_kernel(config: &MacacaConfig) -> MacacaResult<(Arc<LlmRouter>, Arc<dyn LlmProvider>, Arc<Kernel>)>`
- `discover_and_start_apps(...) -> MacacaResult<StartedApps>`
- `load_skill_catalog(...) -> SkillCatalog`
- `build_startup_tools(...) -> MacacaResult<Arc<dyn ToolCatalog>>`
- `build_persistence_state(...) -> MacacaResult<...>`
- `build_app_state(...) -> Arc<AppState>`
- `build_router(state: Arc<AppState>) -> Router`

Keep helpers private unless a later OpenSpec change needs a public builder.

### Task 4: Keep `start_server` as the Facade

After helper extraction, `start_server(port)` should still:

- load config
- construct state
- build router
- bind and serve

It should not change:

- route paths
- CORS policy
- app discovery directories
- auto-start behavior
- driver/skill/MCP loading behavior
- EventLog/session/audit/run_trace setup
- PlanLoop/WorkerLoop auto-start behavior

### Task 5: Validation

Run focused checks:

```bash
cargo fmt --all
cargo check -p macaca-web
openspec validate refactor-web-server-bootstrap-helpers --strict
```

If feasible, run a smoke startup:

```bash
cargo run --release --bin macaca -- web --port 3001
curl -fsS http://localhost:3001/api/status
curl -fsS http://localhost:3001/api/apps
```

Expected status:

- server starts
- apps are discovered when launched from `macaca/`
- existing routes remain available

### Task 6: Follow-Up WebServerBuilder Change

Only after Task 5 passes, open a separate OpenSpec change for `WebServerBuilder`.

The builder should compose the private helpers introduced in the first slice and make `start_server(port)` delegate to it. Do not introduce `ChatSessionMediator`, `TraceEventForwarder`, or route command objects in the same change.

## 6. Definition of Done

- Superpowers plan is recorded before OpenSpec implementation.
- OpenSpec proposal/design/tasks/spec are valid.
- `start_server` remains the only public web server entrypoint in the first slice.
- First implementation slice changes only bootstrap assembly, not chat/session/loop behavior.
- `cargo check -p macaca-web` passes.
- Manual `/api/status` and `/api/apps` smoke checks pass if the server is started.
