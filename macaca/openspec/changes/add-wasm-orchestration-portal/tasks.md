## 1. Specification

- [x] 1.1 Create proposal, design, tasks, and delta spec for `add-wasm-orchestration-portal`.
- [x] 1.2 Validate the OpenSpec change with `openspec validate add-wasm-orchestration-portal --strict`.
- [x] 1.3 Confirm the design uses service/facade boundaries and introduces no architecture exemptions.

## 2. Protocol Contracts

- [x] 2.1 Add `ApplicationImport::AgentDelegate` with canonical import name `macaca:agent/delegate`.
- [x] 2.2 Add provider-neutral Application Service agent delegation command/result DTOs.
- [x] 2.3 Add stable command constants for `application.agent.delegate`.
- [x] 2.4 Add tests for serialization and import category mapping.

## 3. Runtime Host Orchestration Portal

- [x] 3.1 Extend WASM host import bridge dispatch for `macaca:task/create_goal`, `macaca:task/query`, and `macaca:agent/delegate`.
- [x] 3.2 Map task imports to `service.task` commands through ServiceRuntime.
- [x] 3.3 Map agent delegation import to Application Service through ServiceRuntime.
- [x] 3.4 Add detailed English comments and logs at admission, mapping, dispatch, completion, denial, and failure.
- [x] 3.5 Add tests for success, missing session, unavailable service, and policy-denied paths.

## 4. Application Service Agent Delegation

- [x] 4.1 Add a generic `ApplicationOrchestrationBackend` trait owned by runtime-host.
- [x] 4.2 Inject an optional backend into `ApplicationSystemServiceProvider`.
- [x] 4.3 Implement `application.agent.delegate` by validating app/session/agent scope and calling the backend.
- [x] 4.4 Return structured unavailable when no backend is configured.
- [x] 4.5 Add provider tests with a fake backend.

## 5. Web Adapter Wiring

- [x] 5.1 Provide a Web-owned backend adapter that delegates to app-scoped `ApplicationExecutor` without leaking Web state into runtime-host.
- [x] 5.2 Pass the backend into Application Service startup composition.
- [x] 5.3 Change WASM chat dispatch selection so WASM apps with declared agents still execute through WASM host dispatch.
- [x] 5.4 Ensure app-scoped executor and PlanLoop/WorkerLoop startup occurs for declared-agent WASM sessions.

## 6. Verification

- [x] 6.1 Run `cargo fmt -p macaca-proto -p macaca-runtime-host -p macaca-web`.
- [x] 6.2 Run targeted runtime-host tests for WASM orchestration imports.
- [x] 6.3 Run targeted web tests for WASM chat dispatch and executor preparation.
- [x] 6.4 Run the dependency-boundary integration tests that protect microkernel/service/application/shell ownership.
- [x] 6.5 Run `cargo check -p macaca-web`.
- [x] 6.6 Run a live WASM session smoke test through `/api/chat/v2`.
