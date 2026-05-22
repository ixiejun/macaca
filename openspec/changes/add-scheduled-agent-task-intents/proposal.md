# Change: Add Scheduled Agent Task Intents

## Why

Macaca's serviceized Scheduler can manage generic jobs, and Agent Execution can
run app-scoped agents, but the system does not yet have a generic way to capture
long-running scheduled agent task intent. Users need two equivalent entry paths:
manual task creation from the application operations UI, and natural-language
delegation to an application's entry agent so the agent can create the scheduled
task on the user's behalf.

## What Changes

- Add a provider-neutral Scheduled Agent Task service contract that accepts
  traced task intent, target agent, schedule policy, bounded context, and user
  prompt material.
- Store prompt material as controlled payload/audit mementos owned by the new
  service; Scheduler jobs and run summaries carry only `AutonomyPayloadRef`,
  digest, safe summary, trace id, audit id, and bounded metadata.
- Activate runtime-host dispatch for `SchedulerTargetCommand::AgentExecution`
  by resolving payload refs and calling `service.agent_execution`.
- Add application-scoped Web/frontend manual task creation and inspection
  surfaces that remain shell adapters.
- Add a generic entry-agent tool that lets application entry agents create the
  same scheduled agent tasks through the service boundary.
- Add tests and boundary gates that prevent raw prompt leakage, Scheduler prompt
  ownership, shell-owned scheduling semantics, and application-specific logic.

## Impact

- Affected specs:
  - `scheduled-agent-task-service`
  - `scheduler-service`
  - `agent-execution-service`
  - `sdk-system-facade`
  - `web-cli-thin-shell-v0`
  - `serviceization-dependency-gate`
  - `serviceization-escape-hatches`
- Affected code:
  - `macaca/crates/foundation/macaca-proto`
  - new scheduled-agent-task service crate under `macaca/crates/services/`
  - `macaca/crates/runtime/macaca-runtime-host`
  - `macaca/crates/facade/macaca-sdk`
  - `macaca/crates/shells/macaca-web`
  - `frontend/components/autonomy`
  - `frontend/lib/autonomy*.ts`
  - Macaca integration and boundary tests

## Non-Goals

- Do not make Scheduler store raw prompts or interpret task requirements.
- Do not add application-specific scheduled-task templates in OS, Web, SDK, or
  frontend code.
- Do not replace manifest-declared heartbeat agent execution.
- Do not require a concrete LLM, model, driver, gateway, chain, or payment
  provider.
