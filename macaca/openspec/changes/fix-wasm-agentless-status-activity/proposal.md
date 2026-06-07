# Change: Fix WASM agentless status activity

## Why

Agentless WASM sessions can perform host-dispatch work while the visible entry agent remains `IDLE`. This makes the operator surface contradict the active runtime session even though execution is progressing.

## What Changes

- Synchronize the generic entry agent activity with the WASM host-dispatch lifecycle.
- Mark the entry agent `Working` before host dispatch begins.
- Mark the entry agent `Idle` after terminal success or unavailable-style completion.
- Mark the entry agent `Error` when host dispatch returns an execution error.

## Impact

- Affected specs: `session-event-log`
- Affected code: `macaca-web` chat orchestration status synchronization
