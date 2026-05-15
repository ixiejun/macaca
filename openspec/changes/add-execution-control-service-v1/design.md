# Design: Execution Control Service v1

## Context

Macaca already has multiple pause/resume patterns:

- Fork-join delegation pauses a coordinator until fork validation resumes it.
- Goal/task orchestration pauses a coordinator after goal creation until planner, worker, review, and evaluator complete the goal.
- Application ABI work defines application lifecycle pause/resume and checkpoint payloads.
- Runtime refactor notes identify session, trace, and resume as framework concerns rather than web glue.

The recent serviceized chat main-thread bug showed the boundary problem clearly: once entry-agent execution moved behind `service.agent_execution`, the old coordinator-only pause middleware was no longer attached. The immediate repair restored that behavior for `ChatMainThread`, but the stable design must make pause/resume an explicit optional execution capability.

## Goals

- Make pause/resume a general execution-control capability, not a special case for chat or goals.
- Let applications opt in through manifest/application metadata and let individual `AgentExecutionCommand`s override within policy.
- Preserve a single `service.agent_execution` start path while making execution control available to ordinary runtime executions.
- Make all pause/resume decisions traceable, auditable, replayable, and policy checked.
- Keep trigger semantics configurable through strategies rather than application, agent, workflow, provider, or driver name branches.
- Support an additive migration from built-in runtime capability to `service.execution_control`.

## Non-Goals

- Do not redesign planner, worker, reviewer, goal evaluator, workflow engine, or application lifecycle semantics.
- Do not add application-specific pause rules to `macaca-web`, kernel, SDK, runtime-host, or generic services.
- Do not require every application or execution to use pause/resume.
- Do not persist raw prompts, raw manifests, credentials, provider payloads, unbounded tool output, or unsanitized checkpoint payloads.
- Do not make Web, CLI, frontend, or gateway shells owners of execution-control semantics.

## Design Patterns

- **Command**: pause, resume, checkpoint, query state, and policy merge operations are typed commands with typed results.
- **Strategy**: trigger rules and resume-source selection are replaceable strategies chosen by app policy and command overrides.
- **State**: execution control uses explicit states such as `Running`, `PauseRequested`, `Paused`, `ResumeRequested`, `Resuming`, `Completed`, `Cancelled`, `TimedOut`, and `Failed`.
- **Observer**: trace, audit, EventLog, RunTrace, SSE, and service events observe execution-control events instead of owning semantics.
- **Memento**: checkpoints are bounded snapshot references that allow replay and diagnostics without exposing internal channels or runtime objects.
- **Decorator**: trace, policy, resource, entitlement, metering, and sanitization checks wrap execution-control calls before side effects.
- **Facade**: SDK and shell consumers call focused clients or `SystemFacade`; they do not construct providers or manipulate runtime internals.
- **Adapter / Bridge**: legacy YAML, WASM, workflow, planner/worker, and chat flows adapt their existing signals into provider-neutral execution-control commands.

## Architecture

```text
Application manifest default policy
  + AgentExecutionCommand override
  -> ExecutionControlPolicyResolver
  -> service.agent_execution
  -> Runtime execution-control capability
  -> PauseResume middleware / runtime state adapter
  -> ExecutionControlEvent stream
  -> EventLog / RunTrace / Audit / service diagnostics
```

Stage 1 keeps the implementation inside runtime-host as a built-in capability. It introduces provider-neutral DTOs first, then lets `service.agent_execution` install execution-control adapters whenever the resolved policy enables them.

Stage 2 registers `service.execution_control` and routes pause/resume/checkpoint/state operations through `ServiceRuntime`. The stage-1 runtime capability becomes the first built-in provider for that service. Consumers move from direct runtime capability calls to service calls without changing policy shape.

## Policy Model

Execution-control policy is resolved from two sources:

1. Application default policy declared by manifest/application metadata.
2. Per-run override carried by `AgentExecutionCommand`.

The resolver must be deterministic:

- If neither source enables execution control, no pause/resume adapter is installed.
- If the app declares disabled and does not allow overrides, command override is denied.
- If the app declares enabled, command override may narrow triggers, resume sources, timeout, checkpoint mode, or audit verbosity.
- If the app allows dynamic opt-in, command override may enable execution control for that run.
- Unsupported trigger or resume-source names return structured `unsupported`, not silent fallback.

## Trigger And Resume Strategies

The contract should model trigger and resume sources generically:

- Tool-call barrier, such as after a specific declared tool succeeds.
- Goal lifecycle signal, such as goal completed, failed, cancelled, or timed out.
- Fork/delegation signal, such as fork validated or fork failed.
- Approval signal, such as approval granted or denied.
- Workflow barrier, such as step completed or external event arrived.
- Application lifecycle signal, such as app pause, resume, shutdown, or upgrade.
- Plugin/service event signal, declared by capability id and event kind.

Each strategy must produce a reason code, trace id, optional task/goal/fork id, and bounded diagnostic metadata.

## Service Contract

Stage 2 exposes `service.execution_control` with commands equivalent to:

- `resolve_policy`
- `register_execution`
- `request_pause`
- `record_checkpoint`
- `await_resume`
- `request_resume`
- `cancel_wait`
- `query_state`
- `snapshot`

Every command must carry application id, session id, execution id, trace context, source, and policy context. Commands that can cause side effects must pass policy before state changes.

## Layer Ownership

- Kernel: identity primitives, trace identity, session primitive contract, service-call routing, policy facade.
- Runtime host: built-in provider, service provider registration, decorators, unavailable behavior, snapshots.
- System services: execution control, agent execution, task planning/review, drivers, skills, MCP, LLM, context, memory.
- Application framework: manifest declarations, app-scoped capability defaults, application lifecycle adapter, app session envelope.
- Applications: product policy choices within declared capabilities.
- Shells: command submission, rendering, approval UI, diagnostics, event subscription.

## Trace, Audit, And Logs

Execution control must emit sanitized structured evidence at key nodes:

- policy resolved,
- execution registered,
- pause requested,
- pause accepted or rejected,
- pause entered,
- checkpoint recorded,
- resume source subscribed,
- resume requested,
- resume accepted or rejected,
- resume delivered,
- timeout or cancellation,
- state queried,
- snapshot produced.

Logs and audit records must include stable ids, trace id, service id, command name, execution id, state transition, reason code, and bounded metadata. They must not include raw prompts, raw manifest bodies, credentials, package bytes, WASM bytes, private keys, raw provider payloads, or unbounded tool output.

## Migration Plan

1. Define OpenSpec contract and provider-neutral DTO names.
2. Add protocol DTOs for policies, triggers, resume sources, commands, results, state, events, and snapshots.
3. Add stage-1 policy resolver and runtime capability.
4. Migrate `service.agent_execution` to resolve and install execution control for any eligible run.
5. Replace chat-main-thread-specific pause wiring with policy-driven execution control.
6. Add manifest/application metadata support for default policy and command override support for per-run policy.
7. Add trace/audit/EventLog/RunTrace evidence tests.
8. Add `service.execution_control` provider using the same DTOs and capability implementation.
9. Migrate stage-1 direct capability calls to `ServiceRuntime.call(service.execution_control, ...)`.
10. Add unavailable provider behavior and dependency-boundary gates.
11. Mark path-specific pause/resume helpers deprecated and block new direct call sites.

## Risks And Mitigations

- Risk: execution control becomes another workflow engine.
  - Mitigation: it owns state transitions, pause/resume commands, and evidence only; planners, workflows, and apps own domain semantics.
- Risk: policy config becomes too flexible and hard to reason about.
  - Mitigation: use typed trigger/resume enums plus declared extension ids, reject unknowns, and require deterministic merge tests.
- Risk: service calls add latency to hot loop paths.
  - Mitigation: stage 1 uses in-process capability; stage 2 keeps local built-in provider and measures overhead with targeted tests.
- Risk: checkpoints leak sensitive execution context.
  - Mitigation: checkpoints are references plus bounded sanitized metadata; raw payload persistence requires explicit safe store contracts.
- Risk: old web glue survives.
  - Mitigation: add static tests that reject new `PauseOnGoal` / direct session-channel wiring outside approved adapters.

## Open Questions

- Should execution-control timeout defaults be application-declared only, or should OS provide a conservative global default?
- Should approval UI resume signals be represented as execution-control resume sources in this change, or left as a later adapter?
- Which persistent store should own durable checkpoint references for stage 2 snapshots?
