# Change: Unify application execution path

## Why

Macaca currently has multiple production execution paths that can create tasks, emit events, and project terminal state for the same application session. In the latest CODEX-WASM-WORKBENCH session, the WASM `agent_delegate` path completed and wrote files, while a separate compatibility fallback task graph failed and blocked the Task Board, causing the execution projection to report failure.

This is an architecture problem, not a Workbench-specific bug. YAML applications, WASM applications, GenUI applications, headless applications, and app-owned UI applications must all execute through one provider-neutral path.

## What Changes

- Define `service.application_execution` as the only application execution ingress for all application types.
- Define `service.task` as the only owner of execution task graph creation, task lifecycle, review, and task terminal aggregation.
- Define `service.agent_execution` as the only production boundary for agent work started by application execution or task execution.
- Demote Web shell, frontend, app-owned UI, WASM host imports, YAML workflow adapters, and compatibility loop paths to adapters that submit typed commands and render projections.
- Prevent compatibility fallback task graphs from becoming authoritative terminal state for application execution sessions.
- Require one replayable EventLog/audit chain and one current-state projection per execution run.
- Add governance and test gates that reject app-specific, provider-specific, workflow-specific, or business-domain branches below the application layer.

## Impact

- Affected specs: `unified-application-execution-path`
- Affected code:
  - `macaca/crates/foundation/macaca-proto/src/application_execution.rs`
  - `macaca/crates/services/macaca-task/src/*`
  - `macaca/crates/runtime/macaca-runtime-host/src/application_execution_hosted.rs`
  - `macaca/crates/runtime/macaca-runtime-host/src/wasm_runtime_provider/host_import_bridge.rs`
  - YAML application adapter/runtime files under `macaca/crates/application/` and `macaca/crates/runtime/`
  - `macaca/crates/shells/macaca-web/src/loop_manager.rs`
  - `macaca/crates/shells/macaca-web/src/application_execution_routes.rs`
  - `frontend/`
  - `apps/codex-wasm-workbench/ui/src/`

## Constraints

- No application-specific code branches.
- No hardcoded Workbench, Codex, YAML workflow, language, model, provider, driver, gateway, or business-domain logic in OS or generic service code.
- All cross-boundary calls must be typed commands with trace context.
- All side effects must pass policy and capability gates.
- All key execution nodes must emit bounded logs and sanitized trace/audit evidence.
- GitNexus CRITICAL/HIGH warnings are recorded as memo only for this refactor unless they identify a concrete direct correctness issue in edited symbols.
