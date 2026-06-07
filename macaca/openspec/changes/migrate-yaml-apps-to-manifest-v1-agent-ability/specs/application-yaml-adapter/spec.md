## ADDED Requirements

### Requirement: YAML to Manifest v1 Adapter
The Application Framework SHALL provide an adapter that converts legacy YAML application manifests into Application Manifest v1 without changing existing YAML runtime behavior.

#### Scenario: Existing YAML application is preserved
- **WHEN** an existing YAML application is loaded through the compatibility path
- **THEN** entry agent resolution, agent config resolution, workflows, resources, context, skills, and tool policy behavior SHALL remain compatible with the current implementation.

#### Scenario: YAML is projected to Manifest v1
- **WHEN** a YAML application is adapted for the Application Platform
- **THEN** the adapter SHALL produce an Application Manifest v1 projection rather than extending YAML as the privileged platform schema.

### Requirement: YAML Agents Become AgentAbility
The YAML adapter SHALL project YAML agent declarations into AgentAbility descriptors.

#### Scenario: Inline agent becomes ability
- **WHEN** a YAML manifest contains inline agent definitions
- **THEN** each relevant agent declaration SHALL be represented in AgentAbility metadata with sanitized capability, service, permission, and activation data.

#### Scenario: File-path agent remains compatible
- **WHEN** a YAML manifest references agent config files
- **THEN** the adapter SHALL preserve legacy file resolution behavior while exposing sanitized AgentAbility projections.

### Requirement: YAML Conversion Report
The YAML adapter SHALL return a conversion report describing inferred defaults, compatibility warnings, legacy-only fields, and projection diagnostics.

#### Scenario: Defaults are reported
- **WHEN** the adapter infers entry agent, runtime kind, ability id, or default permissions
- **THEN** the conversion report SHALL record the inference with safe ids and reason codes.

#### Scenario: Report is sanitized
- **WHEN** conversion diagnostics are serialized or logged
- **THEN** they SHALL NOT include prompt bodies, raw full manifest bodies, raw agent configs, secrets, env values, API keys, or raw host payloads.

### Requirement: Descriptor Generation Prefers Manifest v1
Package and ABI descriptor generation SHALL prefer Manifest v1 projections for YAML applications while retaining deprecated legacy helpers for compatibility.

#### Scenario: Package descriptor preserves key fields
- **WHEN** a YAML application package descriptor is generated through the new projection
- **THEN** it SHALL preserve application id, package id, runtime kind, entry, permissions, service requirements, and declared capabilities compatible with the legacy descriptor path.

#### Scenario: Deprecated helper remains searchable
- **WHEN** old descriptor helpers remain in code
- **THEN** they SHALL be marked deprecated and SHALL NOT be the preferred new production path.
