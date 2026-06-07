## 1. Context and Impact

- [x] 1.1 Read web refactor plan, current web code, and existing web-related OpenSpec changes.
- [x] 1.2 Run GitNexus impact for `start_server`, `post_chat_v2`, and `ensure_plan_and_worker_loops`.
- [x] 1.3 Report HIGH/CRITICAL blast radius before edits.

## 2. OpenSpec

- [x] 2.1 Create proposal, design, tasks, and delta spec.
- [x] 2.2 Validate `refactor-macaca-web-patterns` with `--strict`.

## 3. Bootstrap Builder and Facade

- [x] 3.1 Add `WebServerBuilder` as canonical startup builder.
- [x] 3.2 Add `WebRuntimeFacade` for state/router binding.
- [x] 3.3 Make `start_server(port)` deprecated and delegate to `WebServerBuilder`.
- [x] 3.4 Preserve existing route paths, CORS, app discovery, app startup, tool loading, driver loading, persistence, executor registration, and hook consumer startup.

## 4. Additive Pattern Primitives

- [x] 4.1 Add `TraceEventForwarder` and `TraceEventNormalizer` shells without replacing live event paths.
- [x] 4.2 Add `SessionReplayState` cursor/replay shell without replacing session reconstruction.
- [x] 4.3 Add `ChatSessionMediator` shell without replacing `post_chat_v2`.
- [x] 4.4 Add `RouteCommand` shell without replacing routes.

## 5. Deprecation and Migration Guards

- [x] 5.1 Ensure deprecated web Rust entrypoints remain present.
- [x] 5.2 Ensure no old interface is deleted.
- [x] 5.3 Avoid adding `#[allow(deprecated)]` because no migrated caller uses deprecated web APIs.

## 6. Verification

- [x] 6.1 Run `cargo fmt --all`.
- [x] 6.2 Run `cargo check -p macaca-web`.
- [x] 6.3 Run `openspec validate refactor-macaca-web-patterns --strict`.
- [ ] 6.4 Run smoke `/api/status` and `/api/apps` if backend startup is feasible.
- [x] 6.5 Run GitNexus detect-changes and review affected scope.
