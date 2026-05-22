## ADDED Requirements

### Requirement: SDK SHALL expose Scheduled Agent Task focused clients

The SDK SHALL expose provider-neutral Scheduled Agent Task focused client
methods so Web, CLI, application runtimes, and entry-agent tools can create and
inspect recurring agent work without constructing concrete providers or using
legacy Scheduler routes.

#### Scenario: Web creates a scheduled agent task through SDK

- **WHEN** Macaca Web receives a manual scheduled-agent-task create request
- **THEN** it SHALL construct a traced `CreateScheduledAgentTaskCommand`
- **AND** it SHALL call the focused Scheduled Agent Task client
- **AND** it SHALL NOT construct Scheduler providers, Agent Execution providers, payload stores, or concrete runtime-host providers.

#### Scenario: Entry agent tool creates a scheduled agent task through SDK or service runtime

- **WHEN** an entry agent invokes the scheduled-agent-task creation tool
- **THEN** the tool boundary SHALL submit the same provider-neutral command shape used by Web
- **AND** it SHALL preserve structured unavailable, denied, validation, provider-failure, and success results.

#### Scenario: Scheduled Agent Task service is unavailable

- **WHEN** the focused client receives a create, get, list, cancel, or payload-resolution command but the service is unavailable
- **THEN** it SHALL return a structured unavailable result
- **AND** it SHALL log operation, trace id, app scope, scheduled task id when available, and safe error code
- **AND** it SHALL NOT fake a successful mutation.
