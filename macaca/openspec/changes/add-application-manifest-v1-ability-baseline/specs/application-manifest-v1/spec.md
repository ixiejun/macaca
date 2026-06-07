## ADDED Requirements

### Requirement: Application Manifest v1 Contract
The system SHALL define a provider-neutral Application Manifest v1 contract that can describe application package metadata, runtime profile, ability descriptors, permissions, service dependencies, UI declarations, commerce metadata, plugin dependencies, and compatibility constraints.

#### Scenario: Manifest describes multiple application forms
- **WHEN** a developer describes a YAML, WASM, GenUI, headless, hybrid, Store-distributed, or Plugin-enhanced application
- **THEN** Manifest v1 SHALL represent the application without requiring Web, Kernel, runtime-host provider, or business-specific types.

#### Scenario: Manifest remains provider-neutral
- **WHEN** Manifest v1 is compiled in `macaca-proto`
- **THEN** it SHALL NOT depend on `macaca-kernel`, `macaca-runtime-host`, `macaca-web`, provider crates, or application-specific code.

### Requirement: Ability Descriptor Model
The system SHALL define an Ability Descriptor model that allows an application to contain multiple abilities with independent kind, implementation, activation, lifecycle, permission, service, capability, and UI declarations.

#### Scenario: Minimum ability kinds are supported
- **WHEN** an application declares Agent, UI, Headless, Scheduled, Gateway, or Extension abilities
- **THEN** each ability SHALL be representable as a first-class descriptor under the application manifest.

#### Scenario: Ability descriptors are composable
- **WHEN** an application contains multiple abilities
- **THEN** the manifest SHALL preserve each ability's declarations without flattening them into ambiguous top-level fields.

### Requirement: Manifest and Ability Admission Specifications
The Application Framework SHALL provide reusable specification-style checks for manifest validity, ability validity, permission declarations, service requirements, capability declarations, trace requirements, runtime kind support, and compatibility declarations.

#### Scenario: Invalid declaration is rejected
- **WHEN** a manifest or ability references a required service or permission without a corresponding declaration
- **THEN** admission SHALL return structured diagnostics before runtime execution.

#### Scenario: Unsafe data is not logged
- **WHEN** manifest or ability admission fails
- **THEN** diagnostics and logs SHALL include safe ids, kinds, trace id when available, and reason codes, and SHALL NOT include prompt bodies, raw full manifest bodies, raw agent configs, secrets, env values, API keys, or raw host payloads.

### Requirement: Legacy YAML Compatibility Boundary
The system SHALL keep legacy YAML application models available while preventing YAML from becoming the privileged source for new Application Platform capabilities.

#### Scenario: YAML behavior is unchanged in this proposal
- **WHEN** existing YAML applications are loaded through legacy paths
- **THEN** their current behavior SHALL remain unchanged by the introduction of Manifest v1 contracts.

#### Scenario: YAML is prepared for adapter migration
- **WHEN** future proposals migrate YAML applications
- **THEN** they SHALL adapt YAML into Manifest v1 and AgentAbility descriptors rather than extending YAML as the only application schema.
