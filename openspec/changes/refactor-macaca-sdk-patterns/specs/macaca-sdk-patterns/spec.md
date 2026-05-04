## ADDED Requirements

### Requirement: SDK builder produces AgentSpec

The SDK SHALL provide an `AgentSpec` builder product that captures declarative agent configuration without directly requiring runtime registration.

#### Scenario: Build spec from config

- **WHEN** an `AgentBuilder` builds an `AgentSpec` from a valid `AgentConfig`
- **THEN** the spec contains the same name, capabilities, permission, prompt template, LLM options, and trace policy metadata needed to build the current `DeclarativeAgent`.

### Requirement: Existing agent builder behavior remains compatible

Existing `AgentBuilder::build` and `AgentBuilder::build_with_manifest` behavior SHALL remain compatible.

#### Scenario: Build declarative agent through compatibility path

- **WHEN** existing code calls `AgentBuilder::from_config(config).build_with_manifest()`
- **THEN** it receives a `DeclarativeAgent` and `AgentManifest` with fields equivalent to the pre-refactor behavior.

### Requirement: Persona prototype supports clone and override

The SDK SHALL provide persona prototype primitives that instantiate modified personas without mutating the original prototype.

#### Scenario: Override identity

- **WHEN** a persona prototype with base identity is instantiated with an identity override
- **THEN** the returned persona contains the override
- **AND** the prototype's base persona remains unchanged.

### Requirement: SDK validation is chain-based

The SDK SHALL validate agent configs through a default validation chain equivalent to current validation behavior.

#### Scenario: Invalid permission level

- **WHEN** an agent config has an unsupported permission level
- **THEN** validation fails with a config error equivalent to current behavior.

### Requirement: SDK facade registers agents through registry adapter

The SDK SHALL provide a facade that registers SDK agent declarations through a registry adapter while preserving current kernel registration semantics.

#### Scenario: Register from config through facade

- **WHEN** a valid agent config is registered through the SDK facade
- **THEN** the agent is registered in the kernel with the same manifest and runtime agent behavior as the existing helper path.

### Requirement: Deprecated registry helpers remain compatible

Existing registry helper functions SHALL remain callable after being marked deprecated.

#### Scenario: Existing register_from_file caller

- **WHEN** existing code calls `register_from_file`
- **THEN** registration succeeds using the new facade internally
- **AND** no application-specific behavior is introduced.

### Requirement: Deprecated builder compatibility methods remain compatible

Existing builder compatibility methods SHALL remain callable after being marked deprecated.

#### Scenario: Existing build_with_manifest caller

- **WHEN** existing code calls `AgentBuilder::build_with_manifest`
- **THEN** the method succeeds through the `AgentSpec` path internally
- **AND** the returned manifest remains equivalent to pre-refactor behavior.
