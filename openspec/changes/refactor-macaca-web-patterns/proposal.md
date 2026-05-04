# Change: Refactor macaca-web with pattern-based primitives

## Why

`macaca-web` is the final delivery entry layer and still holds large cross-layer orchestration surfaces: server bootstrap, router construction, chat session orchestration, event forwarding, session replay, traced agent construction adapters, and route handlers.

The previous refactors moved many lower-layer concerns into dedicated crates, but `macaca-web` still needs a single proposal that defines the next web-side pattern primitives and the compatibility path for old entrypoints.

## What Changes

- Add a `WebServerBuilder` and `WebRuntimeFacade` path for web server bootstrap while keeping the old public `start_server` entrypoint as deprecated compatibility.
- Add web-local event forwarding and replay primitives that can later unify SSE, EventLog, and refresh recovery behavior.
- Add web-local chat/session mediation and route command primitives as additive adapters before moving core handlers.
- Keep existing HTTP routes and payloads compatible.
- Mark old direct entrypoints as deprecated and make them compatibility-only delegates where possible, without deleting them.

## Impact

- Affected specs: `macaca-web-patterns`
- Affected code: `macaca-web`, with validation through `macaca-app`, `macaca-kernel`, and integration smoke checks where needed
- Compatibility impact: no public HTTP API removal; old Rust entrypoints remain present but deprecated for migration discovery.
