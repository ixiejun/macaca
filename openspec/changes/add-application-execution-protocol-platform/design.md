# Design: Application Execution Protocol Platform

## Context

Macaca OS already has many of the lower-level building blocks required for industrial application execution: ServiceRuntime, application ABI, WASM runtime provider contracts, workbench services, LLM model selection, execution control, approval, session events, EventLog, and thin-shell governance. The missing architectural piece is a protocol platform that turns those building blocks into a durable, provider-neutral application execution plane.

The immediate symptom came from `CODEX-WASM-WORKBENCH`: an app UI can initiate work, display events, and call services, but a real Codex-class application cannot rely on browser JavaScript to own the LLM/tool loop, write the authoritative event stream, or keep running after the tab closes. That symptom generalizes to every complex application. A Macaca application must be able to run through Macaca-hosted backend execution, its own backend, or a remote agent/runtime while still sharing the same Macaca session, EventLog, replay, control, trace, audit, policy, and capability infrastructure.

This design treats Macaca OS as a protocol platform. OS services define and enforce the protocol. Applications and providers supply behavior behind the protocol. Shells remain adapters.

## Goals

- Define `service.application_execution` as the serviceized capability family for application execution sessions.
- Define one protocol used by all application execution provider shapes.
- Implement all three provider strategies in this change: `macaca_hosted`, `external_app_backend`, and `remote_agent`.
- Persist all execution facts as durable session events before realtime delivery.
- Let frontends render replay/current state by subscribing to events and querying projections, not by owning execution state.
- Route every control command through typed service commands, policy, audit, and provider adapters.
- Support application-owned backend systems without letting them bypass Macaca EventLog, trace, audit, capability, or policy.
- Support remote agents as leased protocol participants rather than trusted host-internal code.
- Keep Macaca OS application-neutral, provider-neutral, model-neutral, driver-neutral, gateway-neutral, and workflow-neutral.

## Non-Goals

- No Codex-specific execution service.
- No Workbench-specific route or provider branch.
- No browser-owned long-running loop for production execution.
- No direct frontend persistence of authoritative execution events.
- No replacement of existing `service.agent_execution`; application execution may delegate agent work through that service but does not become the agent runtime itself.
- No raw prompt, secret, credential, package, WASM, provider payload, or unbounded output in observability surfaces.

## Architecture Overview

```text
Applications / App-Owned UI / API Clients
  | start_execution / subscribe / replay / control
  v
SDK / SystemFacade / Shell Adapters
  | typed commands only
  v
service.application_execution
  | trace + policy + resource + entitlement + audit decorators
  | provider strategy routing
  | durable session event append
  | current-state projection
  v
ApplicationExecutionProvider
  |-- macaca_hosted
  |     Runtime Host loads application execution component/adapter.
  |
  |-- external_app_backend
  |     Application backend executes, writes events through Macaca gateway.
  |
  |-- remote_agent
        Remote worker executes under lease, heartbeat, and control channel.

EventLog / Session Event Store
  -> replay API
  -> current-state projector
  -> realtime/SSE/WebSocket observer
  -> audit/diagnostics
```

The frontend never calls a provider directly. The frontend never owns the loop. The provider never writes authoritative UI state directly to the frontend. Every execution fact flows through Macaca event ingress and persistence.

## Protocol Surfaces

### 1. Execution Start Protocol

`StartApplicationExecutionCommand` creates or resumes the durable execution envelope.

Required fields:

- `application_id`
- `session_id`
- `run_id` or `idempotency_key` from which the service can derive one
- `task_input` as a bounded, sanitized command payload or payload reference
- `workspace_ref`
- `requested_capabilities`
- `provider_preference`
- `trace_context`
- `policy_context`
- `tenant_id` when available
- `actor`

Required outcomes:

- `accepted`
- `denied`
- `unavailable`
- `unsupported`
- `failed`

On `accepted`, the result includes:

- `session_id`
- `run_id`
- `provider_id`
- `provider_kind`
- `event_cursor`
- `control_ref`
- `workspace_ref`

The command MUST append `session.started` or `execution.accepted` before returning success. If provider assignment fails, it MUST append a structured failure event unless policy denied the request before event creation.

### 2. Session Event Protocol

`ApplicationExecutionEventEnvelope` is the only authoritative execution fact shape.

Required fields:

- `application_id`
- `session_id`
- `run_id`
- `seq`
- `timestamp`
- `event_type`
- `trace_id`
- `actor`
- `provider_id`
- `provider_kind`
- `visibility`
- `causality`
- `sanitized_payload`
- `payload_ref` when the payload is too large or sensitive for inline storage
- `schema_version`

Required event types for this change:

- `session.started`
- `execution.accepted`
- `provider.assigned`
- `provider.heartbeat`
- `provider.snapshot`
- `llm.requested`
- `llm.completed`
- `tool.call.requested`
- `tool.call.dispatched`
- `tool.call.completed`
- `approval.requested`
- `approval.resolved`
- `checkpoint.created`
- `control.requested`
- `control.delivered`
- `control.completed`
- `execution.completed`
- `execution.failed`
- `execution.cancelled`

Event append MUST be idempotent. Either Macaca assigns `seq`, or the provider supplies an idempotency key and Macaca assigns the final sequence. Provider-supplied sequence values may be accepted only after strict monotonic validation within a provider lease.

### 3. Execution Control Protocol

`ApplicationExecutionControlCommand` captures user or system control actions.

Supported commands:

- `cancel`
- `approve`
- `reject`
- `pause`
- `resume`
- `retry`
- `inject_input`

Every control command MUST include:

- `application_id`
- `session_id`
- `run_id`
- `control_id` or `idempotency_key`
- `command`
- `actor`
- `reason_code`
- `trace_context`
- `policy_context`
- bounded command payload or payload reference

The service MUST:

1. Validate session/run/provider binding.
2. Evaluate policy before side effects.
3. Append `control.requested`.
4. Route to the selected provider adapter.
5. Append `control.delivered` or structured delivery failure.
6. Append `control.completed` when the provider reports completion.

Duplicate controls with the same idempotency key MUST return the original outcome and MUST NOT redeliver side effects.

### 4. Provider Protocol

Every provider strategy implements one contract:

```text
describe()
start(command)
control(command)
health(scope)
snapshot(scope)
resume(checkpoint)
shutdown(scope)
```

Required provider descriptor fields:

- `provider_id`
- `provider_kind`
- `protocol_version`
- `supported_commands`
- `supported_events`
- `checkpoint_support`
- `heartbeat_policy`
- `control_delivery`
- `capability_declarations`
- `resource_profile`
- `transport_kind`
- `health_state`

Provider kinds:

- `macaca_hosted`
- `external_app_backend`
- `remote_agent`
- `unavailable`

Provider adapters MUST NOT branch on application id, workflow name, model name, provider name, driver name, or business domain. Provider selection is based on manifest-declared execution profile, capability availability, policy, tenant constraints, health, and explicit provider preference when allowed.

### 5. Gateway Ingress Protocol

External backends and remote agents cannot write directly to shell state. They must call Macaca gateway ingress.

Gateway commands:

- `append_event`
- `report_heartbeat`
- `report_snapshot`
- `request_approval`
- `report_completion`
- `report_failure`

Gateway ingress MUST validate:

- callback identity or signed token
- application/session/run binding
- provider lease
- command/schema version
- idempotency key
- event type allowlist
- payload size and sanitization
- trace context or trace continuation
- policy/resource/entitlement constraints

Invalid gateway calls MUST return structured denied, unsupported, stale-lease, invalid-schema, duplicate, or unavailable results. They MUST NOT partially append unsafe events.

### 6. Replay and Current-State Protocol

Current state is a projection derived from durable events.

Projection output includes:

- session lifecycle
- run lifecycle
- assigned provider
- latest heartbeat
- pending approvals
- active control commands
- latest checkpoint
- tool/LLM step summaries
- terminal result or failure
- replay cursor

Replay API MUST support:

- from beginning
- from cursor/seq
- bounded page size
- event type filters
- visibility filters
- deterministic ordering

Realtime subscriptions MUST publish events after persistence or publish persisted event references. They MUST NOT publish frontend-only events as authoritative execution state.

## Provider Strategy Details

### macaca_hosted

The Macaca-hosted provider runs inside backend-owned runtime infrastructure. It may load WASM application components, YAML adapters, GenUI/headless application adapters, or future hosted components through application ABI and runtime-host provider factories.

Implementation responsibilities:

- Resolve application manifest and execution profile.
- Create session/run/workspace envelope.
- Start backend-owned async execution task.
- Route application service calls through declared capabilities and ServiceRuntime.
- Append events at each lifecycle node.
- Respect control commands through cancellation tokens, approval wait handles, pause/resume state, and checkpoint restore.
- Persist checkpoints before waits or shutdown boundaries when supported.
- Return structured unavailable if the application runtime is absent or unsupported.

### external_app_backend

The external backend provider treats an application-owned backend as the execution owner while Macaca remains the protocol owner.

Implementation responsibilities:

- Validate manifest-declared backend endpoint, protocol version, callback identity, allowed scopes, timeout, heartbeat policy, and supported controls.
- Call backend start endpoint with session/run/workspace/callback envelope.
- Store provider assignment and callback lease.
- Receive callback events through gateway ingress.
- Forward control commands to backend control endpoint.
- Mark provider stale or failed on heartbeat timeout.
- Never trust backend-provided payloads without schema validation and sanitization.

### remote_agent

The remote agent provider treats remote workers as leased protocol participants.

Implementation responsibilities:

- Register remote agent descriptors and health state.
- Match execution requests to remote agents by capability, resource policy, tenant/region constraints, and health.
- Issue execution leases with expiry and scoped callback/control credentials.
- Dispatch start command over the selected transport.
- Receive heartbeat/event/snapshot/completion callbacks through gateway ingress.
- Deliver control commands over the registered control transport.
- Expire stale leases and append structured failure events.
- Support resume from checkpoint when both protocol and policy allow it.

## Shell and UI Boundary

Web, CLI, frontend, and app-owned UI bundles may only:

- call start execution
- subscribe to session events
- query replay/current state
- send control commands
- render approval, trace, diagnostics, and event-derived UI state

They MUST NOT:

- run production LLM/tool loops in browser JavaScript
- persist authoritative execution events from frontend state
- own provider assignment
- own approval/cancel semantics
- call external app backends directly for authoritative execution state
- bypass Macaca gateway ingress

Existing app UI bridge behavior that starts `/api/chat/v2` directly must be migrated or wrapped so it becomes a compatibility adapter over `service.application_execution` once the protocol service is available.

## Data Safety

EventLog, trace, audit, logs, snapshots, diagnostics, and realtime payloads MUST NOT contain:

- raw secrets
- provider credentials
- callback tokens
- private keys
- raw prompts
- raw provider payloads
- raw tool input/output when sensitive or unbounded
- raw manifests
- package bytes
- WASM bytes
- unbounded stdout/stderr

Large or sensitive content must be represented by bounded summaries, hashes, redacted fields, or `payload_ref` entries governed by storage policy.

## Error Model

Errors must be structured and stable:

- `unavailable`
- `disabled`
- `denied`
- `unsupported`
- `invalid_schema`
- `invalid_state`
- `duplicate`
- `stale_lease`
- `timeout`
- `provider_failed`
- `policy_denied`
- `resource_denied`
- `entitlement_denied`
- `sanitization_failed`

Every error includes:

- stable code
- layer
- operation
- application/session/run scope when available
- provider id/kind when available
- trace id
- sanitized reason
- retryability

## Migration Plan

1. Add protocol DTOs and tests without changing runtime behavior.
2. Add SDK/SystemFacade clients that return unavailable until the service exists.
3. Add `service.application_execution` unavailable provider and descriptor.
4. Add EventLog append/replay/current-state projection behind tests.
5. Implement `macaca_hosted`.
6. Implement `external_app_backend` gateway ingress and provider adapter.
7. Implement `remote_agent` registry, lease, gateway ingress, and provider adapter.
8. Migrate shell/app UI start-subscribe-control path to the new service.
9. Migrate `CODEX-WASM-WORKBENCH` validation to prove backend-owned execution and browser-close survival.
10. Add dependency-boundary and regression gates.

Rollback is additive: callers can keep existing app UI/chat paths until the new service is enabled. The service must return structured unavailable when disabled, so disabling the bootstrap returns the system to pre-change behavior without fake success.

## Testing Strategy

Test layers:

- DTO serde and schema-version tests.
- Provider descriptor/admission tests.
- Event append idempotency and sanitization tests.
- Current-state projection tests from synthetic event streams.
- Control-command idempotency and policy-denial tests.
- `macaca_hosted` unit/integration tests with fake hosted app component.
- `external_app_backend` tests using a local fake backend server or in-process transport adapter.
- `remote_agent` tests using a fake remote agent registry/transport.
- Web/API tests proving start/replay/control endpoints are shell adapters.
- Browser-close/reopen integration proof using `CODEX-WASM-WORKBENCH`.
- Dependency-boundary tests preventing shell-owned execution and app-specific OS branches.

## Risks and Mitigations

- Risk: The platform becomes too broad to finish.
  - Mitigation: Implement by protocol slice order. Each slice has DTO tests, service tests, provider tests, and a commit boundary.
- Risk: external backends bypass EventLog.
  - Mitigation: Do not expose authoritative frontend subscription endpoints for external backend state. Gateway ingress is mandatory.
- Risk: remote agents gain too much trust.
  - Mitigation: lease, scoped credentials, capability delegation, heartbeat expiry, and policy gates are required before side effects.
- Risk: app-specific logic leaks into OS.
  - Mitigation: dependency-boundary tests and text guards reject application id/workflow/model/provider hardcoding in generic service code.
- Risk: sensitive payloads enter observability.
  - Mitigation: sanitizer tests and payload-ref rules are mandatory before event append.
