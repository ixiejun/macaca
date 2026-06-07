## ADDED Requirements

### Requirement: Code Intelligence Service
The system SHALL provide `service.code_intelligence` for provider-backed code
search, symbol context, file reference discovery, and analyzer diagnostics.

#### Scenario: Search code through provider adapter
- **WHEN** an application searches code
- **THEN** the service SHALL route through a configured analyzer provider or
  return structured unavailable
- **AND** results SHALL include sanitized paths, snippets within budget, trace
  refs, and provider health diagnostics

### Requirement: Git and Patch Service
The system SHALL provide `service.git` for repository status, diff, apply patch,
rollback marker creation, path policy, and patch provenance.

#### Scenario: Apply patch with provenance
- **WHEN** an application applies a patch
- **THEN** `service.git` SHALL validate path policy, record pre-change mementos,
  apply the patch, and emit audit refs
- **AND** rollback markers SHALL allow later replay and recovery

### Requirement: Review Service
The system SHALL provide `service.review` for generic review start, review
progress, structured findings, review results, and artifact-backed evidence.

#### Scenario: Review produces findings
- **WHEN** a review completes
- **THEN** the service SHALL return structured findings with severity, location,
  rationale, trace refs, and bounded evidence
- **AND** it SHALL not embed coding-product-specific workflow semantics

### Requirement: Diagnostics Service
The system SHALL provide `service.diagnostics` for health snapshots, feedback
upload, trace bundles, redaction, and bounded diagnostic reports.

#### Scenario: Build diagnostic bundle
- **WHEN** an operator requests a diagnostic bundle
- **THEN** the service SHALL gather trace, audit, config, provider health, and
  artifact refs through focused clients
- **AND** raw prompts, secrets, file contents, provider payloads, and unbounded
  logs SHALL be redacted or omitted

### Requirement: Optional Realtime and Remote Environment Services
The system SHALL provide optional `service.realtime` and
`service.remote_environment` contracts with unavailable behavior when providers
are absent.

#### Scenario: Remote environment absent
- **WHEN** an application selects a remote environment provider that is not
  installed or healthy
- **THEN** the service SHALL return structured unavailable diagnostics
- **AND** local workflows and base OS startup SHALL continue
