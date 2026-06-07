## ADDED Requirements

### Requirement: Macaca SHALL expose governed knowledge digests as context candidates

Macaca SHALL provide a knowledge digest context provider that adapts governed memory claims, compiled digests, and artifacts into context candidates.

#### Scenario: Compiled digest becomes context candidate

- **GIVEN** memory governance has compiled claims or digest artifacts for the current scope
- **WHEN** a model request is composed
- **THEN** `KnowledgeDigestContextProvider` SHALL adapt eligible digest items into context candidates
- **AND** those candidates SHALL include source id, scope, confidence, freshness, evidence references, trust level, budget, and diagnostics

#### Scenario: No digest does not fail composition

- **GIVEN** no compiled digest is available for the current scope
- **WHEN** knowledge digest provider runs
- **THEN** context composition SHALL continue
- **AND** the provider SHALL emit an empty result or diagnostic without failing the model request

### Requirement: Digest-vs-raw selection SHALL prefer governed knowledge only when supported by evidence

Macaca SHALL use a replaceable strategy to choose between governed digest/claim context and raw active recall candidates.

#### Scenario: Supported digest outranks same-source raw recall

- **GIVEN** a digest claim and a raw recall candidate refer to the same evidence source
- **AND** the digest claim has sufficient confidence, freshness, and evidence coverage
- **WHEN** context selection runs
- **THEN** Macaca SHALL prefer the digest claim over the duplicate raw recall candidate
- **AND** the skipped raw candidate SHALL be reported as covered by governed digest

#### Scenario: Stale digest does not outrank fresh recall

- **GIVEN** a digest claim is stale according to freshness policy
- **AND** a fresh raw recall candidate is relevant to the current request
- **WHEN** digest-vs-raw selection runs
- **THEN** Macaca SHALL NOT blindly prefer the stale digest
- **AND** it SHALL select, annotate, or combine candidates according to freshness and confidence policy

### Requirement: Knowledge digest context SHALL preserve evidence traceability without raw sensitive leakage

Knowledge digest context SHALL expose evidence references for auditability while avoiding full sensitive source leakage by default.

#### Scenario: Evidence is reported by reference

- **GIVEN** a digest claim includes evidence from memory ids, candidate ids, events, or artifacts
- **WHEN** the claim is included in context report
- **THEN** the report SHALL include evidence ids, source labels, hashes, or artifact ids
- **AND** it SHALL NOT include full raw evidence content by default

#### Scenario: Sensitive evidence is redacted

- **GIVEN** an evidence reference or digest content matches redaction policy
- **WHEN** knowledge digest context is rendered or reported
- **THEN** Macaca SHALL redact or skip sensitive fields according to policy
- **AND** the redaction decision SHALL be reportable without leaking the sensitive value

### Requirement: Tombstones and deletion propagation SHALL apply to knowledge digest context

Knowledge digest context SHALL respect memory tombstones and deletion propagation so deleted memories cannot reappear through compiled claims or artifacts.

#### Scenario: Tombstoned evidence is excluded

- **GIVEN** a digest claim references tombstoned memory evidence
- **WHEN** knowledge digest provider validates the claim
- **THEN** the tombstoned evidence SHALL be excluded
- **AND** the claim SHALL be skipped, downgraded, or redacted according to policy

#### Scenario: Tombstone decision is reportable

- **GIVEN** a digest candidate is skipped because of tombstone propagation
- **WHEN** the context report is generated
- **THEN** the report SHALL include a tombstone-related decision reason
- **AND** it SHALL NOT reveal deleted memory content

### Requirement: Knowledge digest context SHALL obey scope and request-only boundaries

Knowledge digest context SHALL obey memory scope visibility and SHALL NOT mutate canonical session transcript.

#### Scenario: Agent private digest remains private

- **GIVEN** a digest claim belongs to one agent's `AgentPrivate` scope
- **WHEN** another agent in the same application composes context
- **THEN** the private digest SHALL NOT be visible unless policy explicitly grants access

#### Scenario: Digest context is not written to transcript

- **GIVEN** a digest claim is included in a model request
- **WHEN** the request is sent to the LLM provider
- **THEN** the digest may appear as governed memory context
- **AND** it SHALL NOT be written into canonical session messages

### Requirement: Knowledge digest provider SHALL be pluggable and governance-aware

Macaca SHALL allow replacement of knowledge digest provider or digest-vs-raw selection policy without coupling runtime/framework to memory internals.

#### Scenario: Custom digest provider is selected

- **GIVEN** configuration selects a custom knowledge digest provider
- **WHEN** context composition runs
- **THEN** runtime/framework SHALL still call the context facade
- **AND** the custom provider output SHALL pass candidate validation, budget, redaction, trust, and report policy

#### Scenario: Provider cannot bypass governance

- **GIVEN** a custom provider returns digest content with missing evidence or invalid scope
- **WHEN** provider runtime validates output
- **THEN** Macaca SHALL reject, downgrade, or skip the candidate according to policy
- **AND** the decision SHALL be reportable
