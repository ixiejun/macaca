## 1. Event And Audit Surface

- [x] 1.1 Add planning start/completed events.
- [x] 1.2 Add hidden diagnostic events.
- [x] 1.3 Add policy decision events.
- [x] 1.4 Add approval requested/resolved events.
- [x] 1.5 Add resource lease acquired/released events.
- [x] 1.6 Add invocation started/progress/completed/failed/cancelled events.
- [x] 1.7 Add result persisted and artifact opened events.
- [x] 1.8 Add provider health changed events.

## 2. API And Shell Routes

- [x] 2.1 Add Web `tool_routes.rs`.
- [x] 2.2 Add routes for plan snapshot, provider status, provider health, policy explain, audit query, and artifact metadata.
- [x] 2.3 Add SDK calls for status, health, policy explain, snapshot, and audit query.
- [x] 2.4 Add CLI command adapters if CLI exposes tool diagnostics.
- [x] 2.5 Confirm routes call `SystemToolClient` only.

## 3. Frontend

- [x] 3.1 Add `ToolCapabilityPanel`.
- [x] 3.2 Add `ToolInvocationTracePanel`.
- [x] 3.3 Add `frontend/src/lib/api/tools.ts`.
- [x] 3.4 Render visible tools, hidden diagnostics, provider health, invocation lifecycle, approval state, artifact refs, and audit refs.
- [x] 3.5 Ensure frontend does not contain policy or provider lifecycle logic.

## 4. Validation

- [x] 4.1 Add audit replay tests.
- [x] 4.2 Add EventLog/SSE sanitization tests.
- [x] 4.3 Add Web route tests.
- [x] 4.4 Add shell-boundary tests.
- [x] 4.5 Add frontend type and lint checks.
- [x] 4.6 Run `cargo test -p macaca-runtime-host tool_service_audit -- --nocapture`.
- [x] 4.7 Run `cargo test -p macaca-web tool_routes -- --nocapture`.
- [x] 4.8 Run `cd frontend && npm run lint`.
- [x] 4.9 Run `openspec validate add-industrial-tool-observability-and-shell-diagnostics --strict`.
- [x] 4.10 Run `git diff --check`.

## 5. Governance Notes

- [x] 5.1 Confirm Web/CLI/frontend remain shells.
- [x] 5.2 Confirm UI payloads and events are sanitized and bounded.
- [x] 5.3 Record GitNexus `CRITICAL` and `HIGH` warnings as notes per user instruction.
