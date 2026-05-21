## 1. Planning and OpenSpec

- [x] 1.1 Write Superpowers implementation plan.
- [x] 1.2 Create OpenSpec proposal, design, tasks, and delta specs.
- [x] 1.3 Run `openspec validate add-manifest-heartbeat-agent-execution --strict`.

## 2. Application Manifest and Projection

- [x] 2.1 Add app-owned `autonomy.heartbeat.agents` manifest DTOs.
- [x] 2.2 Add Application Service heartbeat-agent query command/result DTOs.
- [x] 2.3 Project manifest-declared heartbeat agents through sanitized Application Service views.
- [x] 2.4 Validate unknown heartbeat agent declarations as structured diagnostics.

## 3. Agent Execution Heartbeat Intent

- [x] 3.1 Add provider-neutral `AgentExecutionIntent::Heartbeat`.
- [x] 3.2 Add structured skipped result/status for heartbeat no-op cases.
- [x] 3.3 Require `HEARTBEAT.md` source evidence before heartbeat model/tool invocation.
- [x] 3.4 Add focused Agent Execution tests for heartbeat source-evidence gating.

## 4. Runtime Host Bridge

- [x] 4.1 Add `HeartbeatAgentDispatchStrategy` under runtime-host.
- [x] 4.2 Wire HeartbeatLane to query declarations and dispatch Agent Execution commands after accepted native wakes.
- [x] 4.3 Add sanitized logs for declaration query, dispatch request, completion, skip, and failure.
- [x] 4.4 Preserve Scheduler independence from heartbeat agent dispatch.

## 5. Proof Fixture and Integration

- [x] 5.1 Add heartbeat declaration to the selected local WASM app fixture.
- [x] 5.2 Add integration tests proving declaration-driven heartbeat agent dispatch.
- [x] 5.3 Prove absent declarations and unavailable services return structured evidence.
- [x] 5.4 Verify local WASM `technical_analyst` `HEARTBEAT.md` sentinel behavior.

## 6. Verification

- [x] 6.1 Run formatting and focused compile checks.
- [x] 6.2 Run proto/app/runtime-host/web focused tests.
- [x] 6.3 Run autonomy integration tests.
- [x] 6.4 Run serviceization escape-hatch and dependency-boundary gates.
- [x] 6.5 Run `openspec validate add-manifest-heartbeat-agent-execution --strict` after implementation.
