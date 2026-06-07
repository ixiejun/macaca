## ADDED Requirements

### Requirement: Skill Curation Operations Shell

The system SHALL expose Skill governance, curation dry-run, alias, and draft
experience proposal state to Web and frontend callers only through service or
SDK facade boundaries.

#### Scenario: Web route aggregates service-owned Skill operations state
- **GIVEN** an operator opens the application operations surface
- **WHEN** the frontend requests Skill operations state for an application
- **THEN** the Web shell calls the SDK Skill client for governance snapshot, curation dry-run, alias snapshot, and experience proposal snapshot
- **AND** the response contains sanitized counts and records without raw skill instructions, prompts, manifests, package bytes, provider payloads, or secrets

#### Scenario: Shell does not own curation semantics
- **GIVEN** the Skill service returns dry-run recommendations, aliases, lifecycle states, and draft proposals
- **WHEN** Web or frontend renders the operations surface
- **THEN** it displays the returned service DTOs
- **AND** it does not implement archive, merge, stale, alias, promotion, rejection, or patch classification rules locally

### Requirement: Read-Only Skill Operations Panel

The system SHALL provide a read-only application operations panel for Skill
governance and evolution state until policy-approved mutation commands are
specified.

#### Scenario: Panel refreshes sanitized state
- **GIVEN** Skill operations state is available through the Web shell
- **WHEN** the operator refreshes the Skill operations panel
- **THEN** the panel reloads the aggregated service snapshot
- **AND** no skill files, governance records, aliases, lifecycle states, or proposals are mutated
