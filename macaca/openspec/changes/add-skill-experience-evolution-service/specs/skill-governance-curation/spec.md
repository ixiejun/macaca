## ADDED Requirements

### Requirement: Draft-Only Skill Experience Proposals

The system SHALL expose a traced Skill service command that converts sanitized verified task evidence into a draft skill experience proposal without creating, patching, archiving, deleting, or activating skill files.

#### Scenario: Verified reusable task evidence creates proposal
- **GIVEN** a task has verified evidence references and a bounded reusable procedure summary
- **WHEN** a caller invokes the Skill experience proposal command through the Skill service
- **THEN** the service returns a draft proposal record with sanitized evidence references
- **AND** the result states that active skill state was not mutated

#### Scenario: Missing evidence is rejected
- **GIVEN** a task experience candidate has no evidence references
- **WHEN** the Skill experience proposal command is invoked
- **THEN** the service rejects the proposal with a structured validation error
- **AND** no proposal record is stored

### Requirement: Service-Owned Experience Evolution Boundary

The system SHALL keep experience classification and proposal creation inside replaceable Skill service providers and expose it to shells, task services, and applications only through SDK/facade clients.

#### Scenario: Shell or task caller consumes facade
- **GIVEN** a caller needs to propose reusable task experience
- **WHEN** it sends the request
- **THEN** it uses the SDK Skill client or service facade
- **AND** it does not write skill files or implement evolution classification locally

### Requirement: Sanitized Proposal Observability

The system SHALL log and return bounded proposal metadata without raw prompts, raw provider payloads, secrets, manifests, package bytes, or unbounded task output.

#### Scenario: Proposal records contain bounded metadata
- **GIVEN** reusable task experience is proposed
- **WHEN** the service records the proposal
- **THEN** the stored proposal includes trace id, proposal id, action, classification, bounded summary, and evidence ids
- **AND** it excludes raw prompts, raw task output, and provider payloads
