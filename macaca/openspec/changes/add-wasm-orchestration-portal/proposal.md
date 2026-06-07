# Change: Add WASM Orchestration Portal

## Why

WASM applications currently execute through an agentless host-dispatch fast path and can call generic services such as `service.call` and `ui.render`, but they do not yet have a first-class, policy-governed way to use Macaca OS task, multi-agent, skill, and MCP capabilities with the same baseline power as YAML applications.

Macaca is an Agent OS, so an L2Wasm application must be able to orchestrate goals, tasks, agent delegation, MCP tools, skills, and UI surfaces through generic OS contracts. The guest may choose a more flexible orchestration strategy than a YAML workflow, but it must not bypass ServiceRuntime, application policy, trace/audit, session persistence, or app-scoped executor isolation.

## What Changes

- Add a WASM Orchestration Portal that maps WASM task and agent orchestration imports to typed, trace-required, policy-governed service commands.
- Register app-scoped executor/PlanLoop/WorkerLoop support for WASM applications that declare application agents, instead of forcing every WASM app into the agentless fast path.
- Route WASM task creation/query and agent delegation through generic OS services and existing app-scoped execution primitives.
- Preserve `service.call` as the generic path for LLM, Skill, MCP, Driver, Memory, Finance, and other replaceable services.
- Add audit-friendly command/result metadata and logs for every WASM orchestration decision, including allowed, denied, unavailable, delegated, completed, and failed states.
- Keep all behavior application-agnostic: no app names, workflow names, symbols, domain-specific payloads, or business logic in OS crates.

## Impact

- Affected specs: `wasm-orchestration-portal` (new), existing WASM host import service portal behavior, existing Application Service session behavior.
- Affected code:
  - `macaca/crates/foundation/macaca-proto/src/application_abi.rs`
  - `macaca/crates/foundation/macaca-proto/src/wasm_runtime_provider/host_import.rs`
  - `macaca/crates/foundation/macaca-proto/src/application_service.rs`
  - `macaca/crates/foundation/macaca-proto/src/lib.rs`
  - `macaca/crates/runtime/macaca-runtime-host/src/wasm_runtime_provider/host_import_bridge.rs`
  - `macaca/crates/runtime/macaca-runtime-host/src/application_service_provider.rs`
  - `macaca/crates/runtime/macaca-runtime-host/src/lib.rs`
  - `macaca/crates/shells/macaca-web/src/chat_orchestrator.rs`
  - `macaca/crates/shells/macaca-web/src/lib.rs`
  - `macaca/crates/services/macaca-task/src/service_adapter.rs`
  - targeted tests in `macaca-runtime-host`, `macaca-web`, and `macaca-integration-tests`
- Compatibility: YAML applications, `/api/chat/v2`, session logs, task board, GenUI rendering, service-call audit replay, and current crypto/stock WASM apps must remain compatible.

## Non-Goals

- Do not make `macaca-web` the long-term owner of orchestration semantics.
- Do not put task, MCP, skill, driver, or LLM provider logic into the WASM runtime provider.
- Do not introduce application-specific or domain-specific branching.
- Do not require every WASM app to declare agents; agentless WASM apps must continue to work.
- Do not implement real external MCP transport or skill marketplace behavior beyond the existing service boundaries.
