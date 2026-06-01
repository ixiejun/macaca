# Change: Optimize Codex-class application execution replay and WebSocket increments

## Why

The Codex-class Workbench already starts execution through `service.application_execution`, but the app-owned UI should make the durable EventLog contract more explicit: refresh must reconstruct state from backend replay first, and realtime delivery must be a WebSocket observer over persisted backend events rather than a browser-owned trace buffer.

## What Changes

- Add a WebSocket observer endpoint for application execution events that reuses the existing EventLog-backed query and subscription path.
- Update the Codex Workbench UI to replay persisted events before opening realtime transport, then consume WebSocket messages as incremental render-cache updates.
- Keep Codex task semantics inside the application bundle and keep Macaca OS code provider-neutral, application-neutral, traceable, and audited.
- Preserve the existing replay/current-state/control protocol and keep browser-side LLM/tool loops debug-only.

## Impact

- Affected specs: `application-execution-protocol-platform`
- Affected code: `macaca-web` thin adapter routes/tests, Codex Workbench app-owned UI/tests
- Boundary note: no kernel, SDK provider construction, runtime-host provider strategy, or application-specific OS branch is introduced.
