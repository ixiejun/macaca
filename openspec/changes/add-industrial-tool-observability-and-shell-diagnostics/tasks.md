## 1. Event And Audit Surface

- [ ] 1.1 Add planning start/completed events.
- [ ] 1.2 Add hidden diagnostic events.
- [ ] 1.3 Add policy decision events.
- [ ] 1.4 Add approval requested/resolved events.
- [ ] 1.5 Add resource lease acquired/released events.
- [ ] 1.6 Add invocation started/progress/completed/failed/cancelled events.
- [ ] 1.7 Add result persisted and artifact opened events.
- [ ] 1.8 Add provider health changed events.

## 2. API And Shell Routes

- [ ] 2.1 Add Web `tool_routes.rs`.
- [ ] 2.2 Add routes for plan snapshot, provider status, provider health, policy explain, audit query, and artifact metadata.
- [ ] 2.3 Add SDK calls for status, health, policy explain, snapshot, and audit query.
- [ ] 2.4 Add CLI command adapters if CLI exposes tool diagnostics.
- [ ] 2.5 Confirm routes call `SystemToolClient` only.

## 3. Frontend

- [ ] 3.1 Add `ToolCapabilityPanel`.
- [ ] 3.2 Add `ToolInvocationTracePanel`.
- [ ] 3.3 Add `frontend/src/lib/api/tools.ts`.
- [ ] 3.4 Render visible tools, hidden diagnostics, provider health, invocation lifecycle, approval state, artifact refs, and audit refs.
- [ ] 3.5 Ensure frontend does not contain policy or provider lifecycle logic.

## 4. Validation

- [ ] 4.1 Add audit replay tests.
- [ ] 4.2 Add EventLog/SSE sanitization tests.
- [ ] 4.3 Add Web route tests.
- [ ] 4.4 Add shell-boundary tests.
- [ ] 4.5 Add frontend type and lint checks.
- [ ] 4.6 Run `cargo test -p macaca-runtime-host tool_service_audit -- --nocapture`.
- [ ] 4.7 Run `cargo test -p macaca-web tool_routes -- --nocapture`.
- [ ] 4.8 Run `cd frontend && npm run lint`.
- [ ] 4.9 Run `openspec validate add-industrial-tool-observability-and-shell-diagnostics --strict`.
- [ ] 4.10 Run `git diff --check`.

## 5. Governance Notes

- [ ] 5.1 Confirm Web/CLI/frontend remain shells.
- [ ] 5.2 Confirm UI payloads and events are sanitized and bounded.
- [ ] 5.3 Record GitNexus `CRITICAL` and `HIGH` warnings as notes per user instruction.
