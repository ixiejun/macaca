# Change: Add WASM lifecycle state checkpoint

## Why
Long-running WASM applications need explicit lifecycle, checkpoint, restore, upgrade, and rollback contracts before Macaca can run them as 7x24 infrastructure workloads. The runtime must fail closed, keep raw guest memory out of portable snapshots, and expose traceable audit metadata without hard-coding application-specific behavior.

## What Changes
- Add provider-neutral WASM lifecycle states, typed transitions, validation rules, and fail-closed reason codes.
- Add sanitized checkpoint/restore memento contracts that carry metadata only and never raw guest memory dumps.
- Add upgrade/rollback reports based on artifact id, artifact hash, and ABI compatibility metadata instead of app names or provider-specific cases.
- Add lifecycle audit events for requested, completed, failed, drained, checkpointed, restored, upgraded, and rolled-back transitions.
- Integrate the default in-process WASM provider with lifecycle dispatch, checkpoint fallback, unsupported pause/resume/drain semantics, and sanitized logs.

## Impact
- Affected specs: `wasm-application-lifecycle`, `wasm-checkpoint-restore`, `wasm-upgrade-rollback`, `wasm-lifecycle-audit`
- Affected code: `macaca-proto` WASM runtime provider DTOs and `macaca-runtime-host` WASM provider modules
