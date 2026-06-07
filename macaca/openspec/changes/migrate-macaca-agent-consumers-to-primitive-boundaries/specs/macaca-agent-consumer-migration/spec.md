## ADDED Requirements

### Requirement: Upper crates use additive macaca-agent primitives

Upper crates SHALL construct agent service bundles through additive `macaca-agent` primitive APIs rather than deprecated compatibility helpers.

#### Scenario: Kernel builds empty services

- **GIVEN** the kernel executes a registered agent
- **WHEN** it needs an empty service bundle
- **THEN** it constructs services with `AgentServices::builder().build()`
- **AND** the no-op memory, IPC, and persistence fallback behavior remains unchanged.

#### Scenario: SDK tests build empty services

- **GIVEN** SDK declarative-agent tests need a service bundle
- **WHEN** they run an agent with no injected services
- **THEN** they construct services with `AgentServices::builder().build()`
- **AND** existing test assertions remain unchanged.
