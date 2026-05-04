# Migrate macaca-web Consumers to Pattern Primitives

Date: 2026-05-04

## 1. Current Code Facts

This plan follows:

- `AGENTS.md`
- `openspec/AGENTS.md`
- `macaca/docs/design_patterns.md`
- `macaca/docs/design-pattern-refactor-plans/README.md`
- `macaca/docs/design-pattern-refactor-plans/refactor-order.md`
- `macaca/docs/design-pattern-refactor-plans/macaca-web.md`
- `openspec/changes/refactor-macaca-web-patterns/`

The current `macaca-web` refactor introduced:

- `WebServerBuilder` as the canonical startup path.
- `WebRuntimeFacade` as the router/state binding facade.
- Deprecated compatibility facade `start_server(port)`.
- Additive web primitives: `TraceEventForwarder`, `TraceEventNormalizer`, `SessionReplayState`, `ChatSessionMediator`, and `RouteCommand`.
- `orchestration_tools.rs` as a split-out web orchestration tool assembly module.

Current direct Rust consumers:

- `macaca-cli/src/main.rs` already uses `macaca_web::WebServerBuilder::new().port(port).serve().await`.
- `rg "start_server\(" macaca/crates` shows only the deprecated definition in `macaca-web/src/lib.rs`.

Current HTTP/UI consumers:

- `frontend/lib/api.ts` is the main frontend API client and calls `/api/status`, `/api/apps`, `/api/apps/{id}/agents`, `/api/apps/{id}/agents/stream`, `/api/apps/{id}/sessions`, `/api/sessions/detail/{session_id}`, `/api/chat/v2`, `/api/sessions/stream/{session_id}`, `/api/chat/stop`, `/api/apps/{id}/todos`, and `/api/sessions/{id}/events`.
- `frontend/app/chat/[appId]/page.tsx` owns a large part of session replay, EventLog incremental loading, SSE event mapping, and coordinator/delegated trace display.
- `frontend/next.config.ts` rewrites `/api/:path*` to `http://localhost:3001/api/:path*`.
- `frontend/lib/api.ts` independently computes `API_BASE` and also defaults to port `3001`.
- `frontend/app/page.tsx` hardcodes the user-facing error text "backend is running on port 3001".
- `macaca/scripts/trace_watch.py` consumes run-trace and EventLog endpoints and defaults to `MACACA_API` or `http://127.0.0.1:3001`.
- `macaca/tests/e2e_project_task.sh` hardcodes `BASE`, `APP_ID`, and concrete agent names `backend`, `frontend`, `architect`, plus a `coordinator` existence check.
- `macaca/README_PART4.md` still documents legacy `POST /api/chat`, while backend routing only keeps `/api/chat/v2` and `chat_orchestrator::post_chat` is deprecated.
- `frontend/docs/legacy-ui-feature-alignment.md` still discusses legacy `/api/chat`.

Design-pattern fit before planning:

- Facade: frontend should use a single API boundary instead of duplicating base URL and endpoint construction logic.
- Adapter: scripts and tests should adapt to the current web contract without embedding app-specific IDs or role names.
- Memento: frontend session replay should align with `SessionReplayState` semantics, using cursor/latest sequence consistently.
- Mediator: chat workspace state should move toward a frontend-side session mediator before backend `ChatSessionMediator` is used by live handlers.
- Command: route/API actions can be represented as small commands in tests or frontend clients, but this should stay lightweight and not over-abstract.

## 2. Superpowers Brainstorm

### Option A: Rust-only consumer migration guard

Scope:

- Verify and lock the `macaca-cli` migration to `WebServerBuilder`.
- Add a small test or static scan that fails if `macaca_web::start_server` is called outside the compatibility definition.
- Update docs that mention `start_server()` as the startup surface.

Benefits:

- Lowest risk because the Rust direct consumer is already migrated.
- Creates a durable guard so future code cannot reintroduce deprecated startup calls.
- Small, reviewable, and aligned with the deprecation policy.

Risks:

- Does not address frontend/session/event consumer complexity.
- Gives a false sense of completion while HTTP clients still duplicate backend base logic and event replay code.
- Static scans can be brittle unless scoped narrowly.

### Option B: Frontend API facade migration

Scope:

- Move `frontend/lib/api.ts` from ad-hoc URL construction toward a small `MacacaApiClient` facade.
- Centralize API base resolution, EventSource URL construction, fetch error handling, and EventLog replay calls.
- Keep all endpoints and payloads unchanged.
- Update `next.config.ts` and error text to use one documented base URL source.

Benefits:

- Aligns with the backend `WebRuntimeFacade` and `SessionReplayState` direction without changing server behavior.
- Reduces duplicated `API_BASE + endpoint` string assembly.
- Makes future `TraceEventForwarder`/session replay migration easier because the frontend has one consumption boundary.

Risks:

- `frontend/lib/api.ts` is used by both launcher and chat workspace; mistakes can break the UI even if backend is unchanged.
- EventSource has different URL constraints from fetch; the facade must not hide those differences incorrectly.
- If done too broadly, the change can become a frontend rewrite instead of a consumer migration.

### Option C: Session replay consumer migration

Scope:

- Introduce a frontend replay cursor model that mirrors backend `SessionReplayState`.
- Migrate `frontend/app/chat/[appId]/page.tsx` incremental EventLog fetches to use the model.
- Keep current SSE event handling and backend endpoints unchanged.

Benefits:

- Directly targets the most important browser refresh/live trace consistency path.
- Provides a clearer boundary for future backend `TraceEventForwarder` migration.
- Can reduce duplicate latest-seq handling and driver trace injection code.

Risks:

- High UI regression risk: duplicate trace steps, missing driver traces, broken refresh recovery, or stale `lastProcessedSeq`.
- `ChatPage` is large and stateful; changing replay logic before an API facade exists increases coupling.
- Requires stronger browser or integration smoke checks.

### Option D: E2E and script consumer migration

Scope:

- Make `macaca/tests/e2e_project_task.sh` discover app IDs and agent names from `/api/apps` and `/api/apps/{id}/agents`.
- Remove hardcoded `APP_ID`, `backend`, `frontend`, `architect`, and `coordinator` assumptions where possible.
- Keep `trace_watch.py` default behavior but document `MACACA_API`; optionally add a shared API base convention.

Benefits:

- Directly enforces the Macaca OS rule that infra code must not hardcode app/agent/business names.
- Reduces false negatives when apps differ from the sample fullstack app.
- Does not require backend behavior changes.

Risks:

- Existing E2E may intentionally target a specific example app; making it generic could reduce coverage of that scenario unless a fixture mode remains.
- Dynamic agent selection needs careful criteria, such as capabilities or simply "first non-entry agent", to avoid business-specific logic.
- Shell tests can become hard to read if generic discovery is overdone.

### Option E: Documentation/API contract migration

Scope:

- Update stale docs that still mention legacy `/api/chat`.
- Update docs to describe `WebServerBuilder` as the Rust startup API and `/api/chat/v2` as the HTTP chat endpoint.
- Mark `/api/chat` as removed/deprecated in user-facing docs without deleting backend compatibility code.

Benefits:

- Low code risk and important for migration discoverability.
- Prevents new consumers from copying deprecated examples.
- Complements static deprecated-call scans.

Risks:

- Docs-only migration does not improve runtime safety.
- There are many historical audit docs; editing all of them can create noisy churn.
- Some docs intentionally describe legacy behavior; those should be annotated rather than rewritten as current truth.

## 3. Recommendation

Use a combined but staged plan:

1. Start with Option A and Option E for Rust startup/deprecated API guardrails.
2. Then implement Option B as the primary upper-layer migration: a frontend API facade that preserves existing HTTP/SSE contracts.
3. Then implement Option D for test/script genericity, especially removing hardcoded app IDs and fixed agent role names from active E2E code.
4. Defer Option C until the API facade is in place, unless a narrow replay-cursor adapter can be added without changing `ChatPage` behavior.

Rationale:

- Direct Rust migration is already done; the next value is preventing deprecated reintroduction and removing stale examples.
- The frontend is the most important upper-layer consumer, but it should first get a stable API facade before touching replay internals.
- E2E hardcoded app/agent names violate the project rule and should be migrated before relying on these tests as generic OS validation.
- Chat/session replay is high value but high risk; it should be a later slice with explicit browser smoke validation.

## 4. Risk Register

- Risk: frontend API facade changes can break both fetch and EventSource flows.
  Control: keep endpoint paths identical, add no new dependency, and validate with `npm run lint` plus manual smoke against `/api/status`, `/api/apps`, `/api/chat/v2`, and session stream when backend is available.

- Risk: E2E generic discovery may hide app-specific regressions.
  Control: support environment overrides such as `MACACA_API`, `APP_ID`, and optional agent names, but default to dynamic discovery.

- Risk: stale docs may describe legacy architecture intentionally.
  Control: only update active README/how-to/API docs as current truth; annotate historical audit docs instead of rewriting them.

- Risk: `ChatPage` contains hardcoded coordinator UI semantics.
  Control: treat coordinator label migration as a separate proposal unless the backend exposes an entry-agent display contract.

- Risk: GitNexus impact for frontend TypeScript may be weak or unavailable.
  Control: still run GitNexus for Rust symbols before Rust edits, and use targeted `rg` plus TypeScript validation for frontend-only changes.

## 5. Write Plan

### Task 1: Prepare OpenSpec consumer migration proposal

Create a new change, recommended ID:

- `migrate-macaca-web-consumers-to-pattern-primitives`

OpenSpec artifacts:

- `proposal.md`: explain why upper-layer consumers must move to the new web primitives and stop copying deprecated/legacy contracts.
- `design.md`: document API facade, deprecation guard, E2E generic discovery, and non-goals.
- `tasks.md`: track implementation slices and validation.
- `specs/macaca-web-consumer-migration/spec.md`: add requirements for builder startup usage, frontend API facade, no deprecated chat endpoint usage, and generic E2E consumption.

Validation:

```bash
openspec validate migrate-macaca-web-consumers-to-pattern-primitives --strict
```

### Task 2: Consumer inventory and impact checks

Read/confirm:

- `macaca/crates/macaca-cli/src/main.rs`
- `frontend/lib/api.ts`
- `frontend/app/page.tsx`
- `frontend/app/chat/[appId]/page.tsx`
- `frontend/next.config.ts`
- `macaca/scripts/trace_watch.py`
- `macaca/tests/e2e_project_task.sh`
- active README/API docs that mention `/api/chat` or `start_server`

Run scans:

```bash
rg -n "start_server\(|macaca_web::start_server|/api/chat\b|localhost:3001|APP_ID=|backend|frontend|architect|coordinator" \
  macaca frontend --glob '!**/node_modules/**' --glob '!target/**'
```

For Rust edits, run GitNexus impact before changing affected symbols:

```bash
npx gitnexus impact --repo agent 'Function:macaca/crates/macaca-cli/src/main.rs:main' --direction upstream
```

### Task 3: Rust startup guard slice

Goal:

- Ensure `WebServerBuilder` is the only non-compat Rust startup path.

Implementation candidates:

- Keep `macaca-cli` using `WebServerBuilder`.
- Add a lightweight repository scan test or scripted validation in the OpenSpec task list rather than adding a new dependency.
- Keep `start_server` deprecated and present.

Validation:

```bash
rg -n "start_server\(" macaca/crates
cargo check -p macaca-cli
```

Expected result:

- Only `macaca-web/src/lib.rs` defines `start_server`.

### Task 4: Frontend API facade slice

Goal:

- Centralize frontend web API consumption behind one facade while preserving every current endpoint and payload shape.

Implementation candidates:

- Introduce a small `MacacaApiClient` in `frontend/lib/api.ts` or a new `frontend/lib/macaca-api-client.ts`.
- Keep existing exported functions as compatibility delegates to avoid broad UI churn.
- Centralize:
  - `apiBase()`
  - `apiUrl(path)`
  - `eventSourceUrl(path)`
  - `jsonFetch<T>(path, init?)`
  - chat streaming fetch
  - session event replay fetch
- Keep `NEXT_PUBLIC_API_BASE` as the override.
- Do not change endpoint paths.

Validation:

```bash
cd frontend
npm run lint
```

If backend is available:

```bash
curl -fsS http://localhost:3001/api/status
curl -fsS http://localhost:3001/api/apps
```

### Task 5: Active docs/API contract slice

Goal:

- Stop active docs from recommending removed/deprecated interfaces.

Implementation candidates:

- Update `macaca/README_PART4.md` to remove or mark `POST /api/chat` as legacy removed and point to `/api/chat/v2`.
- Update current startup docs to prefer `macaca web --port` / `WebServerBuilder` where Rust API is mentioned.
- Leave historical audit docs intact unless they are framed as current instructions.

Validation:

```bash
rg -n "/api/chat\b|start_server\(\)" macaca/README.md macaca/README_PART*.md frontend/docs
```

### Task 6: E2E/script generic consumer slice

Goal:

- Make active tests/scripts compatible with arbitrary Macaca applications.

Implementation candidates:

- In `e2e_project_task.sh`, let `BASE` default from `MACACA_API`.
- Let `APP_ID` be optional; when absent, fetch `/api/apps` and choose the first started/available app.
- Replace fixed agent board checks for `backend/frontend/architect` with dynamic checks over agents from `/api/apps/{id}/agents`.
- Replace required `coordinator` existence check with a generic "at least one agent is registered" or entry-agent contract if the backend exposes one.
- Keep explicit env overrides for app-specific fixture tests.

Validation:

```bash
bash -n macaca/tests/e2e_project_task.sh
python3 -m py_compile macaca/scripts/trace_watch.py
```

If backend is available:

```bash
MACACA_API=http://localhost:3001 bash macaca/tests/e2e_project_task.sh
```

### Task 7: Deferred session replay migration

Only after Tasks 4-6 are stable:

- Introduce a frontend replay cursor adapter aligned with backend `SessionReplayState`.
- Migrate only the `lastProcessedSeqRef` and EventLog fetch boundary first.
- Do not change event rendering or trace injection in the same slice.

Validation:

- Browser refresh during active chat does not duplicate or lose driver/delegated trace steps.
- `npm run lint`.
- Backend smoke for `/api/sessions/{id}/events?since=&limit=`.

## 6. Non-Goals

- Do not remove deprecated backend Rust functions.
- Do not reintroduce `/api/chat` routing.
- Do not change backend HTTP route paths or response payloads.
- Do not hardcode workflow, app, driver, or application-specific agent names.
- Do not rewrite `ChatPage` wholesale in the first consumer migration.
- Do not archive OpenSpec changes until implementation and validation are aligned.
