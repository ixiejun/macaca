## ADDED Requirements

### Requirement: Macaca SHALL expose provider-neutral LLM service commands

Macaca SHALL expose an LLM Service contract with typed commands for chat, model selection, and service snapshot. The commands SHALL include explicit application, session, agent, trace, message, model hint, options, budget, and policy context while remaining independent from concrete provider URLs, API keys, and provider-specific configuration.

#### Scenario: Chat command is accepted through the LLM service

- **WHEN** a caller submits an LLM chat command with application, session, agent, trace, messages, and options
- **THEN** the LLM Service SHALL validate the scope and trace context before dispatch
- **AND** it SHALL dispatch through a replaceable provider or router strategy
- **AND** it SHALL NOT require the caller to construct a concrete LLM provider

#### Scenario: Model selection command is provider-neutral

- **WHEN** a caller submits a model selection command with model hints and policy context
- **THEN** the LLM Service SHALL return a model selection result with provider-neutral routing metadata
- **AND** it SHALL NOT expose API keys, provider URLs, or hardcoded model-provider branches in the command contract

### Requirement: Macaca SHALL emit audit-friendly LLM service events and logs

Macaca SHALL emit structured logs and events for LLM model selection, chat dispatch, completion, failure, and snapshot emission.

#### Scenario: LLM chat completes

- **WHEN** an LLM chat request completes successfully
- **THEN** the LLM Service SHALL emit a `llm.chat.completed` event
- **AND** the event SHALL include service id, operation, application/session/agent scope, trace id, status, timing, model routing summary, and token/cost metadata when available
- **AND** the event SHALL NOT dump sensitive prompt or full message content by default

#### Scenario: LLM chat fails

- **WHEN** an LLM chat request fails
- **THEN** the LLM Service SHALL emit a `llm.chat.failed` event
- **AND** the event SHALL include a structured error class and sanitized diagnostic summary

### Requirement: Macaca SHALL expose deterministic LLM service snapshots

Macaca SHALL expose LLM service snapshots that describe service availability, provider inventory, model inventory, routing capability, health, and last audit ids without leaking sensitive request content.

#### Scenario: Snapshot is requested

- **WHEN** a caller requests an LLM service snapshot
- **THEN** the LLM Service SHALL return deterministic provider-neutral snapshot data
- **AND** unavailable providers SHALL be represented as structured unavailable states instead of panics or missing fields

### Requirement: Macaca SHALL keep deprecated LLM compatibility wrappers searchable

Macaca SHALL keep superseded LLM provider adapters and kernel compatibility entry points present as deprecated wrappers until all consumers are migrated.

#### Scenario: Old framework adapter remains during migration

- **WHEN** old framework code still references a provider-backed LLM adapter
- **THEN** the adapter SHALL remain searchable and marked deprecated
- **AND** new production paths SHALL prefer the service-backed LLM client or adapter
