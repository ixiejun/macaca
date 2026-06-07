# Design: Scheduled Agent Task Intents

## Context

Macaca is a 24/7 Agent OS. A scheduled task is often not merely a timer; it is a
durable user intent that wakes an agent at a later time, gives that agent the
task requirements, lets the agent call LLM/tools/services, and records a
complete audit chain. Users may create such tasks manually, but they may also
delegate the setup step to an application's entry agent using natural language.

The existing Scheduler service correctly owns schedule definitions, due-run
materialization, leases, retry, and run history. Agent Execution correctly owns
context-aware agent invocation. The missing piece is a generic intent service
between user/agent input and Scheduler jobs.

## Goals

- Provide one generic service boundary for scheduled agent task intent.
- Let manual UI creation and entry-agent tool creation produce the same typed
  command and audit shape.
- Keep raw prompts out of Scheduler jobs, run summaries, logs, snapshots, and
  frontend-safe list responses.
- Dispatch due Scheduler `AgentExecution` targets through Runtime Host and
  `service.agent_execution`.
- Preserve trace, audit, policy, resource, and structured unavailable behavior.

## Non-Goals

- No application-specific business logic, workflow names, app names, provider
  names, model names, driver names, gateway names, chain names, or payment names
  in OS-layer code.
- No frontend-owned scheduler semantics.
- No Scheduler-owned prompt parsing, prompt storage, task planning, or LLM calls.
- No replacement of heartbeat agent execution; heartbeat remains cadence-owned
  by `service.heartbeat`.

## Ownership

- **Scheduled Agent Task service:** Owns user intent admission, prompt payload
  mementos, payload digests, sanitized summaries, creation audit records, and
  translation from intent to Scheduler `AgentExecution` target.
- **Scheduler service:** Owns time, lifecycle, due-run materialization, leases,
  retries, and run summaries. It stores only target DTOs and payload refs.
- **Runtime Host:** Owns the dispatch Strategy that maps Scheduler
  `AgentExecution` targets to `service.agent_execution` calls.
- **Agent Execution service:** Owns context-aware agent invocation, LLM/tool
  execution, lifecycle events, and final execution result evidence.
- **Web/frontend:** Own only input adaptation and rendering of sanitized state.
- **Entry agent tool:** Owns only conversion from agent tool arguments into the
  same Scheduled Agent Task service command used by Web.

## Design Patterns

- **Command:** All cross-boundary work uses typed command/result DTOs:
  `CreateScheduledAgentTaskCommand`, Scheduler job commands, and
  `AgentExecutionCommand`.
- **Facade:** SDK focused clients hide service-runtime transport from Web,
  frontend adapters, application runtimes, and entry-agent tools.
- **Strategy:** Runtime Host dispatches Scheduler target kinds through
  replaceable target strategies.
- **Memento:** Prompt payload, task summary, Scheduler job/run, and execution
  result evidence are replayable snapshots with redaction guarantees.
- **Observer:** Trace, audit, event log, and future SSE consumers can subscribe
  to task lifecycle evidence.
- **Decorator:** Trace, policy, budget/resource, entitlement, redaction, and
  metering wrap service calls before side effects.
- **Builder:** DTO builders provide explicit optional metadata/context assembly
  without overloading constructors.
- **Specification:** Boundary gates enforce layer direction and redaction.

## Data Flow

```text
Manual UI OR Entry Agent Tool
  -> CreateScheduledAgentTaskCommand
  -> service.scheduled_agent_task
  -> payload memento + digest + audit id
  -> SchedulerRegisterJobCommand with SchedulerTargetCommand::AgentExecution
  -> service.scheduler job/run
  -> Runtime Host SchedulerLane lease
  -> Runtime Host AgentExecution dispatch Strategy
  -> payload ref resolution through service.scheduled_agent_task
  -> AgentExecutionCommand
  -> service.agent_execution
  -> Agent Context + LLM/tools/services
  -> execution result + trace/audit chain
```

## Payload And Audit Rules

Prompt text may enter only through the Scheduled Agent Task service create
command or an equivalent internal payload-resolution call. After admission, all
public and Scheduler-facing state uses `AutonomyPayloadRef` with a reference,
optional digest, redacted summary, and safe metadata.

Logs and snapshots must include safe identifiers and reason codes only:

- scheduled task id
- scheduler job id
- scheduler run id
- target agent
- trace id
- audit id
- payload digest
- lifecycle state
- structured failure code

They must not include raw prompts, raw delegated context, manifests, WASM bytes,
package bytes, secrets, credentials, private keys, raw signatures, raw provider
payloads, or unbounded output.

## API Shape

Manual Web routes are application scoped:

```text
POST   /api/apps/{app_id}/autonomy/scheduled-agent-tasks
GET    /api/apps/{app_id}/autonomy/scheduled-agent-tasks
GET    /api/apps/{app_id}/autonomy/scheduled-agent-tasks/{task_id}
DELETE /api/apps/{app_id}/autonomy/scheduled-agent-tasks/{task_id}
```

The entry-agent tool exposes the same service command shape through the existing
agent context/tool projection path. Its schema is generic:

- `target_agent`
- `task_prompt`
- `schedule`
- `metadata`
- optional bounded `delegated_context`

## Logging

Required sanitized log nodes:

- `scheduled agent task create requested`
- `scheduled agent task payload persisted`
- `scheduled agent task scheduler job registered`
- `scheduled agent task create completed`
- `entry agent requested scheduled task creation`
- `scheduler agent execution run leased`
- `scheduled agent dispatch payload resolved`
- `scheduled agent dispatch invoking agent execution service`
- `scheduled agent dispatch completed`
- `scheduled agent task result recorded`

## Risks And Mitigations

- **Risk:** Scheduler becomes a prompt/task service.
  **Mitigation:** Scheduler accepts only `AutonomyPayloadRef` and target DTOs;
  boundary tests reject raw prompt fields in Scheduler code.
- **Risk:** Web/frontend become semantic owners.
  **Mitigation:** Shells call focused SDK clients and render sanitized state
  only; escape-hatch tests reject legacy route use and hardcoded templates.
- **Risk:** Entry-agent tool bypasses policy.
  **Mitigation:** Tool calls the same service command with trace, application
  scope, target agent, capability, and policy checks.
- **Risk:** Prompt leakage through logs or summaries.
  **Mitigation:** Redaction tests serialize summaries/runs/responses and assert
  the raw fixture prompt is absent.
- **Risk:** Runtime dispatch couples to one provider.
  **Mitigation:** Runtime Host uses service ids, typed DTOs, and `ServiceRuntime`
  calls; unavailable services return structured skipped/retryable outcomes.

## Verification

- OpenSpec strict validation.
- DTO unit tests for validation and redaction.
- Local provider tests for payload memento, audit id, Scheduler target creation,
  and unavailable behavior.
- Runtime-host tests for `AgentExecution` dispatch.
- Web route tests for app-scoped command adaptation.
- Frontend lint/typecheck.
- Integration test for full create -> schedule -> lease -> agent execution ->
  result evidence chain.
- Boundary gates for serviceization dependency direction and prompt leakage.
