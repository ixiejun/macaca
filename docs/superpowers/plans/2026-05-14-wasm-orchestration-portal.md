# WASM Orchestration Portal Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give WASM applications generic access to Macaca task, multi-agent, Skill, MCP, service, and GenUI capabilities through traceable, policy-governed OS boundaries.

**Architecture:** Extend the existing WASM host import bridge as a Facade/Bridge that maps guest task and agent imports into typed ServiceRuntime/Application Service commands. Application Service receives an injected orchestration backend Strategy so runtime-host stays generic and Web remains an adapter.

**Tech Stack:** Rust, macaca-proto DTOs, macaca-runtime-host ServiceRuntime providers, macaca-web app-scoped executor wiring, OpenSpec.

---

### Task 1: Protocol Contracts

**Files:**
- Modify: `/Users/quantum/Code/dev/agent/macaca/crates/foundation/macaca-proto/src/application_abi.rs`
- Modify: `/Users/quantum/Code/dev/agent/macaca/crates/foundation/macaca-proto/src/application_service.rs`
- Modify: `/Users/quantum/Code/dev/agent/macaca/crates/foundation/macaca-proto/src/wasm_runtime_provider/host_import.rs`

- [ ] Add `ApplicationImport::AgentDelegate` with canonical name `macaca:agent/delegate`.
- [ ] Add `APPLICATION_AGENT_DELEGATE_COMMAND`.
- [ ] Add `ApplicationAgentDelegateCommand` and `ApplicationAgentDelegateResult`.
- [ ] Map `ApplicationImport::AgentDelegate` to task/orchestration category.
- [ ] Add focused serialization and category tests.

### Task 2: Host Import Bridge Portal

**Files:**
- Modify: `/Users/quantum/Code/dev/agent/macaca/crates/runtime/macaca-runtime-host/src/wasm_runtime_provider/host_import_bridge.rs`
- Modify: `/Users/quantum/Code/dev/agent/macaca/crates/runtime/macaca-runtime-host/src/wasm_runtime_provider/host_import_tests.rs`

- [ ] Add import dispatch branches for task create, task query, and agent delegate.
- [ ] Validate non-empty app id and session id for orchestration imports.
- [ ] Build Task Service commands with trace, app id, session id, and bounded payload.
- [ ] Route agent delegate through Application Service command.
- [ ] Add detailed English comments explaining the portal boundary and why it keeps WASM app logic generic.
- [ ] Log admission, command mapping, dispatch, completion, denial, and failure.

### Task 3: Application Service Delegation Backend

**Files:**
- Modify: `/Users/quantum/Code/dev/agent/macaca/crates/runtime/macaca-runtime-host/src/application_service_provider.rs`
- Modify: `/Users/quantum/Code/dev/agent/macaca/crates/runtime/macaca-runtime-host/src/lib.rs`

- [ ] Add `ApplicationOrchestrationBackend` trait.
- [ ] Add optional backend field and builder method on `ApplicationSystemServiceProvider`.
- [ ] Implement `application.agent.delegate`.
- [ ] Validate declared app/session/agent scope before invoking the backend.
- [ ] Return structured unavailable when backend is missing.
- [ ] Add fake backend tests.

### Task 4: Web Adapter Wiring

**Files:**
- Modify: `/Users/quantum/Code/dev/agent/macaca/crates/shells/macaca-web/src/lib.rs`
- Modify: `/Users/quantum/Code/dev/agent/macaca/crates/shells/macaca-web/src/chat_orchestrator.rs`

- [ ] Implement a Web-owned backend adapter that delegates through `ApplicationExecutorRegistry`.
- [ ] Pass that backend into Application Service startup composition.
- [ ] Make WASM chat dispatch apply to all L2Wasm apps with runtime ability, including declared-agent apps.
- [ ] Ensure declared-agent WASM sessions prepare app executor and PlanLoop/WorkerLoop before dispatch.
- [ ] Preserve agentless WASM behavior and framework YAML behavior.

### Task 5: Verification

**Files:**
- Test: targeted package tests and governance tests.

- [ ] Run `openspec validate add-wasm-orchestration-portal --strict`.
- [ ] Run `cargo fmt -p macaca-proto -p macaca-runtime-host -p macaca-web`.
- [ ] Run targeted runtime-host WASM host import tests.
- [ ] Run targeted Application Service tests.
- [ ] Run the dependency-boundary integration tests that protect microkernel/service/application/shell ownership.
- [ ] Run `cargo check -p macaca-web`.
- [ ] Restart backend/frontend and run one live WASM session.
