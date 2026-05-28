## ADDED Requirements

### Requirement: Application-neutral Codex-class Proof
The system SHALL include an application-neutral proof that demonstrates a
Codex-class coding workflow using generic Macaca OS services.

#### Scenario: Run real coding workflow through services
- **WHEN** the proof application runs
- **THEN** it SHALL start a thread and turn, inspect repository files, search
  code, apply a patch, run a sandboxed process, call tool/MCP/skill providers,
  perform review, emit diagnostics, and replay audit
- **AND** each step SHALL use service-owned commands rather than shell-owned or
  application-specific OS code

#### Scenario: Stream proof workflow
- **WHEN** the workflow runs through the app protocol gateway
- **THEN** Thread/Turn/Item events, process output, file changes, tool
  lifecycle, approvals, hooks, review findings, and diagnostics SHALL stream as
  typed bounded notifications

### Requirement: Application Framework Manifest Integration
The Application Framework SHALL let YAML, WASM, GenUI, headless, and
workbench-style applications declare the same generic workbench capability
surface through provider-neutral manifest data.

#### Scenario: Admit declared workbench capabilities
- **WHEN** an application manifest declares workbench capabilities, permission
  profiles, tool families, service dependencies, optional providers, plugin
  dependencies, MCP dependencies, skill bundles, event subscriptions, and UI
  surfaces
- **THEN** Application Framework admission SHALL validate those declarations
  without hardcoding application names, product workflows, or provider names
- **AND** sanitized Application Service metadata SHALL expose only bounded refs,
  names, and counts for service clients, context, tool planning, app protocol,
  and shells

### Requirement: No Application-specific OS Branches
The system SHALL prove that supporting a Codex-class coding application does not
introduce application-specific branches below the application layer.

#### Scenario: Boundary scan
- **WHEN** boundary tests scan kernel, SDK, runtime-host, services, Web, CLI,
  and frontend code
- **THEN** they SHALL fail if OS-layer routing branches on application name,
  product workflow name, provider name, model name, gateway name, chain name, or
  business-domain name

### Requirement: Full Support Completion Gate
The change SHALL NOT be considered complete when only descriptors, skeleton
services, or catalog visibility are implemented.

#### Scenario: Descriptor-only implementation
- **WHEN** services expose descriptors but cannot run the full proof workflow
- **THEN** the proposal SHALL remain incomplete
- **AND** tasks SHALL stay unchecked until provider-backed execution, policy,
  audit, streaming, and replay proof pass
