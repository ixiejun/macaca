## ADDED Requirements

### Requirement: Approval Service
The system SHALL provide `service.approval` for approval request creation,
listing, resolution, expiry, cancellation, policy explanation, and audit.

#### Scenario: Privileged side effect approval
- **WHEN** a tool, process, file, Git, plugin, MCP, or remote environment call
  requires approval
- **THEN** the service SHALL persist a pending request before side effects
- **AND** shells SHALL only render and submit decisions, not own approval policy

#### Scenario: Approval audit replay
- **WHEN** an operator queries an approval decision
- **THEN** the service SHALL return sanitized action summary, reviewer class,
  decision, reason code, trace refs, and audit refs

### Requirement: Hook Lifecycle Service
The system SHALL provide `service.hook` for managed pre/post tool hooks, session
hooks, hook catalog, policy resolution, execution, and result audit.

#### Scenario: Pre-tool hook blocks execution
- **WHEN** a pre-tool hook blocks a side-effecting tool call
- **THEN** the hook service SHALL return a structured blocked result before
  provider dispatch
- **AND** the downstream service SHALL not execute the side effect

#### Scenario: Managed-only hook policy
- **WHEN** admin requirements allow managed hooks only
- **THEN** user, project, and session hooks SHALL be ignored while managed hooks
  remain active

### Requirement: Config, Requirements, Permission Profiles, and Feature Flags
The system SHALL provide `service.config` for layered config reads, value
writes, batch writes, schema reads, requirements reads, hot reload, permission
profiles, and feature flags.

#### Scenario: Hot reload config
- **WHEN** config changes are written with reload requested
- **THEN** the config service SHALL update the effective config for eligible
  loaded sessions and emit bounded change notifications

#### Scenario: Requirements constrain permissions
- **WHEN** requirements restrict approval policies, sandbox modes, network
  modes, permissions, or managed hooks
- **THEN** the service SHALL expose executable constraints to policy decorators
  before privileged calls run

### Requirement: LLM Model Catalog and Continuation Validation
The system SHALL harden `service.llm` with model catalog, provider capabilities,
route resolution, continuation validation, budget status, and degradation
explanation.

#### Scenario: Provider continuation protocol validation
- **WHEN** a provider requires model-specific continuation metadata after a tool
  result
- **THEN** `service.llm` SHALL validate that metadata before dispatch
- **AND** failures SHALL return structured provider-protocol diagnostics rather
  than causing opaque agent failures
