# Change: Migrate macaca-web consumers to pattern primitives

## Why

`macaca-web` now exposes pattern-based startup and web boundary primitives, but upper-layer consumers still duplicate backend API base logic, carry legacy `/api/chat` references, and embed app- or agent-specific test assumptions.

This migration aligns the active CLI, frontend, scripts, tests, and docs with the new `macaca-web` boundary without changing backend HTTP routes or removing deprecated compatibility APIs.

## What Changes

- Keep Rust consumers on `WebServerBuilder` and verify no upper-layer code calls deprecated `start_server`.
- Add a frontend API facade so fetch and EventSource consumers share one web API boundary while preserving current endpoint paths and payloads.
- Update active docs so new consumers do not copy legacy `/api/chat` examples.
- Make active E2E script consumption configurable and generic instead of hardcoding app IDs or concrete agent names.
- Keep deferred session replay migration explicitly out of the first implementation unless it can be done as a behavior-preserving adapter.

## Impact

- Affected specs: `macaca-web-consumer-migration`
- Affected code: `frontend/lib/api.ts`, `frontend/app/page.tsx`, `frontend/next.config.ts`, `macaca/tests/e2e_project_task.sh`, active README/API docs
- Compatibility impact: no backend HTTP route removal; deprecated Rust APIs remain present but must not be called by upper-layer code.
