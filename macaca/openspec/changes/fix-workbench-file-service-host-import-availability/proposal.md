# Change: Fix Workbench File Service Host Import Availability

## Why

The app-owned Workbench LLM/tool loop can call `service.llm` and execute
`service.git/git.status` through the existing application UI bridge, but a real
coding task that writes files was initially blocked because
`service.file/file.write` returned a structured unavailable result:

```text
WASM host import service unavailable
```

Follow-up tracing showed that the provider is reachable for simple writes, but
nested project generation was still blocked by a generic `service.file` path
resolution bug: `create_parent_directories=true` could not create missing parent
directories because the local provider canonicalized the missing parent before
the write operation.

This prevents CODEX-WASM-WORKBENCH from completing real frontend/backend
generation tasks through `service.file`, even though the application manifest
declares the service, the host bridge can route model tool calls correctly, and
the service contract explicitly allows parent directory creation.

## What Changes

- Investigate why `service.file` is unavailable through the application host
  import path while other services such as `service.git` are available.
- Confirm whether the existing `service.file` provider is reachable through the
  same app host import route.
- Fix the generic file provider path resolution so `file.write` honors
  `create_parent_directories=true` for nested application workspace files.
- Preserve structured unavailable behavior for deployments where file access is
  intentionally disabled by policy.

## Non-Goals

- Do not add application-specific file generation logic.
- Do not special-case CODEX-WASM-WORKBENCH, Hello World projects, app names, or
  business workflows in Macaca OS.
- Do not bypass file policy, approval, sandbox, trace, audit, or manifest
  capability checks.

## Impact

- Affected specs: `application-runtime-host-provider`,
  `filesystem-process-sandbox`
- Affected code if approved: runtime-host service registration/bootstrap,
  provider wiring, and generic path resolution for `service.file`.
- Current Workbench implementation remains application-owned and unchanged by
  this proposal.
