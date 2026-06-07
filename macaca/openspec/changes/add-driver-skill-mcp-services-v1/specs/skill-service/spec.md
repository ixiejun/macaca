## ADDED Requirements

### Requirement: Skill Service Contract

The system SHALL expose a provider-neutral Skill Service with operations for skill snapshot, executable skill loading, skill tool catalog, skill invocation, status, service snapshot, and cleanup.

#### Scenario: Skill snapshot is requested through service

- **WHEN** Web or SDK requests a skill snapshot
- **THEN** the Skill Service SHALL return provider-neutral snapshot metadata compatible with existing skill snapshot behavior
- **AND** the caller SHALL NOT need direct access to `SkillRuntimeFacade`.

#### Scenario: Executable skills are loaded

- **WHEN** executable skill loading is requested with trace context and scope
- **THEN** the Skill Service SHALL delegate to the configured executable skill loader/facade
- **AND** the result SHALL report loaded, skipped, and failed skill entries as structured metadata.

### Requirement: Skill Tool Catalog

The Skill Service SHALL expose executable skill tools through sanitized capability tool descriptors while retaining Skill Service ownership of skill invocation.

#### Scenario: Skill tools are cataloged

- **WHEN** the framework toolkit requests skill tools
- **THEN** the Skill Service SHALL return sanitized descriptors with origin kind `skill`
- **AND** descriptors SHALL NOT include env, headers, credentials, package secrets, entitlement secrets, or full `SKILL.md` bodies.

#### Scenario: Entitlement readiness is represented

- **WHEN** a skill package requires entitlement or encrypted package readiness
- **THEN** the Skill Service SHALL expose only readiness/status hooks in S6
- **AND** full entitlement decisions SHALL remain outside S6 unless handled by an existing entitlement service.

### Requirement: Skill Tool Invocation

The Skill Service SHALL invoke executable skill tools only through typed, traced, policy-checkable commands.

#### Scenario: Skill tool is invoked through service client

- **WHEN** a Web toolkit tool adapter invokes an executable skill tool
- **THEN** the adapter SHALL call the Skill Service client instead of direct executable skill registration/invocation
- **AND** the service SHALL emit structured logs/events for command accepted, policy checked, dispatch started, completion or failure.

#### Scenario: Skill runtime is unavailable

- **WHEN** no skill runtime/facade is configured
- **THEN** the Skill Service SHALL return structured unavailable or empty sanitized inventory as appropriate
- **AND** the caller SHALL NOT panic or implicitly construct a skill runtime.

### Requirement: Deprecated Skill Compatibility Anchors

Existing direct skill runtime/facade APIs SHALL remain available as deprecated, searchable compatibility anchors during S6 migration.

#### Scenario: Legacy path remains searchable

- **WHEN** a developer searches for deprecated direct skill runtime usage
- **THEN** the codebase SHALL retain explicit deprecated markers or compatibility wrappers
- **AND** new production call paths SHALL prefer the Skill Service client.
