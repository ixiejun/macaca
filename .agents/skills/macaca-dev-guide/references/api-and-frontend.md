# API And Frontend Reference

This is a lightweight orientation note. Verify route names and payloads in
current `macaca/crates/shells/macaca-web/src/` and `frontend/` before editing.
The shell layer should express the stable thin-shell model, not historical
serviceization transition mechanics.

## Shell Responsibilities

Web, CLI, and frontend code may:

- Parse user, browser, gateway, or CLI input.
- Convert input into `SystemFacade`, focused SDK client, or service commands.
- Render chat, GenUI surfaces, approvals, traces, diagnostics, and history.
- Subscribe to event/trace streams and replay persisted history.
- Surface autonomous progress, recoverable state, human approval requests, and
  clear escalation reasons without turning the UI into the task executor.

They must not permanently own task planning, worker execution, provider
construction, payment/package semantics, driver/skill/MCP lifecycle, or
application runtime semantics.

## Local Endpoints And Ports

- Frontend dev server: `http://localhost:3000`
- Rust API server: `http://localhost:3001`
- `GET /` on port `3001` returning the API-server JSON 404 is expected.
- Frontend uses `NEXT_PUBLIC_API_BASE` when set; otherwise local development
  usually points at port `3001`.

## Route Families To Inspect

Look in `macaca/crates/shells/macaca-web/src/` for current route ownership:

- Chat/session/event-log routes and `/api/chat/v2`.
- App, package, manifest, app-owned UI, and GenUI routes.
- Task/goal/todo route adapters.
- Service inspection and service-call adapters.
- Trace/audit/event replay bridges.
- Stop/cancel/TERMINATE adapters.

Prefer route-command or service-command helpers when they already exist. New
routes should be thin adapters around typed commands rather than new semantic
owners.

## Frontend Shape

`frontend/` is a presentation shell. It should keep:

- Application discovery and shell navigation.
- Chat/session surfaces where relevant.
- GenUI mounting for application-owned center experiences.
- Trace, audit, diagnostics, approval, and task visibility.

It should not embed business-domain workflows or provider-specific control
branches. If a workflow belongs to an application, keep it app-owned. If it is
generic OS behavior, route it through an SDK/service boundary.

For 24/7 autonomous operation, frontend views should make state inspectable and
recoverable: goals, current plan, task state, execution trace, approval gates,
retry/escalation reason, and final evidence. The user should not need to keep
prompting the agent just to continue routine work.

## Event And Trace Rules

- Persist event/audit evidence before streaming it to the UI.
- Use stable dedupe keys for history replay plus live updates.
- Keep payloads sanitized and bounded.
- Raw prompts, secrets, manifests, WASM/package bytes, provider payloads,
  private keys, signatures, and credentials must not appear in UI diagnostics.

## Frontend Verification

```bash
cd frontend
npm run lint
npx tsc --noEmit
npx next dev --port 3000
```

After significant frontend changes, open the local app in the in-app browser and
check both desktop and narrow viewport behavior.
