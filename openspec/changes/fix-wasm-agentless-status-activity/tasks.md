## 1. Specification

- [x] 1.1 Add proposal, design, tasks, and spec delta.
- [x] 1.2 Validate OpenSpec in strict mode.

## 2. Status Synchronization

- [x] 2.1 Add a focused helper for entry-agent activity updates from chat orchestration.
- [x] 2.2 Mark agentless WASM host-dispatch sessions `Working` before dispatch.
- [x] 2.3 Mark agentless WASM host-dispatch sessions `Idle` or `Error` at terminal dispatch.
- [x] 2.4 Project delegated executor task events into target-agent `Working`/`Idle`/`Error` status.
- [x] 2.5 Add regression coverage that the WASM fast path updates status without app-specific names.

## 3. Validation

- [x] 3.1 Run targeted `macaca-web` tests.
- [x] 3.2 Run `cargo check -p macaca-web`.
- [x] 3.3 Run GitNexus change detection.
