# Serviceized Agent Execution Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Establish `service.agent_execution` and `service.agent_context` as the serviceized path for agent work, then migrate WASM `agent.delegate` onto it.

**Architecture:** Add provider-neutral service DTOs in `macaca-proto`, service provider shells in `macaca-runtime-host`, and Web-owned built-in backends that reuse the existing framework context/runtime behavior. WASM `macaca:agent/delegate` will dispatch through `service.agent_execution` instead of directly delegating to `ApplicationExecutor`.

**Tech Stack:** Rust workspace, `macaca-proto`, `macaca-runtime-host`, `macaca-web`, ServiceRuntime, OpenSpec.

---

### Task 1: Agent Service Contracts

**Files:**
- Create: `macaca/crates/foundation/macaca-proto/src/agent_execution_service.rs`
- Modify: `macaca/crates/foundation/macaca-proto/src/lib.rs`
- Test: `macaca/crates/foundation/macaca-proto/src/agent_execution_service.rs`

- [x] **Step 1: Add DTOs for service commands and results**

Define constants for `service.agent_execution`, `service.agent_context`, `agent.execute`, and `agent.context.build`. Add `AgentExecutionCommand`, `AgentExecutionResult`, `AgentExecutionStatus`, `AgentContextBuildCommand`, `AgentContextSnapshot`, and supporting source/diagnostic structs. The DTOs must separate `user_prompt` from trusted context and include app/session/task/trace scope.

- [x] **Step 2: Export the module**

Add `pub mod agent_execution_service;` and `pub use agent_execution_service::*;` in `macaca-proto/src/lib.rs`.

- [x] **Step 3: Add contract tests**

Add tests proving commands round-trip with trace, reject empty identity through constructors, and preserve `user_prompt` separately from context snapshot fields.

- [x] **Step 4: Verify**

Run: `cargo test -p macaca-proto agent_execution_service -- --nocapture`

### Task 2: Runtime-Host Service Providers

**Files:**
- Create: `macaca/crates/runtime/macaca-runtime-host/src/agent_context_service_provider.rs`
- Create: `macaca/crates/runtime/macaca-runtime-host/src/agent_execution_service_provider.rs`
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/lib.rs`
- Test: provider unit tests in the new files

- [x] **Step 1: Add backend traits**

Define `AgentContextBackend` and `AgentExecutionBackend` traits in runtime-host provider files. The providers decode `ServiceCommand`, require trace, delegate to injected backends, and return `ServiceCallResult`.

- [x] **Step 2: Add unavailable providers**

Provide `AgentContextSystemServiceProvider::unavailable()` and `AgentExecutionSystemServiceProvider::unavailable()` so absence returns structured `ServiceUnavailable`.

- [x] **Step 3: Add service descriptors**

Add `agent_context_service_descriptor()` and `agent_execution_service_descriptor()` with stable ids, service types, trace schemas, scopes, and metadata.

- [x] **Step 4: Export from runtime-host**

Export provider types, backend traits, descriptors, and service constants from `macaca-runtime-host/src/lib.rs`.

- [x] **Step 5: Verify**

Run: `cargo test -p macaca-runtime-host agent_context_service_provider agent_execution_service_provider -- --nocapture`

### Task 3: Web Built-In Backends

**Files:**
- Create: `macaca/crates/shells/macaca-web/src/agent_context_backend.rs`
- Create: `macaca/crates/shells/macaca-web/src/agent_execution_backend.rs`
- Modify: `macaca/crates/shells/macaca-web/src/lib.rs`
- Modify: `macaca/crates/shells/macaca-web/src/framework_runner.rs`
- Test: targeted web tests where practical

- [x] **Step 1: Make context builder callable by backend**

Expose a crate-visible wrapper around the current context builder so the Web backend can reuse existing persona, skill snapshot, workspace, and tool-policy behavior without copying logic.

- [x] **Step 2: Implement WebAgentContextBackend**

Use `FrameworkRunner` context construction and emit `AgentContextSnapshot` metadata. The first implementation preserves current prompt behavior and records context snapshot evidence.

- [x] **Step 3: Implement WebAgentExecutionBackend**

Call `WebAgentRunner::execute_agent_with_events` or equivalent framework runtime path, passing only `user_prompt` as user input. Return `AgentExecutionResult` with task id, agent, output, status, and trace metadata.

- [x] **Step 4: Register services after AppState creation**

Register and start `service.agent_context` and `service.agent_execution` in Web startup using `StaticServiceProviderFactory` and the Web backends.

- [x] **Step 5: Verify**

Run: `cargo check -p macaca-web`

### Task 4: WASM Delegate Migration

**Files:**
- Modify: `macaca/crates/shells/macaca-web/src/wasm_orchestration_backend.rs`
- Modify: `macaca/crates/runtime/macaca-runtime-host/src/application_service_provider.rs` only if the service result shape requires adaptation
- Test: relevant runtime-host/web tests

- [x] **Step 1: Replace direct executor delegation**

Change `WebApplicationOrchestrationBackend::delegate_agent` to build `AgentExecutionCommand` and call `service.agent_execution` through `ServiceRuntime` or a service-backed client.

- [x] **Step 2: Preserve bounded wait semantics**

Because WASM host command chains need `${host.results.N.output}`, keep the bounded result behavior by returning the service execution result output directly.

- [x] **Step 3: Preserve trace and session metadata**

Use the trace and Web-visible session id carried by the Application Service command. Do not fall back to provider-private WASM session ids.

- [ ] **Step 4: Verify**

Run a WASM `BTC` request and confirm delegate events plus `skill_catalog_built` / `skill_snapshot_created` persist in `/api/sessions/:id/events`.

### Task 5: Boundary Gates And Spec Status

**Files:**
- Modify: `openspec/changes/serviceize-agent-execution-v1/tasks.md`
- Add tests where feasible under touched crates

- [x] **Step 1: Add static/boundary tests**

Add tests preventing WASM delegate from using `ApplicationExecutor::delegate_task` as the final semantic path and proving service ids appear in the dispatch chain.

- [x] **Step 2: Update OpenSpec tasks**

Mark completed first-slice tasks in `openspec/changes/serviceize-agent-execution-v1/tasks.md` only for work actually implemented.

- [x] **Step 3: Run final verification**

Run:
- `cargo fmt`
- `cargo check -p macaca-proto -p macaca-runtime-host -p macaca-web`
- `openspec validate serviceize-agent-execution-v1 --strict`
- `gitnexus_detect_changes(scope: "all")`

- [ ] **Step 4: Commit**

Commit the implementation with a message like `feat: serviceize wasm agent execution`.
