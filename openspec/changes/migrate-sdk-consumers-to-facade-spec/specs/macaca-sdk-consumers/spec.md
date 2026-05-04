## ADDED Requirements

### Requirement: Consumers use SDK facade for config registration

Upper consumers SHALL register SDK `AgentConfig` values through the `MacacaSdk` facade instead of deprecated registry helper functions.

#### Scenario: App runtime registers declarative agents

- **WHEN** `macaca-app` starts a declarative application and resolves agent configs
- **THEN** it registers each config through `MacacaSdk::for_kernel(kernel).register_config(config)`
- **AND** it does not call deprecated `macaca_sdk::register_from_config`

### Requirement: Consumers use AgentSpec for manual SDK-built registration

Upper consumers that manually register SDK-built agents SHALL build an `AgentSpec` and derive the manifest and runtime agent from that spec.

#### Scenario: Kernel tests manually register SDK-built agents

- **WHEN** a kernel or integration test needs to call `kernel.register_agent(...)` with an SDK-built agent
- **THEN** it builds an `AgentSpec` using `AgentBuilder::build_spec`
- **AND** it derives the manifest before converting the spec into the runtime agent
- **AND** it does not call deprecated `AgentBuilder::build_with_manifest`

### Requirement: Deprecated SDK APIs remain compatibility-only

Deprecated SDK APIs SHALL remain present inside `macaca-sdk` but SHALL NOT be used by upper consumer code.

#### Scenario: Deprecated usage scan

- **WHEN** the repository is scanned for `register_from_config`, `register_from_file`, and `build_with_manifest`
- **THEN** remaining usages are limited to `macaca-sdk` compatibility implementation or tests
- **AND** upper consumer crates do not use those deprecated entry points
