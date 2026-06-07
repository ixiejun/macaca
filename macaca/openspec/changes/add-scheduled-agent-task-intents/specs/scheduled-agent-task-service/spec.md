## ADDED Requirements

### Requirement: Scheduled Agent Task service SHALL own scheduled agent task intent

Macaca SHALL provide a provider-neutral Scheduled Agent Task service that accepts
traced user or entry-agent intent for recurring agent work and converts that
intent into Scheduler jobs without making Scheduler own raw prompt material.

#### Scenario: User manually creates a scheduled agent task

- **GIVEN** a user submits an application-scoped scheduled agent task with schedule, target agent, and task prompt
- **WHEN** Web adapts the request into a `CreateScheduledAgentTaskCommand`
- **THEN** the Scheduled Agent Task service SHALL validate trace, scope, schedule, target agent, policy, and prompt bounds
- **AND** it SHALL persist prompt material as a controlled payload memento
- **AND** it SHALL register a Scheduler job whose target is `SchedulerTargetCommand::AgentExecution`
- **AND** the Scheduler job SHALL carry only an `AutonomyPayloadRef`, safe metadata, trace id, and audit correlation.

#### Scenario: Entry agent creates a scheduled agent task

- **GIVEN** a user asks an application entry agent to create recurring agent work
- **WHEN** the entry agent invokes the generic scheduled-agent-task creation tool
- **THEN** the tool SHALL submit the same provider-neutral create command as Web
- **AND** the service SHALL produce the same command result shape, audit evidence, Scheduler target kind, and redaction guarantees.

#### Scenario: Service provider is unavailable

- **WHEN** a scheduled-agent-task create, get, list, cancel, or payload-resolution command reaches an unavailable provider
- **THEN** the service SHALL return structured unavailable evidence
- **AND** it SHALL NOT panic, hang, silently succeed, create a Scheduler job, or fake an execution result.

### Requirement: Scheduled Agent Task payloads SHALL be redacted outside the owning service

The Scheduled Agent Task service SHALL own raw task prompt payload material and
SHALL expose only bounded references, digests, redacted summaries, and safe
metadata to Scheduler, Web, frontend, logs, snapshots, and audit-safe summaries.

#### Scenario: Scheduler stores a scheduled agent task target

- **WHEN** the Scheduled Agent Task service registers a Scheduler job
- **THEN** the job target SHALL include `AutonomyPayloadRef`
- **AND** it SHALL NOT include raw task prompt text, raw delegated context, raw provider payloads, secrets, manifests, WASM bytes, package bytes, credentials, private keys, raw signatures, or unbounded output.

#### Scenario: Operator lists scheduled agent tasks

- **WHEN** Web, CLI, SDK, or frontend lists scheduled agent task summaries
- **THEN** each summary SHALL include task id, target agent, schedule summary, lifecycle state, payload digest, redacted summary, trace id, audit id when available, and safe timestamps
- **AND** it SHALL NOT include raw task prompt text or raw delegated context.

### Requirement: Scheduled Agent Task service SHALL provide replayable audit correlation

The Scheduled Agent Task service SHALL emit replayable audit correlation across
intent admission, payload persistence, Scheduler job registration, due-run
dispatch, agent execution, and final result recording.

#### Scenario: Scheduled task creation succeeds

- **WHEN** a scheduled agent task is created
- **THEN** the result SHALL include trace id, task id, Scheduler job id when available, payload digest, audit id when available, and safe metadata
- **AND** the service SHALL emit sanitized logs for create request, validation, payload persistence, Scheduler registration, success, rejection, and failure.

#### Scenario: Scheduled task execution completes

- **WHEN** a due Scheduler run dispatches to Agent Execution and completes
- **THEN** the replayable evidence chain SHALL correlate scheduled task id, Scheduler job id, Scheduler run id, payload digest, Agent Execution trace id, execution result status, and audit id when available
- **AND** it SHALL remain inspectable without exposing raw prompts or raw provider output.
