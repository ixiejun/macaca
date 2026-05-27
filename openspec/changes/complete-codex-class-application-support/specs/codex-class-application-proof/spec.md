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
