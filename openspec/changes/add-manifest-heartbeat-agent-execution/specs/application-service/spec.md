## ADDED Requirements

### Requirement: Manifest-Declared Heartbeat Agents
Application manifests SHALL be able to declare heartbeat-participating agents
under an application-owned autonomy heartbeat section.

#### Scenario: Manifest declares one heartbeat agent
- **WHEN** an application manifest contains `autonomy.heartbeat.enabled: true`
  and one `autonomy.heartbeat.agents[]` entry whose name matches a declared
  application agent
- **THEN** the application manifest parser SHALL accept the declaration
- **AND** the declaration SHALL NOT grant execution without service policy and
  runtime-host dispatch.

#### Scenario: Heartbeat file does not select agents
- **WHEN** an agent profile directory contains `HEARTBEAT.md`
- **AND** the application manifest does not declare that agent under
  `autonomy.heartbeat.agents[]`
- **THEN** the application SHALL NOT treat that agent as a heartbeat participant.

### Requirement: Sanitized Heartbeat Agent Projection
Application Service SHALL expose a traced, sanitized projection of manifest
heartbeat-agent declarations.

#### Scenario: Projection returns bounded declarations
- **WHEN** Runtime Host sends an application-scoped heartbeat-agent query with a
  trace context
- **THEN** Application Service SHALL return only application id, agent name,
  enabled state, profile id, bounded metadata, and diagnostics
- **AND** it SHALL NOT return raw manifests, prompt bodies, `HEARTBEAT.md`
  content, secrets, package bytes, WASM bytes, or unbounded payloads.

#### Scenario: Unknown declared agent
- **WHEN** a heartbeat declaration references an agent that is not declared by
  the application manifest
- **THEN** Application Service SHALL return structured invalid-manifest
  diagnostics for that declaration
- **AND** Runtime Host SHALL skip dispatch for that declaration.
