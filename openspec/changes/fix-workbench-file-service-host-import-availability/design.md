# Design: Workbench File Service Host Import Availability

## Context

`service.git/git.status` succeeds through the app-owned UI bridge for
CODEX-WASM-WORKBENCH, proving the generic `service.call` bridge and LLM/tool
loop can execute service-backed tool calls. A direct bridge call to
`service.file/file.write` originally returned `WASM host import service
unavailable`, which blocked real code generation tasks that need file creation
or patching.

Further tracing narrowed the current blocker: simple `service.file/file.write`
calls reach the registered provider, while nested writes such as
`frontend/src/App.js` fail before side effects when the missing parent directory
is canonicalized. The bug is generic provider behavior, not
CODEX-WASM-WORKBENCH business logic.

## Goals

- Make `service.file` available through the application host import route when
  the application manifest declares the service and policy allows it.
- Ensure `file.write` honors `create_parent_directories=true` for nested files
  under the registered application workspace.
- Keep file side effects owned by `service.file`, not by Web, frontend, WASM
  application code, or CODEX-WASM-WORKBENCH-specific adapters.
- Preserve structured unavailable/denied states when policy or deployment
  intentionally disables file operations.
- Keep trace, audit, approval, sandbox, and memento evidence intact.

## Non-Goals

- No app-specific file templates or coding workflows.
- No shell-owned file semantics.
- No generic fallback to direct filesystem access.

## Architecture

Use the existing service runtime and application host import path:

```text
App-owned UI
  -> service.call bridge
  -> Application Service host dispatch
  -> WASM host import service portal
  -> service.file provider
  -> policy/sandbox/audit/file side effect
```

The fix should be either provider wiring, provider registration, or generic
provider behavior behind the existing `service.file` contract. The application
layer must not change to compensate for unavailable or incorrectly implemented
OS service behavior.

## Validation

- Direct app UI bridge call to `service.file/file.write` succeeds when policy
  allows writing to a declared workspace.
- Direct app UI bridge call to `service.file/file.write` creates missing parent
  directories when the request sets `create_parent_directories=true`.
- Direct app UI bridge call to `service.file/file.read` can read the written
  file with bounded content.
- CODEX-WASM-WORKBENCH real tool loop can create a small frontend/backend sample
  using model-requested `macaca_file` tool calls.
- Unavailable/denied policy cases remain structured and auditable.
