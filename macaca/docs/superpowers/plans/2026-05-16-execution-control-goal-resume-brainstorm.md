# Execution Control Goal Resume Brainstorm

## Context

The current fullstack dev scenario verifies a generic execution-control requirement:

- The entry coordinator creates a goal through a declared tool barrier.
- The coordinator must pause after the goal is accepted.
- Planner, worker, review, and evaluator agents continue the goal lifecycle.
- The coordinator resumes only after an approved goal-lifecycle signal arrives.

This must remain a Macaca OS execution-control concern, not a chat-specific,
application-specific, or Web-owned workflow rule. The solution must preserve the
architecture constitution in:

- `macaca/docs/macaca-os-architecture-governance.md`
- `macaca/docs/macaca-os-microkernel-boundaries.md`
- `macaca/docs/macaca-os-serviceization-allowlist.md`

The existing implementation already has `service.execution_control`, policy
resolution, registration, a runtime middleware, and legacy Web resume adapters.
The weak point is that resume delivery is still split between deprecated
session channels and goal-to-session maps, so missing or mismatched mappings can
leave a coordinator waiting even though the goal completed.

## Constraints

- Do not add application-name, agent-name, workflow-name, provider-name, driver-name, model-name, or business-domain branches.
- Do not move planner, worker, reviewer, evaluator, or goal semantics into the kernel.
- Do not let Web, CLI, or frontend become execution-control semantic owners.
- Keep all cross-boundary behavior command-shaped, trace-required, policy-checked, auditable, and replayable.
- Keep raw prompts, raw manifests, provider payloads, unbounded output, secrets, and credentials out of logs, snapshots, and diagnostics.
- All implementation code should include detailed English comments and structured logs at key execution nodes.

## Option A: Patch The Legacy Session Resume Map

Extend the existing Web `goal_to_session` mapping and direct `resume_tx` send path
so it records more identifiers and retries when the first lookup misses.

Patterns:

- Adapter: Web adapts goal lifecycle events into the existing session channel.
- Observer: existing EventLog and SSE receive extra resume diagnostics.

Pros:

- Smallest immediate patch.
- Likely fixes the observed fullstack dev scenario quickly.

Cons:

- Keeps Web as the de facto owner of resume semantics.
- Continues bypassing `service.execution_control.request_resume`.
- Still relies on fragile in-memory session maps.
- Harder to audit because the service state machine does not see the accepted resume signal.

Risk:

- High architecture drift risk. This is acceptable only as a temporary mitigation, not the stable design.

## Option B: Execution-Control Resume Bridge In Web

Add a focused Web host adapter that listens to generic goal lifecycle and fork
lifecycle events, resolves the paused execution by service-owned correlation
metadata, calls `service.execution_control.request_resume`, persists the returned
events, and only then delivers the local `RuntimeResumeSignal` to the in-process
receiver.

Patterns:

- Adapter / Bridge: Web translates non-serializable local channels into provider-neutral service commands.
- Command: every resume delivery becomes `ExecutionControlResumeCommand`.
- Observer: service events are persisted before SSE/local delivery.
- State: service accepts or rejects duplicate, stale, cancelled, or timed-out resumes.
- Memento: execution snapshots explain paused, resumed, rejected, and timed-out states.

Pros:

- Keeps the current local receiver model while making the service state machine authoritative.
- Removes the most fragile part of the current behavior: direct resume delivery before service evidence.
- Provides a clean migration path from deprecated session-channel helpers to service-backed adapters.
- Can be implemented without redesigning task planning or goal lifecycle services.

Cons:

- Web still owns the in-process channel handoff until a serializable wait broker exists.
- Needs a small correlation registry so a goal/fork signal can find the paused execution id and session.

Risk:

- Medium. It is architecturally aligned if the bridge is explicitly adapter-only and all semantics live in service commands.

## Option C: Service-Owned Await/Notify Broker

Move wait registration and resume delivery fully into `service.execution_control`.
The middleware calls `await_resume`; goal/fork/approval/workflow event producers
call `request_resume`; the service stores a wait handle and wakes the paused
execution through a provider-owned broker.

Patterns:

- Mediator: execution-control service mediates between pause triggers and resume sources.
- Command: wait and resume are typed service commands.
- State: a single service-owned state machine owns all wait transitions.
- Observer: all lifecycle transitions emit trace/audit/EventLog evidence.
- Memento: snapshots include wait registrations, correlation keys, accepted sources, and terminal decisions.

Pros:

- Most complete and durable architecture.
- Removes direct session-channel ownership from Web and hook consumers.
- Makes timeout, duplicate resume, crash recovery, and diagnostics service-owned.
- Future remote/plugin execution-control providers become possible.

Cons:

- Requires changing the current runtime middleware contract from local receiver wait to service-backed wait.
- Needs careful async design to avoid holding service locks across waits.
- Broader test surface: local runtime, ServiceRuntime, Web adapter, goal lifecycle, and event replay.

Risk:

- Medium implementation risk, lowest long-term architecture risk.

## Option D: Task-Service Lifecycle Subscription

Expose a provider-neutral task/goal lifecycle event stream from `service.task`.
`service.execution_control` subscribes to declared resume sources and resumes
registered executions when matching task/goal events arrive.

Patterns:

- Observer: task lifecycle events are a service event stream.
- Strategy: execution-control policies select resume-source match strategies.
- Specification: correlation rules are executable and reject unsupported event shapes.

Pros:

- Cleanest separation of task semantics from execution-control state.
- Scales beyond goal completion to task failure, review completion, approval, and workflow barriers.
- Avoids Web owning lifecycle matching.

Cons:

- Requires task service event surface hardening if it is not already complete.
- Larger cross-service design than needed for the immediate bug.

Risk:

- Medium to high scope risk now, strong future direction after Option B or C.

## Recommended Approach

Use a two-step design: implement Option B first as the stable near-term fix, and
shape it so Option C can absorb it later without changing policy or public DTOs.

The recommended slice is:

- Add an `ExecutionControlResumeBridge` in the Web host layer.
- Keep it adapter-only: it may read local active sessions and deliver local `RuntimeResumeSignal`, but it must not decide business semantics.
- Register paused executions with correlation metadata such as `session_id`, `execution_id`, `task_id`, `goal_id` when available, `resume_source`, and `trace_id`.
- On `GoalCompleted`, `GoalFailed`, fork completion, or fork failure, build `ExecutionControlResumeCommand` using the generic resume source.
- Call `service.execution_control.request_resume` before touching `resume_tx`.
- Persist returned execution-control events through `runtime_event_bridge` before SSE or local channel delivery.
- Deliver the local resume signal only if the service accepted the resume transition.
- Query or snapshot `service.execution_control` when no paused execution correlation is found, then emit a bounded diagnostic event instead of silently continuing.

This keeps the immediate behavior reliable while honoring the constitution:

- Kernel stays limited to primitives and routing.
- Execution-control service owns state transitions and evidence.
- Task/goal services own lifecycle semantics.
- Web owns only host adaptation and rendering/subscription side effects.
- No application-specific or provider-specific branch is introduced.

## Concrete Design Sketch

### New Adapter

Create a small adapter in `macaca-web`, for example:

- `execution_control_resume_bridge.rs`
- `ExecutionControlResumeBridge`
- `PausedExecutionCorrelation`
- `ResumeSignalTranslator`

The bridge should receive:

- `AppState`
- `ServiceRuntime`
- `ExecutionControlScope`
- sanitized local resume signal metadata

The bridge should expose:

- `register_paused_execution(...)`
- `resume_from_goal_lifecycle(...)`
- `resume_from_fork_lifecycle(...)`
- `query_waiting_execution(...)`

### Service Calls

At the barrier:

- `request_pause`
- `record_checkpoint`
- `await_resume`
- local wait begins only after those events are persisted

At goal/fork completion:

- `request_resume`
- if accepted, send local `RuntimeResumeSignal`
- if rejected, log and persist `resume_rejected`

At timeout or shutdown:

- `cancel_wait`
- local receiver is closed or released only after service evidence exists

### Correlation Rules

The bridge should match on provider-neutral identity, in this order:

- exact `execution_id`
- exact `task_id` or `goal_id` stored in execution-control scope metadata
- exact `session_id` plus allowed resume source

If multiple candidates exist, the bridge must reject with an ambiguous diagnostic
instead of guessing. If no candidate exists, it must query/snapshot the service
and emit a bounded diagnostic that includes stable ids and reason code only.

## Tests To Drive The Fix

- Coordinator creates a goal, execution-control records pause, checkpoint, await, resume, and resume-delivered events.
- Coordinator does not continue immediately after `create_goal`.
- Worker/review/evaluator agents can complete the goal while the coordinator is paused.
- Goal completion resumes the exact paused execution by correlation id.
- Duplicate goal completion produces `resume_rejected` and does not send a second local resume signal.
- Missing correlation emits bounded diagnostics and does not crash or fake success.
- Absent `service.execution_control` returns structured unavailable behavior.
- Static gate rejects new direct `resume_tx.send` or `pause_signal.store` call sites outside approved adapters.

## Implementation Plan Candidate

1. Add an OpenSpec follow-up change such as `fix-execution-control-goal-resume-bridge`.
2. Add failing tests around the fullstack goal pause/resume lifecycle.
3. Add `ExecutionControlResumeBridge` with detailed English comments and structured tracing logs.
4. Move existing `loop_manager` and `hook_consumer` direct resume handoffs through the bridge.
5. Make `ExecutionControlMiddleware` call service commands for pause, checkpoint, await, and cancel instead of only toggling the local flag.
6. Persist all service-returned events through `runtime_event_bridge` before live SSE or local resume delivery.
7. Strengthen static gates so deprecated direct session-channel writes cannot grow.
8. Run targeted Web/runtime-host tests, OpenSpec validation, dependency-boundary gates, and a fullstack dev smoke test.

## Decision

Proceed with Option B now, with DTOs and command names aligned to Option C.
Option A is too fragile for the architecture goals. Option D is a strong later
service-to-service evolution once task lifecycle event contracts are ready.
