## 1. OpenSpec

- [x] 1.1 Create proposal, design, tasks, and spec.
- [x] 1.2 Validate `migrate-macaca-web-consumers-to-pattern-primitives` with `--strict`.

## 2. Inventory and Impact

- [x] 2.1 Re-scan deprecated `start_server` usage.
- [x] 2.2 Review frontend API, Next rewrite, active E2E script, trace script, and active docs.
- [x] 2.3 Run GitNexus impact before Rust symbol edits, if any Rust symbol is modified.

## 3. Rust Consumer Guard

- [x] 3.1 Keep `macaca-cli` on `WebServerBuilder`.
- [x] 3.2 Ensure no upper-layer Rust code calls deprecated `start_server`.

## 4. Frontend API Facade

- [x] 4.1 Add a lightweight `MacacaApiClient` facade for API base, fetch URL, EventSource URL, and JSON fetch.
- [x] 4.2 Keep existing `frontend/lib/api.ts` exported functions as compatibility delegates.
- [x] 4.3 Preserve every current endpoint path and payload shape.
- [x] 4.4 Update frontend user-facing backend base text to avoid hardcoded port-only guidance.

## 5. E2E and Script Consumers

- [x] 5.1 Make `e2e_project_task.sh` use `MACACA_API`/`BASE` fallback consistently.
- [x] 5.2 Make `APP_ID` optional with discovery from `/api/apps`.
- [x] 5.3 Replace fixed `backend`/`frontend`/`architect` board checks with discovered-agent checks.
- [x] 5.4 Replace required `coordinator` assertion with a generic agent availability assertion.
- [x] 5.5 Keep `trace_watch.py` compatible and validate syntax.

## 6. Active Docs

- [x] 6.1 Update active README/API docs so legacy `/api/chat` is not recommended.
- [x] 6.2 Leave historical audit/design docs intact unless they are written as current instructions.

## 7. Verification

- [x] 7.1 Run `openspec validate migrate-macaca-web-consumers-to-pattern-primitives --strict`.
- [x] 7.2 Run deprecated API scans.
- [x] 7.3 Run `cargo check -p macaca-cli`.
- [x] 7.4 Run `cd frontend && npm run lint`.
- [x] 7.5 Run `bash -n macaca/tests/e2e_project_task.sh`.
- [x] 7.6 Run `python3 -m py_compile macaca/scripts/trace_watch.py`.
- [x] 7.7 Run GitNexus detect-changes and review affected scope.
