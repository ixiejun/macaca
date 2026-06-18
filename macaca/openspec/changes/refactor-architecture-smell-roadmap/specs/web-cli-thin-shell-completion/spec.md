## ADDED Requirements

### Requirement: Web SHALL delegate task decomposition semantics to service-owned commands

Macaca Web SHALL NOT own task decomposition, task classification, planner role selection, worker role selection, review policy, retry policy, or task dependency semantics. Web SHALL adapt user input into provider-neutral SDK/facade/service commands and render the returned task/decomposition DTOs.

#### Scenario: Web requests task decomposition through facade
- **WHEN** a Web route or session loop needs to decompose a user goal into tasks
- **THEN** it SHALL call a focused SDK client, `SystemFacade`, or provider-neutral service command
- **AND** the command SHALL carry trace context and available app/session/task scope
- **AND** Web SHALL NOT classify the goal by hardcoded keyword chains, role names, workflow names, or application names

#### Scenario: Decomposition service is unavailable
- **WHEN** the decomposition service or focused client is unavailable
- **THEN** Web SHALL return or render a structured unavailable state
- **AND** Web SHALL NOT fall back to shell-owned task planning semantics
- **AND** the rejection SHALL be logged with sanitized operation, scope, trace id when available, and reason code

### Requirement: Web and CLI SHALL fail semantic ownership scans

Macaca SHALL provide an executable gate that rejects shell-owned system semantics in Web and CLI, including task planning, task decomposition, review orchestration, worker-loop execution, direct provider reads, and direct runtime ownership.

#### Scenario: Shell contains task keyword planning
- **WHEN** the shell semantic ownership gate scans production Web or CLI Rust source
- **THEN** keyword-driven task/planning/decomposition semantics SHALL fail the gate
- **AND** the diagnostic SHALL name the file, line, token, and required SDK/facade/service replacement

#### Scenario: Shell only delegates
- **WHEN** Web or CLI code only maps input/output and delegates through SDK/facade/service clients
- **THEN** the shell semantic ownership gate SHALL pass
- **AND** the diagnostic output SHALL remain deterministic and audit-friendly
