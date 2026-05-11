## ADDED Requirements

### Requirement: Macaca SHALL provide a stable Plugin SDK facade

Macaca SHALL provide a Plugin SDK facade that lets plugin authors and built-in adapter authors create manifests, register capabilities, register hooks, declare config/secret requirements, report health, and run contract tests without depending on internal kernel or runtime-host types.

#### Scenario: SDK builds a valid descriptor plugin manifest

- **WHEN** a developer uses the Plugin SDK manifest and capability builders to create a descriptor plugin
- **THEN** the SDK SHALL produce provider-neutral protocol DTOs accepted by Plugin Control Plane and Plugin Capability Registry
- **AND** the developer SHALL NOT need to import kernel or runtime-host internals

### Requirement: Macaca SHALL provide plugin contract test utilities

Macaca SHALL provide a Plugin Contract Test Kit that validates manifest shape, capability descriptors, hook descriptors, config/secret requirements, unavailable-safe behavior, and Route C boundary compliance.

#### Scenario: Contract test catches missing permission declaration

- **WHEN** a plugin declares a privileged capability without required permission metadata
- **THEN** the contract test kit SHALL fail with a structured diagnostic
- **AND** the diagnostic SHALL identify the missing declaration without exposing secrets or provider credentials

### Requirement: Macaca SHALL define plugin host skeletons behind Abstract Factory

Macaca SHALL define descriptor, built-in adapter, WASM, process, and remote proxy plugin host skeletons behind a runtime-host Abstract Factory.

#### Scenario: WASM host skeleton is unavailable-safe

- **WHEN** a plugin requests WASM execution before real WASM execution is implemented
- **THEN** Plugin Host Factory SHALL return a structured unavailable result or unavailable-safe host
- **AND** it SHALL NOT execute WASM bytes, panic, hang, or silently accept execution

#### Scenario: Process host skeleton does not spawn by default

- **WHEN** a plugin requests process execution before process execution is implemented
- **THEN** Plugin Host Factory SHALL return structured unavailable behavior
- **AND** it SHALL NOT spawn local processes or shell commands

### Requirement: Macaca SHALL trace host lifecycle operations

Macaca SHALL emit structured logs and trace/audit events for host selection, prepare, start, call, hook invocation, stop, cleanup, health probe, timeout, resource denial, and unavailable results.

#### Scenario: Remote proxy host health probe is auditable

- **WHEN** a remote proxy host health probe is requested
- **THEN** the host skeleton SHALL return structured health or unavailable status
- **AND** the operation SHALL emit trace/audit data with plugin id, runtime kind, operation, status, trace id, and structured error code

### Requirement: Macaca SHALL document plugin SDK and host usage

Macaca SHALL document descriptor plugins, built-in adapter plugins, hook plugins, capability plugins, WASM skeleton plugins, process skeleton plugins, remote proxy skeleton plugins, and contract test commands in the plugin development guide.

#### Scenario: Developer follows minimal descriptor plugin guide

- **WHEN** a developer reads the minimal descriptor plugin guide
- **THEN** the guide SHALL show manifest construction, capability declaration, permission/resource declaration, validation, and contract test commands
- **AND** it SHALL explain unavailable-safe behavior for unimplemented execution hosts
