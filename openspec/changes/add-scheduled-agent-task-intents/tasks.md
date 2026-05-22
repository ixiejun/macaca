# Tasks: Add Scheduled Agent Task Intents

## 1. Service Contract

- [x] 1.1 Add provider-neutral Scheduled Agent Task DTOs in `macaca-proto`.
- [x] 1.2 Add typed create/get/list/cancel command names and result summaries.
- [x] 1.3 Add Builder-style helpers for optional delegated context and metadata.
- [x] 1.4 Add DTO tests for trace, target agent, schedule, prompt validation, and safe summary redaction.
- [x] 1.5 Export the new contract from `macaca-proto`.

## 2. Payload And Audit

- [x] 2.1 Add `macaca-scheduled-agent-task` service crate with descriptor, health, snapshot, local provider, and unavailable provider.
- [x] 2.2 Implement provider-local prompt payload mementos with digest and redacted summary.
- [x] 2.3 Implement sanitized audit correlation for task create, payload persistence, Scheduler registration, cancellation, and result recording.
- [x] 2.4 Translate admitted intent into `SchedulerTargetCommand::AgentExecution` with `AutonomyPayloadRef`.
- [x] 2.5 Add structured logs for create request, validation, payload persistence, scheduler registration, success, denial, unavailable, and failure.
- [x] 2.6 Add tests proving raw prompt does not appear in summaries, Scheduler jobs, run summaries, or audit-safe responses.

## 3. Runtime Dispatch

- [x] 3.1 Add Runtime Host dispatch Strategy for `SchedulerTargetCommand::AgentExecution`.
- [x] 3.2 Resolve `AutonomyPayloadRef` through the Scheduled Agent Task service before invoking agent execution.
- [x] 3.3 Build `AgentExecutionCommand` with target agent, user prompt, delegated context, execution intent, trace, policy, and safe metadata.
- [x] 3.4 Return structured succeeded, retryable, skipped, unavailable, and failed outcomes to Scheduler run-control.
- [x] 3.5 Add sanitized dispatch logs for payload resolution, agent execution call, completion, skip, retry, and failure.
- [x] 3.6 Add runtime-host tests proving due Scheduler runs invoke `service.agent_execution` and preserve trace/audit ids.

## 4. UI

- [x] 4.1 Add Web routes under `/api/apps/{app_id}/autonomy/scheduled-agent-tasks`.
- [x] 4.2 Add focused SDK client methods for Scheduled Agent Task commands.
- [x] 4.3 Add frontend API helpers and TypeScript DTOs for manual scheduled-agent-task creation and safe summaries.
- [x] 4.4 Add a manual task editor UI with task prompt, target agent, schedule, name, and metadata fields.
- [x] 4.5 Ensure frontend list/detail surfaces render only sanitized summary, digest, trace id, audit id, lifecycle, and result status.
- [x] 4.6 Add Web/frontend validation and unavailable/denied/error rendering without frontend-owned scheduling semantics.

## 5. Entry Agent Tool

- [x] 5.1 Locate the existing generic tool/capability projection boundary used by entry agents.
- [x] 5.2 Expose a generic `scheduled_agent_task.create` tool schema to eligible entry agents.
- [x] 5.3 Route tool invocations to the Scheduled Agent Task service using the same command shape as Web.
- [x] 5.4 Add policy/capability checks for application scope, trace, target agent, resource/budget, and service availability.
- [x] 5.5 Add sanitized tool invocation logs and structured denied/unavailable results.
- [x] 5.6 Add tests proving UI-created and entry-agent-created scheduled tasks produce equivalent service commands.

## 6. Tests And Boundary Gates

- [x] 6.1 Add integration test for create -> Scheduler job -> due run -> Runtime Host dispatch -> Agent Execution -> result/audit chain.
- [x] 6.2 Add dependency-boundary gates preventing Scheduler providers from storing or interpreting raw prompts.
- [x] 6.3 Add escape-hatch gates preventing Web/frontend scheduled-agent-task code from using legacy `/api/apps/{app_id}/schedules`.
- [x] 6.4 Add gates preventing application-specific task templates, workflow names, provider names, model names, driver names, gateway names, chain names, payment names, or business-domain branches in OS-layer scheduled task code.
- [x] 6.5 Add redaction tests for logs/snapshots/responses where practical, using a raw prompt fixture string.
- [x] 6.6 Run `cargo fmt`, targeted cargo tests, frontend lint/typecheck, `openspec validate add-scheduled-agent-task-intents --strict`, and `git diff --check`.
