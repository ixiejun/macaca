# Application Execution Protocol Platform Validation Evidence

Date: 2026-06-01

## Scope

This evidence records sanitized local validation for the provider-neutral
application execution protocol platform. It contains no raw provider payloads,
secrets, prompts, package bytes, or workspace file contents.

## Provider Coverage

- `macaca_hosted`: validated through a fake hosted adapter that emits durable
  execution events, waits for approval, receives approval through typed control,
  appends completion, and reconstructs terminal state from replay.
- `external_app_backend`: existing fake backend tests validated start, gateway
  callback ingress, approval, completion, duplicate callback handling, invalid
  callback identity rejection, and heartbeat timeout behavior.
- `remote_agent`: validated through a fake remote transport with registration,
  capability/tenant/lease selection, scoped lease issue, start dispatch, control
  delivery, stale lease rejection, and accepted heartbeat gateway append.

## Frontend Boundary

- The CODEX-WASM-WORKBENCH production UI now starts execution through
  `/api/apps/{app_id}/execution/start`.
- It subscribes to persisted events through
  `/api/apps/{app_id}/execution/events`.
- It reconstructs state through replay/current-state routes.
- Browser-side LLM/tool loop code remains available only behind the
  `debug_tool_loop=1` query flag and is excluded from production validation.
- UI event arrays are render caches rebuilt from replay, not durable state.

## Commands

```text
openspec validate add-application-execution-protocol-platform --strict
Result: valid

cargo test -p macaca-runtime-host application_execution_ -- --nocapture
Result: 39 passed, 0 failed

cargo test -p macaca-integration-tests application_execution_ -- --nocapture
Result: application execution scope-control, shell-ownership, and dependency-boundary gates passed

npm test
Result: 18 passed, 0 failed

node --check apps/codex-wasm-workbench/ui/app.js
node --check apps/codex-wasm-workbench/ui/render.js
Result: syntax checks passed
```

## Deferred Real-World Proof

The repository-level fake/local tests prove the generic protocol and boundary
behavior. A full live Workbench Hello World execution was not run because this
local validation environment does not provide a configured live LLM/tool
provider stack and runtime-host application execution provider stack. That
live proof remains the only non-unit evidence gap.
