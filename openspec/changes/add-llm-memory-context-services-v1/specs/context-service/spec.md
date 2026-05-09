## ADDED Requirements

### Requirement: Macaca SHALL expose provider-neutral Context service commands

Macaca SHALL expose a Context Service contract with typed commands for context assembly, active recall orchestration, provider inventory, engine inventory, and service snapshot. Commands SHALL include explicit application, session, agent, trace, budget, provider chain, policy, and context assembly intent.

#### Scenario: Context assembly is requested

- **WHEN** a caller submits a context assembly command with application, session, agent, trace, budget, and assembly intent
- **THEN** the Context Service SHALL compose model-ready context through replaceable context providers and engine strategies
- **AND** it SHALL return assembled messages or compiled context metadata with a context report
- **AND** it SHALL NOT require Web, CLI, or framework code to construct concrete context engines

#### Scenario: Context provider inventory is requested

- **WHEN** a caller requests provider or engine inventory
- **THEN** the Context Service SHALL return deterministic inventory metadata
- **AND** unavailable providers or engines SHALL be represented as structured unavailable states

### Requirement: Macaca SHALL orchestrate active recall through Memory Service boundaries

Macaca SHALL let Context Service orchestrate active recall policy and diagnostics while delegating memory storage and retrieval to Memory Service through a service client bridge.

#### Scenario: Active recall needs long-term memory

- **WHEN** context assembly decides active recall is needed
- **THEN** the Context Service SHALL call Memory Service through a scoped memory recall or prefetch command
- **AND** it SHALL include application, session, agent, trace, budget, and policy context
- **AND** it SHALL NOT bind directly to a concrete memory backend

#### Scenario: Active recall is unavailable

- **WHEN** Memory Service is not configured or policy denies recall
- **THEN** the Context Service SHALL return active recall diagnostics with structured unavailable or denied status
- **AND** context assembly SHALL continue with available providers when policy permits fallback

### Requirement: Macaca SHALL keep LLM calls outside Context Service ownership

Macaca SHALL keep model invocation ownership in LLM Service. Context Service MAY prepare prompts, summaries, reports, or digest inputs, but any model call SHALL be represented as an explicit service call strategy rather than hidden provider invocation.

#### Scenario: Context assembly prepares model-ready messages

- **WHEN** Context Service finishes assembly
- **THEN** it SHALL return model-ready messages or compiled context metadata
- **AND** LLM Service SHALL remain responsible for chat dispatch and model selection

### Requirement: Macaca SHALL emit audit-friendly Context service events and snapshots

Macaca SHALL emit structured logs, events, and deterministic snapshots for context assembly, provider chain execution, active recall, budget decisions, knowledge digest composition, inventory, and report generation.

#### Scenario: Context assembly completes

- **WHEN** context assembly completes
- **THEN** the Context Service SHALL emit a structured event with operation, scope, trace id, provider chain summary, active recall diagnostics, budget summary, report id, and sanitized status
- **AND** the event SHALL NOT dump sensitive prompt, memory content, or full provider payload by default

#### Scenario: Context snapshot is requested

- **WHEN** a caller requests a Context Service snapshot
- **THEN** the service SHALL return deterministic health, provider inventory, engine inventory, active recall capability, digest capability, policy status, and last audit ids

### Requirement: Macaca SHALL keep deprecated Context compatibility wrappers searchable

Macaca SHALL keep superseded context facade, engine, and report construction paths present as deprecated wrappers until all consumers are migrated to Context Service.

#### Scenario: Old context reporting path remains during migration

- **WHEN** old Web or framework code still references direct context report construction
- **THEN** the path SHALL remain searchable and marked deprecated
- **AND** new production paths SHALL prefer Context Service clients and adapters
