## 0. Pre-Implementation Gates

- [x] 0.1 Read `docs/application-execution-protocol-platform-brainstorm.md` and confirm the implementation scope includes all three provider kinds: `macaca_hosted`, `external_app_backend`, and `remote_agent`.
- [x] 0.2 Re-read `macaca/docs/macaca-os-architecture-governance.md`, `macaca/docs/macaca-os-microkernel-boundaries.md`, and `macaca/docs/macaca-os-serviceization-allowlist.md`; record in the implementation notes which layer owns each new behavior.
- [x] 0.3 Run `openspec validate add-application-execution-protocol-platform --strict` before code work and keep the change valid throughout implementation.
- [x] 0.4 Run `openspec list` and confirm no active change already owns `service.application_execution` or the same application execution protocol platform.
- [x] 0.5 Run GitNexus impact analysis before editing any existing symbol; record direct callers, affected processes, and risk in the implementation notes. HIGH/CRITICAL findings are allowed only as notes if they are broad existing debt, but direct correctness risks for edited symbols must be addressed.

## 1. Protocol DTO Foundation

- [x] 1.1 Add a provider-neutral protocol module under the foundation/proto crate, expected path `macaca/crates/foundation/macaca-proto/src/application_execution.rs`, and wire it from the crate root without importing runtime-host, Web, CLI, frontend, provider, or application package code.
- [x] 1.2 Define `ApplicationExecutionProviderKind` with exactly these stable variants: `MacacaHosted`, `ExternalAppBackend`, `RemoteAgent`, and `Unavailable`; include serde round-trip tests.
- [x] 1.3 Define `ApplicationExecutionLifecycleState` for session/run state transitions: `Accepted`, `AssigningProvider`, `Running`, `WaitingForApproval`, `Paused`, `Resuming`, `Cancelling`, `Completed`, `Failed`, and `Cancelled`; include tests proving terminal states are recognized.
- [x] 1.4 Define `ApplicationExecutionEventType` covering session, execution, provider, LLM, tool, approval, checkpoint, control, completion, failure, and cancellation event families listed in `design.md`.
- [x] 1.5 Define `StartApplicationExecutionCommand` with application id, session id, optional run id, task input or payload ref, workspace ref, requested capabilities, provider preference, trace context, policy context, tenant id, actor, and idempotency key.
- [x] 1.6 Define `StartApplicationExecutionResult` with `accepted`, `denied`, `unavailable`, `unsupported`, and `failed` outcomes plus session id, run id, provider id, provider kind, event cursor, control ref, workspace ref, and structured error.
- [x] 1.7 Define `ApplicationExecutionEventEnvelope` with application id, session id, run id, sequence, timestamp, event type, trace id, actor, provider id, provider kind, visibility, causality, sanitized payload, optional payload ref, schema version, and idempotency key.
- [x] 1.8 Define `ApplicationExecutionControlCommand` and `ApplicationExecutionControlResult` for `cancel`, `approve`, `reject`, `pause`, `resume`, `retry`, and `inject_input`; require idempotency key, actor, reason code, trace context, policy context, and bounded payload or payload ref.
- [x] 1.9 Define `ApplicationExecutionProviderDescriptor` with provider id, provider kind, protocol version, supported commands, supported events, checkpoint support, heartbeat policy, control delivery, capability declarations, resource profile, transport kind, and health state.
- [x] 1.10 Define `ApplicationExecutionProviderLease` for remote/external participants with lease id, provider id, session id, run id, expiry, heartbeat deadline, scoped callback identity reference, allowed event types, and allowed controls.
- [x] 1.11 Define gateway ingress commands/results: `AppendExecutionEventCommand`, `ReportExecutionHeartbeatCommand`, `ReportExecutionSnapshotCommand`, `RequestExecutionApprovalCommand`, `ReportExecutionCompletionCommand`, and `ReportExecutionFailureCommand`.
- [x] 1.12 Define `ApplicationExecutionSnapshot` and `ApplicationExecutionCurrentState` projections with run lifecycle, assigned provider, latest heartbeat, pending approvals, active controls, latest checkpoint, summarized LLM/tool steps, terminal result, terminal error, and replay cursor.
- [x] 1.13 Define `ApplicationExecutionError` with stable codes: unavailable, disabled, denied, unsupported, invalid_schema, invalid_state, duplicate, stale_lease, timeout, provider_failed, policy_denied, resource_denied, entitlement_denied, and sanitization_failed.
- [x] 1.14 Add detailed English comments to every public DTO explaining what owns it, how it crosses boundaries, what must be sanitized, and why it cannot contain application-specific business logic.
- [x] 1.15 Add focused proto tests for serde round trips, schema version preservation, idempotency key preservation, terminal state helpers, and structured error serialization.

## 2. SDK and SystemFacade Client Contract

- [x] 2.1 Add an application execution focused client under `macaca/crates/facade/macaca-sdk/src/`, following existing workbench/service client patterns and without constructing runtime providers.
- [x] 2.2 Implement typed methods: `start_execution`, `send_control`, `append_gateway_event` only for authorized gateway contexts, `query_current_state`, `replay_events`, `provider_health`, and `snapshot`.
- [x] 2.3 Add a Null Object/unavailable client that returns structured unavailable for every method and records sanitized trace context when supplied.
- [x] 2.4 Expose the client through the appropriate SDK/SystemFacade assembly path so shells can obtain it without importing runtime-host internals.
- [x] 2.5 Add SDK tests proving missing service returns unavailable, command DTOs preserve trace/session/application scope, and the SDK does not branch on provider kind except to carry typed hints.

## 3. Service Runtime Boundary

- [x] 3.1 Add `service.application_execution` constants, descriptor, command names, health payloads, and snapshot payloads in the foundation/service contract area.
- [x] 3.2 Add an unavailable `SystemService` implementation for `service.application_execution` in runtime-host that returns structured unavailable for every command.
- [x] 3.3 Register the unavailable provider in the runtime-host composition root first, then add a bootstrap option for enabling the built-in provider stack.
- [x] 3.4 Add service descriptor tests proving the service id, command surface, health, lifecycle, and snapshot are deterministic.
- [x] 3.5 Add ServiceRuntime call tests proving missing trace is rejected before dispatch, policy denial stops side effects, and unknown commands return structured unsupported.
- [x] 3.6 Add structured logs at service registration, start, command receive, policy reject, provider assignment, event append, control route, gateway ingress, snapshot, and failure nodes.
- [x] 3.7 Ensure all logs use bounded sanitized fields only and include application id, session id, run id, provider id/kind, command name, trace id, status, and reason code when available.

## 4. EventLog Persistence and Replay

- [x] 4.1 Identify the existing session/EventLog append and replay APIs used by chat, GenUI, trace, and session recovery; reuse them instead of adding a parallel event store.
- [x] 4.2 Add an `ApplicationExecutionEventStore` adapter owned by runtime-host or persistence service boundary that persists `ApplicationExecutionEventEnvelope` values into the existing durable event mechanism.
- [x] 4.3 Implement idempotent append by `(application_id, session_id, run_id, idempotency_key)` and stable sequence assignment.
- [x] 4.4 Implement schema validation before append; reject invalid event type, missing trace/session/run, stale lease, unsupported schema version, and oversized inline payloads.
- [x] 4.5 Implement payload sanitization before append; use bounded summaries, hashes, redacted fields, or `payload_ref` for sensitive/large content.
- [x] 4.6 Implement replay by session/run with from-start, from-cursor, page size, visibility filter, and event type filter.
- [x] 4.7 Implement current-state projection as a deterministic reducer over replayed events; include pending approvals, active controls, provider heartbeat, latest checkpoint, and terminal outcome.
- [x] 4.8 Add tests proving duplicate appends return original event/cursor, replay ordering is deterministic, projection is stable after refresh, and unsafe payloads are rejected or redacted.

## 5. Provider Strategy Registry

- [x] 5.1 Define the `ApplicationExecutionProvider` trait in runtime-host or an approved application execution provider module with `describe`, `start`, `control`, `health`, `snapshot`, `resume`, and `shutdown`.
- [x] 5.2 Implement an `ApplicationExecutionProviderRegistry` that registers providers by descriptor and selects providers by manifest execution profile, provider preference, capability availability, policy, tenant constraints, and health.
- [x] 5.3 Implement a provider selection result that records all considered providers, rejection reasons, selected provider id/kind, trace id, and policy status without leaking secrets.
- [x] 5.4 Add admission validation for provider descriptor version, supported commands, supported events, heartbeat policy, checkpoint support, capability declarations, resource profile, and transport kind.
- [x] 5.5 Add tests proving selection never branches on application id, workflow name, model name, driver name, gateway name, or business domain.
- [x] 5.6 Add structured unavailable provider behavior when no provider is registered, no provider supports the required capability, policy denies all providers, or a selected provider is unhealthy.

## 6. `macaca_hosted` Provider

- [ ] 6.1 Implement the `macaca_hosted` provider as a runtime-host provider strategy that loads application execution through existing application ABI/runtime-host seams.
- [ ] 6.2 Start backend-owned execution tasks with cancellation tokens, run/session/workspace envelope, trace context, and service-call capability scope.
- [ ] 6.3 Route application service calls through declared capabilities and `ServiceRuntime`; never call concrete LLM/file/process/sandbox/tool providers directly from application-specific code.
- [ ] 6.4 Append `provider.assigned`, `execution.accepted`, `provider.heartbeat`, lifecycle, LLM/tool summary, approval, checkpoint, completion, failure, and cancellation events at key execution nodes.
- [ ] 6.5 Implement control handling for cancel, approve, reject, pause, resume, retry, and inject_input using generic wait handles/state transitions.
- [ ] 6.6 Persist checkpoints before long waits and before graceful shutdown when the application/runtime supports checkpointing.
- [ ] 6.7 Return structured unavailable or unsupported when the application runtime, ABI export, required host import, or service dependency is missing.
- [ ] 6.8 Add unit tests using a fake hosted app adapter that emits events, blocks on approval, resumes after approval, handles cancel, and completes without browser participation.
- [ ] 6.9 Add integration tests proving `macaca_hosted` execution continues after the shell subscriber disconnects and replay reconstructs state after reconnect.

## 7. `external_app_backend` Provider

- [ ] 7.1 Extend application manifest parsing or execution profile metadata to declare an external backend endpoint, protocol version, callback identity reference, supported controls, heartbeat interval, timeout, and event schema version.
- [ ] 7.2 Implement an external backend provider adapter that validates the manifest declaration before provider registration.
- [ ] 7.3 On start, call the backend start endpoint with application id, session id, run id, workspace ref, task input/payload ref, callback gateway URL/ref, scoped callback identity reference, allowed event types, allowed controls, heartbeat policy, trace context, and idempotency key.
- [ ] 7.4 Store provider assignment and lease metadata before returning accepted.
- [ ] 7.5 Implement gateway ingress for external backend callbacks: append_event, report_heartbeat, report_snapshot, request_approval, report_completion, and report_failure.
- [ ] 7.6 Validate callback identity, session/run binding, lease validity, event schema version, idempotency key, allowed event type, payload size, and sanitization before appending any event.
- [ ] 7.7 Implement control forwarding to the backend control endpoint with command idempotency, timeout, retry policy where safe, structured delivery result, and audit evidence.
- [ ] 7.8 Implement heartbeat timeout behavior that marks the provider stale, appends structured failure when required, and exposes diagnostics through health/snapshot.
- [ ] 7.9 Add tests with a fake external backend that starts, writes events through gateway, requests approval, receives approval, completes, handles duplicate callbacks, rejects invalid signatures, and times out on heartbeat loss.

## 8. `remote_agent` Provider

- [ ] 8.1 Define remote agent registration DTOs and provider descriptors using the same provider protocol fields plus remote transport metadata, lease support, capability declarations, and heartbeat policy.
- [ ] 8.2 Implement a remote agent registry that tracks registered agents, health, capabilities, resource profile, tenant/region constraints, current leases, and last heartbeat.
- [ ] 8.3 Implement provider selection against remote agents using capability match, health, resource policy, tenant/region constraints, and lease availability.
- [ ] 8.4 Issue scoped execution leases with expiry, allowed event types, allowed controls, callback identity reference, and trace context.
- [ ] 8.5 Dispatch start commands over the registered remote transport adapter without importing a concrete remote-agent implementation into the kernel or shell.
- [ ] 8.6 Implement remote gateway ingress for heartbeat, event append, snapshot, approval request, completion, and failure using the same validation and sanitization rules as external backends plus lease validation.
- [ ] 8.7 Implement control delivery over the remote control channel and record control.requested, control.delivered, control.completed, or structured failure.
- [ ] 8.8 Implement stale lease expiry, heartbeat miss handling, provider failure projection, and checkpoint-based resume when supported by descriptor and policy.
- [ ] 8.9 Add tests with fake remote agents for registration, provider selection, lease issue, event append, heartbeat, control, stale lease rejection, and resume from checkpoint.

## 9. Service Commands and API Adapters

- [ ] 9.1 Add service command dispatch for start_execution, send_control, replay_events, query_current_state, provider_health, snapshot, gateway append_event, gateway heartbeat, gateway snapshot, gateway approval request, gateway completion, and gateway failure.
- [ ] 9.2 Add Web route adapters only as thin shells over SDK/SystemFacade or focused clients; expected routes may include start, replay, current-state, control, and gateway ingress endpoints.
- [ ] 9.3 Ensure Web routes parse HTTP/SSE input, call typed clients, map structured results to HTTP/SSE, and never run provider loops or own authoritative execution events.
- [ ] 9.4 Ensure realtime/SSE subscriptions stream persisted events or durable event references after EventLog append.
- [ ] 9.5 Add route tests proving invalid input returns structured errors, missing trace/session/app scope is rejected, and browser disconnect does not cancel backend execution unless a control command requests cancel.
- [ ] 9.6 Add gateway route tests proving external callbacks cannot append events without valid identity, lease, schema, idempotency, and sanitization.

## 10. Frontend and App-Owned UI Boundary

- [ ] 10.1 Update frontend/app-owned UI bridge contracts so application UIs can start execution, subscribe/replay, query current state, and send control commands through Macaca APIs.
- [ ] 10.2 Remove or demote any production browser-owned LLM/tool loop path to debug-only mode; production Workbench execution must use `service.application_execution`.
- [ ] 10.3 Ensure UI-local arrays such as timeline/event buffers are render caches only and are never described or used as durable source of truth.
- [ ] 10.4 Add reconnect behavior: when the iframe/app UI mounts with a session id, it queries current state and replays events from the stored cursor.
- [ ] 10.5 Add cancel/approve/reject/resume UI actions that send typed control commands and render control outcomes from the event stream.
- [ ] 10.6 Add frontend tests proving the UI starts a task, disconnects/reconnects, replays events, renders pending approval, sends approval, sends cancel, and never calls external app backend authoritative state endpoints directly.

## 11. CODEX-WASM-WORKBENCH Proof Without OS-Specific Logic

- [ ] 11.1 Update `apps/codex-wasm-workbench` manifest to declare the generic application execution bridge/provider requirements required by the new protocol.
- [ ] 11.2 Move production execution entry to a backend-owned provider path. Any browser-side loop must be clearly marked debug-only and excluded from production validation.
- [ ] 11.3 Ensure Workbench uses generic LLM, file, process, sandbox, approval, tool/MCP, diagnostics, realtime, and session services through declared capabilities.
- [ ] 11.4 Validate that generated code tasks write only into the app/session workspace resolved by Macaca configuration, not arbitrary developer paths.
- [ ] 11.5 Run a real Workbench task that asks for a frontend+backend Hello World project; verify LLM/tool execution, file writes, process/test execution, event persistence, and final result through the generic protocol.
- [ ] 11.6 Start the Workbench task, close or disconnect the browser subscriber, wait for backend progress, reopen the UI, and verify replay/current-state reconstruction from persisted events.
- [ ] 11.7 Repeat the proof using each provider kind: `macaca_hosted`, fake or local `external_app_backend`, and fake or local `remote_agent`.
- [ ] 11.8 Record proof artifacts under an appropriate docs/evidence path with sanitized payloads and no secrets or raw provider payloads.

## 12. Dependency, Governance, and Boundary Gates

- [x] 12.1 Add dependency-boundary tests proving kernel crates do not depend on concrete application execution providers.
- [x] 12.2 Add dependency-boundary tests proving SDK/SystemFacade does not construct runtime-host providers, Web state, CLI state, database backends, or app runtimes.
- [ ] 12.3 Add dependency-boundary tests proving Web/CLI/frontend do not own application execution semantics, provider lifecycle, approval/cancel semantics, or EventLog persistence.
- [x] 12.4 Add tests or static guards rejecting application-specific branches in generic service/runtime code, including branches on `codex-wasm-workbench`, Codex names, workflow names, model names, provider names, driver names, gateway names, and business-domain identifiers.
- [ ] 12.5 Update `macaca/docs/macaca-os-serviceization-allowlist.md` only if this implementation adds, removes, or narrows a real dependency exception; do not update it cosmetically.
- [ ] 12.6 Update `macaca/docs/macaca-os-architecture-governance.md` and `macaca/docs/macaca-os-microkernel-boundaries.md` only if this implementation adds stable ownership language that is not already covered.
- [x] 12.7 Add detailed English comments to new Rust code explaining protocol ownership, provider strategy, adapter boundaries, trace/audit behavior, sanitization, failure modes, and non-goals.

## 13. Verification

- [x] 13.1 Run `openspec validate add-application-execution-protocol-platform --strict`.
- [x] 13.2 Run targeted proto/foundation tests for application execution DTOs and schema validation.
- [x] 13.3 Run targeted SDK tests for unavailable client behavior and typed command forwarding.
- [ ] 13.4 Run targeted runtime-host tests for service descriptor, provider registry, EventLog append/replay, current-state projection, control idempotency, gateway ingress, and provider strategies.
- [ ] 13.5 Run integration tests for all three provider kinds.
- [ ] 13.6 Run Web route tests for start, replay, current state, control, gateway ingress, and subscriber disconnect behavior.
- [ ] 13.7 Run frontend tests for app-owned UI start/subscribe/replay/control rendering.
- [x] 13.8 Run dependency-boundary tests required by Route C governance.
- [ ] 13.9 Run the real Workbench validation from Task 11 and record sanitized evidence.
- [x] 13.10 Run `npx gitnexus detect-changes` where supported, or document the local GitNexus CLI limitation and use available GitNexus impact/context/status checks before commit.

## 14. Completion

- [ ] 14.1 Confirm every task above is complete or explicitly moved to a follow-up proposal with a blocking reason accepted by the user.
- [ ] 14.2 Confirm no implementation code below the application layer contains application-specific business logic or hardcoded Workbench/Codex branches.
- [ ] 14.3 Confirm all provider-unavailable, policy-denied, invalid-schema, stale-lease, timeout, duplicate, and unsupported paths return structured results.
- [ ] 14.4 Confirm browser-close survival and replay/current-state recovery are proven by evidence, not inferred from unit tests alone.
- [ ] 14.5 Commit the OpenSpec, implementation, tests, docs, and sanitized evidence in reviewable commits.
