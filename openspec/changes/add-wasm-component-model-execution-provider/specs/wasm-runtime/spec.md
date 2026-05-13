## ADDED Requirements

### Requirement: Component Model Provider Execution
The system SHALL provide a runtime-host-owned WASM Component Model execution
provider that implements the existing provider-neutral
`WasmApplicationRuntimeProvider` contract without exposing concrete engine types
to public ABI, SDK, application framework, kernel, CLI, Web, or Gateway layers.

#### Scenario: Component export invocation succeeds
- **WHEN** an admitted WASM Component artifact declares a supported WIT export and the command includes trace context
- **THEN** the provider SHALL instantiate the component, invoke the export, route host imports through the service portal, and return a sanitized successful command result

#### Scenario: Component execution fails closed
- **WHEN** a component is invalid, misses a required export, traps, exceeds resource limits, or omits trace context
- **THEN** the provider SHALL return a structured sanitized diagnostic with a stable reason code and SHALL NOT log raw guest bytes, payloads, memory, secrets, filesystem paths, environment values, or network values

### Requirement: Component Provider Governance Boundary
The system SHALL keep Component Model engine dependencies and execution details
inside `macaca-runtime-host` and SHALL NOT add kernel, SDK, Web, CLI,
application framework, or proto dependencies on concrete WASM engines.

#### Scenario: Dependency boundary is preserved
- **WHEN** the Component Model provider is added
- **THEN** Route C dependency boundary checks SHALL pass without adding a new allowlist exception
