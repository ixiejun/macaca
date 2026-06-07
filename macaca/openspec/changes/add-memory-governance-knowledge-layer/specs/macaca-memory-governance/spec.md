## ADDED Requirements

### Requirement: Macaca SHALL separate memory candidates from committed memory

Macaca SHALL provide a candidate layer for automatically captured or inferred memories so that automatic capture does not directly pollute long-term committed memory.

#### Scenario: Automatic capture writes candidate

- **GIVEN** auto capture observes a turn, tool result, delegation result, or agent self-summary
- **WHEN** the content is considered possibly useful
- **THEN** it SHALL be written as a memory candidate
- **AND** it SHALL NOT become committed long-term memory until promoted by policy or explicit user action

#### Scenario: Explicit remember can commit high-confidence memory

- **GIVEN** the user explicitly asks Macaca or an agent to remember something
- **WHEN** the request is valid under scope and privacy policy
- **THEN** Macaca MAY write it directly as committed high-confidence memory
- **AND** it SHALL record provenance and audit metadata

### Requirement: Promotion policy SHALL be replaceable and auditable

Macaca SHALL provide a promotion policy that decides whether candidates become committed memory and SHALL allow replacement by custom policy.

#### Scenario: Default promotion is conservative

- **GIVEN** a candidate is automatically captured from ordinary conversation
- **WHEN** default promotion policy evaluates it
- **THEN** it SHALL consider explicitness, recurrence, confidence, freshness, visibility, privacy, and conflict status
- **AND** it SHALL reject or defer weak candidates

#### Scenario: Promotion records decision

- **GIVEN** a candidate is promoted, rejected, deferred, or merged
- **WHEN** the decision is made
- **THEN** Macaca SHALL record a promotion decision with reason, policy id, source candidate id, target scope, and timestamp

### Requirement: Memory deletion SHALL use tombstones and propagation

Macaca SHALL support deletion semantics that prevent deleted memories from reappearing through index rebuilds, provider sync, or artifacts.

#### Scenario: Delete writes tombstone

- **GIVEN** a committed memory is deleted
- **WHEN** delete succeeds or is accepted
- **THEN** Macaca SHALL write a tombstone for that memory id/scope
- **AND** future rebuild or replay SHALL NOT resurrect the deleted memory

#### Scenario: Delete propagates to backends

- **GIVEN** a memory exists in file/session/vector/remote provider/artifact outputs
- **WHEN** delete is requested
- **THEN** Macaca SHALL attempt deletion or redaction in all relevant backends
- **AND** it SHALL record propagation diagnostics for each backend

#### Scenario: Delete failure is retryable

- **GIVEN** a remote provider or vector backend delete fails
- **WHEN** the failure is recoverable
- **THEN** Macaca SHALL record retryable diagnostics
- **AND** it SHALL keep the tombstone authoritative locally

### Requirement: Macaca SHALL maintain memory audit events

Macaca SHALL maintain audit events for memory write, candidate capture, promotion, merge, conflict, delete, provider sync, and artifact generation.

#### Scenario: Write event includes provenance

- **GIVEN** a memory is written
- **WHEN** audit event is created
- **THEN** it SHALL include memory id, scope, source agent/session/turn/tool when available, operation kind, timestamp, and provider id when applicable

#### Scenario: Audit output redacts secrets

- **GIVEN** audit event metadata contains provider credentials, headers, tokens, or secret-bearing content
- **WHEN** audit event is displayed or exported
- **THEN** secret values SHALL be redacted

### Requirement: Macaca SHALL provide a knowledge compiler capability

Macaca SHALL provide a knowledge compiler capability that can compile raw/candidate/committed memories into structured claims, evidence, decisions, constraints, preferences, freshness markers, and conflict groups.

#### Scenario: Claim includes evidence

- **GIVEN** a knowledge compiler emits a claim
- **WHEN** the claim is stored
- **THEN** it SHALL include references to source memory ids, candidate ids, artifacts, or events as evidence
- **AND** claims without evidence SHALL be marked low confidence or rejected according to policy

#### Scenario: Conflict is represented

- **GIVEN** two memories or claims contradict each other
- **WHEN** the knowledge layer detects or receives the conflict
- **THEN** it SHALL record a conflict group
- **AND** it SHALL preserve freshness/confidence/provenance needed for downstream policy decisions

#### Scenario: Compiled digest can feed context engine

- **GIVEN** a compiled knowledge digest exists for a scope
- **WHEN** context assembly or active recall requests high-quality memory context
- **THEN** Macaca MAY provide the digest as a bounded context source
- **AND** the digest SHALL include source/evidence references for traceability

### Requirement: Memory artifacts SHALL expose summaries safely

Macaca SHALL support memory artifacts such as markdown reports, project decision logs, wiki digests, and governance summaries.

#### Scenario: Artifact lists public summary

- **GIVEN** artifacts are requested for an application/session/project
- **WHEN** the artifact provider lists available outputs
- **THEN** it SHALL include kind, scope, relative path or logical id, content type, updated time, and redaction status

#### Scenario: Artifact avoids raw sensitive leakage by default

- **GIVEN** an artifact is generated without explicit debug/full-content configuration
- **WHEN** the artifact content is produced
- **THEN** it SHALL avoid dumping full sensitive memory contents by default
- **AND** it SHALL prefer summaries, claims, evidence ids, hashes, and redacted excerpts
