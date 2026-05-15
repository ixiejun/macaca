## MODIFIED Requirements
### Requirement: Macaca SHALL expose provider-neutral Context service commands
Macaca SHALL expose a Context Service contract with typed commands for context assembly, active recall orchestration, provider inventory, engine inventory, and service snapshot. Commands SHALL include explicit application, session, agent, trace, budget, provider chain, policy, and context assembly intent. SDK clients that transport those typed commands over the generic System Service boundary SHALL preserve the same trace context on the outer service-call envelope before runtime dispatch.

#### Scenario: Context assembly is requested
- **GIVEN** a runtime-backed SDK Context client
- **WHEN** a caller submits a context assembly command with application, session, agent, trace, budget, and assembly intent
- **THEN** the SDK client SHALL attach that trace to the outer service-call command
- **AND** the Context Service SHALL compose model-ready context through replaceable context providers and engine strategies
- **AND** the Context Service SHALL return assembled messages, options, and a sanitized context report

#### Scenario: Context provider inventory is requested
- **GIVEN** a runtime-backed SDK Context client
- **WHEN** a caller requests provider inventory with trace and scope
- **THEN** the SDK client SHALL attach that trace to the outer service-call command
- **AND** the Context Service SHALL return deterministic inventory metadata
