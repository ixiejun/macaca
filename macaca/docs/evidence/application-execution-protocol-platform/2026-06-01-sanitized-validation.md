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

cargo test -p macaca-integration-tests --test application_execution_workbench_live_proof -- --nocapture
Result: local Workbench Hello World proof passed for macaca_hosted, external_app_backend, and remote_agent provider kinds

npm test
Result: 18 passed, 0 failed

node --check apps/codex-wasm-workbench/ui/app.js
node --check apps/codex-wasm-workbench/ui/render.js
Result: syntax checks passed
```

## Local Workbench Proof

The local proof fixture runs a frontend/backend Hello World task shape through
the generic application execution protocol. It writes generated files only under
the session workspace, records sanitized LLM/tool/file/process/checkpoint and
completion events, drops the browser subscriber before execution completes, and
then reconstructs current state from durable replay using a fresh EventLog
adapter. The same proof is repeated for `macaca_hosted`,
`external_app_backend`, and `remote_agent` provider kinds.

This proof intentionally uses fake/local provider participants because this
repository environment does not contain production credentials or raw provider
payload access. The evidence verifies protocol durability, provider-kind
coverage, replay recovery, workspace confinement, and structured sanitized
observability without leaking raw prompts, generated source contents, secrets,
or provider responses.
