## Context

The previous `refactor-macaca-web-patterns` change introduced `WebServerBuilder`, `WebRuntimeFacade`, and additive web-side primitives such as `TraceEventForwarder`, `SessionReplayState`, `ChatSessionMediator`, and `RouteCommand`.

Direct Rust startup consumption is already migrated in `macaca-cli`, but active upper layers still need cleanup:

- `frontend/lib/api.ts` builds API URLs directly in every exported function.
- `frontend/next.config.ts`, `frontend/lib/api.ts`, and homepage error text all encode backend location assumptions separately.
- `macaca/tests/e2e_project_task.sh` hardcodes `APP_ID` and concrete agent names.
- Active docs still mention legacy `/api/chat` in at least one current README part.

## Goals

- Ensure new Rust callers use `WebServerBuilder`, not deprecated `start_server`.
- Introduce a small frontend facade for HTTP and SSE consumption without changing endpoints.
- Preserve existing frontend exported functions so UI components do not need a broad rewrite.
- Keep E2E tests generic by default while preserving environment overrides for fixture runs.
- Update active docs so current instructions point to `/api/chat/v2`.

## Non-Goals

- Do not delete `start_server`.
- Do not reintroduce `/api/chat` backend routing.
- Do not change backend response schemas or SSE event names.
- Do not rewrite `frontend/app/chat/[appId]/page.tsx` in this change.
- Do not remove historical audit notes that intentionally describe legacy behavior.

## Design Decisions

### Frontend API Facade

Use a lightweight Facade in `frontend/lib/api.ts` rather than a new dependency or generated client.

The facade should centralize:

- API base resolution from `NEXT_PUBLIC_API_BASE`, then browser host fallback, then localhost fallback.
- `apiUrl(path)` for fetch consumers.
- `eventSourceUrl(path)` for SSE consumers.
- `jsonFetch<T>(path, init?)` for JSON calls.

Existing exported functions such as `fetchStatus`, `fetchApps`, `sendChat`, and `subscribeSessionStream` remain as compatibility functions that delegate to the facade.

### E2E Adapter

Treat `macaca/tests/e2e_project_task.sh` as an adapter over the web API:

- `BASE` defaults from `MACACA_API`, with `http://localhost:3001` as fallback.
- `APP_ID` remains an override. If it is not set, discover a usable app from `/api/apps`.
- Agent-board checks iterate over discovered agents instead of hardcoding `backend`, `frontend`, or `architect`.
- The previous `coordinator` assertion becomes a generic "at least one agent is registered" assertion unless an entry-agent contract is exposed later.

### Deprecated API Policy

Deprecated Rust APIs remain available for compatibility and discovery. Upper-layer code must not call them.

This change uses scans as the guard:

```bash
rg -n "start_server\(" macaca/crates
```

Expected result: only the deprecated definition remains.

### Documentation Scope

Only active usage docs should be updated. Historical audit/design documents may continue to mention legacy behavior when they are clearly historical.

## Risks

- Frontend facade can accidentally alter URL construction for EventSource. Mitigation: keep path strings unchanged and centralize only the prefix.
- Generic E2E discovery can reduce fixture specificity. Mitigation: keep `APP_ID` and agent override environment variables possible.
- Docs scan may still find historical `/api/chat` references. Mitigation: distinguish active README/API docs from historical analysis docs.

## Validation

- `openspec validate migrate-macaca-web-consumers-to-pattern-primitives --strict`
- `rg -n "start_server\(" macaca/crates`
- `cargo check -p macaca-cli`
- `cd frontend && npm run lint`
- `bash -n macaca/tests/e2e_project_task.sh`
- `python3 -m py_compile macaca/scripts/trace_watch.py`
