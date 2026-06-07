## 1. Contract And Governance

- [x] 1.1 Run GitNexus impact analysis for the current pause/resume symbols and service-agent execution builders.
- [x] 1.2 Add provider-neutral execution-control DTOs for policy, trigger, resume source, state, command, result, event, and snapshot.
- [x] 1.3 Add serialization and deterministic merge tests for manifest defaults plus command overrides.
- [x] 1.4 Update architecture governance or serviceization allowlist if ownership wording needs to reference `service.execution_control`.

## 2. Stage 1 Runtime Capability

- [x] 2.1 Implement `ExecutionControlPolicyResolver` with explicit denied, unsupported, disabled, and enabled outcomes.
- [x] 2.2 Implement the built-in runtime execution-control capability using Command, Strategy, State, Observer, and Memento boundaries.
- [x] 2.3 Wire `service.agent_execution` to install execution-control adapters for any execution whose resolved policy enables them.
- [x] 2.4 Replace chat-main-thread-specific pause wiring with policy-driven execution control.
- [x] 2.5 Preserve existing goal, planner, worker, review, workflow, YAML, WASM, and chat behavior through compatibility tests.

## 3. Application Selection

- [x] 3.1 Add application manifest/application metadata fields for default execution-control policy.
- [x] 3.2 Add bounded `AgentExecutionCommand` override support for per-run execution-control policy.
- [x] 3.3 Add policy tests proving command overrides can narrow declared defaults and cannot exceed app-declared permissions.
- [x] 3.4 Add no-hardcoding tests for application name, agent name, workflow name, provider name, and driver name branches.

## 4. Trace, Audit, And Diagnostics

- [x] 4.1 Emit sanitized execution-control events for policy resolution, pause, checkpoint, resume, timeout, cancellation, and state query.
- [x] 4.2 Append durable EventLog/RunTrace evidence before live streaming or UI diagnostics.
- [x] 4.3 Add audit replay tests for a goal pause/resume run and an ordinary app-selected runtime pause/resume run.
- [x] 4.4 Add log assertions or snapshot tests for key fields and sensitive-data redaction.

## 5. Stage 2 Serviceization

- [x] 5.1 Register `service.execution_control` with descriptor, lifecycle, health, snapshot, and unavailable provider.
- [x] 5.2 Expose typed commands for resolve policy, register execution, request pause, record checkpoint, await resume, request resume, cancel wait, query state, and snapshot.
- [x] 5.3 Route runtime execution-control operations through `ServiceRuntime.call(service.execution_control, ...)`.
- [x] 5.4 Add policy, resource, entitlement, metering placeholder, trace-required, and audit decorators before side effects.
- [x] 5.5 Preserve stage-1 behavior after service routing replaces direct capability calls.

## 6. Deprecation And Verification

- [x] 6.1 Deprecate path-specific pause/resume helpers after service-backed execution control is active.
- [x] 6.2 Add static or unit gates blocking new direct session-channel pause/resume ownership outside approved adapters.
- [x] 6.3 Run targeted crate tests for proto, runtime-host, web, app, task, and integration boundaries.
- [x] 6.4 Run `openspec validate add-execution-control-service-v1 --strict`.
- [x] 6.5 Run dependency-boundary, trace/audit replay, and full workspace check commands.
