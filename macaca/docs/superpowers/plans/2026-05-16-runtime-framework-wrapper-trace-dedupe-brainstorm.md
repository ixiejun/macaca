# Runtime Framework Wrapper Trace Dedupe Brainstorm

## Context

The observed duplicate-looking trace events are not duplicate EventLog rows.
They are two observation chains persisting the same logical framework tool
operation at two layers:

- The agent-execution semantic chain emits `AgentExecutionEvent::ToolCall` and
  `AgentExecutionEvent::ToolResult`, persisted as `delegated_tool_call` and
  `delegated_tool_result`.
- The tool-command trace chain emits `TraceEvent { type: tool_call/tool_result }`
  without a concrete `driver_id`, which Web maps to
  `AgentExecutionEvent::DriverTrace` and persists as `delegated_driver_trace`
  with `driver_name = macaca-framework`.

The existing Executor and Coordinator trace routes already identify these
framework wrapper traces and skip them. The service-backed Runtime route does
not apply the same admission rule, so the new `service.agent_execution` path
stores both semantic tool events and framework wrapper driver traces.

## Constraints

- Preserve the Macaca OS constitution:
  - `macaca/docs/macaca-os-architecture-governance.md`
  - `macaca/docs/macaca-os-microkernel-boundaries.md`
  - `macaca/docs/macaca-os-serviceization-allowlist.md`
- Do not add application-specific, agent-specific, workflow-specific,
  provider-specific, driver-specific, or business-domain branches.
- Do not make frontend rendering the semantic source of truth.
- Keep valuable provider/driver traces, especially events with concrete
  `driver_id` or richer diagnostic payloads.
- Keep durable EventLog and live SSE behavior aligned.
- Preserve traceability and auditability with bounded sanitized diagnostics.

## Option A: Hide Framework Wrapper Traces In The Frontend

Filter `delegated_driver_trace` events in the React renderer when
`driver_name = macaca-framework` and `type = tool_call/tool_result`.

Pros:

- Small visual fix.
- No backend behavior change.

Cons:

- EventLog still contains redundant rows.
- Replay, diagnostics, and downstream consumers still see duplicated logical
  operations.
- Frontend becomes responsible for deciding trace semantics.

Risk:

- High architecture risk. This treats a routing/admission problem as a display
  problem.

## Option B: Apply The Existing Framework Wrapper Filter To Runtime Route

Extract the current `framework_tool_wrapper` predicate into a small helper and
use it consistently for Executor, Coordinator, and Runtime trace routes.

Patterns:

- Specification: a pure predicate defines which trace events are framework
  wrappers already represented by semantic tool events.
- Adapter: route-specific adapters apply the same predicate before forwarding.
- Observer: only non-wrapper driver traces are observed as driver trace events.

Pros:

- Minimal, targeted fix.
- Aligns service-backed Runtime behavior with existing Executor behavior.
- Keeps durable EventLog and live UI naturally consistent.
- Does not require provider-name or app-name branches.

Cons:

- The predicate still lives in Web/framework adapter code, not in a shared trace
  contract.
- It solves the current wrapper duplication but does not create a general trace
  normalization layer.

Risk:

- Low. This is the right near-term fix if implemented as a reusable predicate
  with explicit tests.

## Option C: Add A Trace Classification Field

Extend `TraceEvent` with a provider-neutral classification, for example:

- `origin = framework_wrapper | concrete_driver | provider_diagnostic`
- `semantic_duplicate_of = tool_event`
- `persistence_policy = semantic_only | driver_trace | both`

The trace router uses this metadata rather than inferring wrapper status from
`driver_id == None` and `type`.

Patterns:

- Command/Event Envelope: trace events carry explicit routing metadata.
- Specification: persistence policy becomes executable and testable.
- Strategy: different hosts can choose policies without changing event shape.

Pros:

- Most explicit long-term model.
- Avoids inference based on missing `driver_id`.
- Scales to future tool/service/provider trace sources.

Cons:

- Requires cross-crate DTO changes and compatibility migration.
- More work than needed for the immediate fullstack duplicate symptom.

Risk:

- Medium. Good follow-up, but not necessary for the first fix.

## Option D: EventLog Idempotency Key

Add a logical idempotency key to event persistence so rows with the same
operation id, event kind, agent, task, and session can be deduplicated.

Patterns:

- Memento / Audit: durable rows include stable replay keys.
- Specification: persistence identity is explicit.

Pros:

- Protects against true duplicate writes from retrying collectors.
- Useful for future at-least-once event delivery.

Cons:

- Does not solve this case cleanly because the two rows have different event
  types and represent different layers.
- Can accidentally erase valuable multi-layer diagnostics if the key is too
  broad.

Risk:

- Medium. Valuable for durable delivery later, but too blunt for this wrapper
  trace issue.

## Recommended Approach

Implement Option B now, and leave Option C as the architectural follow-up.

The near-term fix should:

- Extract `is_framework_tool_wrapper_trace(trace: &TraceEvent) -> bool`.
- Define the predicate as `driver_id.is_none()` and
  `event_type in ["tool_call", "tool_result"]`.
- Apply the predicate before forwarding Runtime route traces into
  `AgentExecutionEvent::DriverTrace`.
- Keep concrete driver/provider traces untouched.
- Add structured logs at debug level when a wrapper trace is suppressed, with
  bounded fields: route, event_type, tool_name, correlation_id.
- Add tests proving Runtime route suppresses framework wrapper tool call/result
  traces while preserving concrete driver traces.
- Add a static regression test proving all `DriverTraceRoute` branches use the
  shared predicate.

This is not application-specific logic. It is a provider-neutral trace
classification rule: a framework wrapper trace with no concrete driver identity
and only the same tool call/result information is already represented by the
semantic agent-execution event.

## Concrete Implementation Sketch

### Backend Helper

Place the helper near `DriverTraceRoute` routing code in `framework_runner.rs`
or a focused trace-routing module:

- `fn is_framework_tool_wrapper_trace(trace: &macaca_tools::TraceEvent) -> bool`
- `fn should_forward_driver_trace(trace: &macaca_tools::TraceEvent) -> bool`

`should_forward_driver_trace` can be implemented as:

- return `false` for framework wrapper tool call/result
- return `true` otherwise

The helper must include English comments explaining:

- semantic tool events already cover wrapper tool call/result lifecycle;
- concrete driver traces remain valuable and must be forwarded;
- the rule avoids EventLog/UI replay duplication without hiding real provider
  diagnostics.

### Runtime Route

In `DriverTraceRoute::Runtime`, before sending
`AgentExecutionEvent::DriverTrace`, apply the helper:

- if suppressed, log a bounded debug event and `continue`;
- otherwise forward as before.

The same helper should be used by Executor and Coordinator branches to prevent
drift.

### Test Cases

Add focused tests around the pure predicate and route behavior:

- no `driver_id`, `tool_call` => suppressed
- no `driver_id`, `tool_result` => suppressed
- no `driver_id`, `thinking` => forwarded
- concrete `driver_id`, `tool_call` => forwarded
- concrete `driver_id`, `tool_result` => forwarded
- Runtime route does not emit `DriverTrace` for suppressed wrapper traces

If route-level async testing is too heavy, start with pure predicate tests plus
static tests that assert Runtime branch calls the helper.

## Follow-Up Architecture

After the immediate fix, consider Option C as a trace-contract cleanup:

- Add explicit trace origin/classification metadata to `TraceEvent`.
- Move wrapper classification closer to the trace producer.
- Let routing policy use explicit metadata rather than inference.
- Preserve backward compatibility by continuing to infer classification for old
  events that omit the new field.

## Decision

Proceed with Option B for the current bug. It is small, auditable, aligns
Runtime with Executor semantics, preserves valuable concrete driver traces, and
keeps the fix below the application layer without hardcoded business logic.
