## ADDED Requirements

### Requirement: Developer-Facing Application SDK Kits
The SDK SHALL expose developer-facing `ApplicationKit` and `AbilityKit` facades for constructing provider-neutral Application Manifest v1 and Ability Descriptor contracts.

#### Scenario: Developer builds an application manifest
- **WHEN** an application author uses `ApplicationKit` to declare package metadata, runtime, abilities, permissions, services, UI, commerce, or plugin dependencies
- **THEN** the SDK SHALL produce serializable Manifest v1 contracts without requiring runtime-host, Web, Kernel, or provider implementation types.

#### Scenario: Developer builds abilities
- **WHEN** an application author uses `AbilityKit` to declare Agent, UI, Headless, Scheduled, Gateway, or Extension abilities
- **THEN** the SDK SHALL produce first-class ability descriptors with stable permission, service, capability, activation, and lifecycle declarations.

### Requirement: SDK Provider-Neutrality
The SDK SHALL NOT construct `AppRuntime`, `Kernel`, `ServiceRuntime`, Web state, runtime-host providers, or provider concrete implementations for Application Platform development helpers.

#### Scenario: SDK compiles without provider construction
- **WHEN** `macaca-sdk` is compiled with Application SDK Kits
- **THEN** it SHALL depend only on provider-neutral contracts and allowed facade dependencies, not runtime-host provider or Web internals.

#### Scenario: Shell client remains separate
- **WHEN** callers need to control installed/running applications
- **THEN** they SHALL use `SystemApplicationClient`, while developer-authored package construction SHALL remain in ApplicationKit/AbilityKit.

### Requirement: Application Contract TestKit
The SDK SHALL provide an Application Contract TestKit that validates manifest, ability, permission, service dependency, trace, runtime, and unsafe payload rules without executing application code.

#### Scenario: Invalid fixture is rejected
- **WHEN** a test fixture omits a required permission, service dependency, ability entry, or trace-required command
- **THEN** the TestKit SHALL return structured diagnostics before runtime execution.

#### Scenario: TestKit diagnostics are safe
- **WHEN** TestKit reports diagnostics
- **THEN** diagnostics SHALL include fixture id, ability kind, operation, reason code, and trace id when supplied, and SHALL NOT include secrets, env values, API keys, raw host payloads, prompt bodies, or unbounded manifest bodies.

### Requirement: Application SDK Examples
The SDK SHALL include generic examples or fixtures for declarative, GenUI, headless, plugin-enhanced, Store-entitled, and WASM skeleton applications.

#### Scenario: Examples are generic
- **WHEN** examples are inspected
- **THEN** they SHALL use generic fixture identifiers and SHALL NOT hardcode business app names, provider names, workflow names, driver names, gateway names, or chain names.
