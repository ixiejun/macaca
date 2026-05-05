## ADDED Requirements

### Requirement: Macaca SHALL provide active memory recall before model calls

Macaca SHALL provide an active recall capability that can prefetch relevant memories before a model call according to policy, scope, token budget, and latency budget.

#### Scenario: Active recall queries private and shared memory

- **GIVEN** an agent is about to perform a model call
- **AND** active recall is enabled
- **WHEN** memory prefetch runs with the current request context
- **THEN** it SHALL query current agent `AgentPrivate` memory
- **AND** it SHALL query relevant `SessionShared` memory
- **AND** it MAY query `ApplicationShared`, `UserScoped`, knowledge, or supplements according to policy

#### Scenario: Active recall can be disabled

- **GIVEN** active recall is disabled by configuration or policy
- **WHEN** a model call is assembled
- **THEN** no active memory recall SHALL be injected
- **AND** normal explicit memory tools SHALL remain available according to tool policy

### Requirement: Active recall SHALL respect budget and latency policy

Active recall SHALL bound its work by max hits, max chars/tokens, provider timeout, and total latency budget.

#### Scenario: Recall results are truncated by budget

- **GIVEN** memory search returns more candidates than budget allows
- **WHEN** active recall selects context
- **THEN** it SHALL select only candidates fitting configured max hits and token/char budget
- **AND** it SHALL record skipped decisions for omitted candidates

#### Scenario: Slow provider times out

- **GIVEN** one memory provider exceeds active recall timeout
- **WHEN** active recall is running
- **THEN** Macaca SHALL record timeout diagnostics
- **AND** it SHALL continue with other provider results or an empty recall result
- **AND** it SHALL not block the model call indefinitely

### Requirement: Active recall output SHALL be dynamic and request-only

Active recall output SHALL be treated as dynamic request context, not canonical transcript and not system instruction.

#### Scenario: Recall injection is not persisted as transcript

- **GIVEN** active recall returns memory snippets for a model call
- **WHEN** the model request is assembled
- **THEN** recall snippets MAY appear in dynamic context sections
- **AND** those snippets SHALL NOT be written back to canonical session transcript

#### Scenario: Recall content is fenced from instructions

- **GIVEN** recalled memory content comes from file, vector store, remote provider, MCP provider, or knowledge supplement
- **WHEN** it is rendered into context
- **THEN** it SHALL be marked/fenced as memory context
- **AND** it SHALL NOT be treated as higher-priority system instruction

### Requirement: Active recall SHALL generate diagnostics

Active recall SHALL produce diagnostics suitable for context reports, trace UI, and debugging.

#### Scenario: Report includes recall source breakdown

- **GIVEN** active recall selects memory candidates
- **WHEN** the context report is produced
- **THEN** it SHALL include provider id, source visibility, scope summary, score, selected/skipped status, size estimate, and latency

#### Scenario: Full memory content is not persisted by default

- **GIVEN** active recall diagnostics are stored
- **WHEN** debug full-content capture is not explicitly enabled
- **THEN** diagnostics SHALL NOT persist full memory content
- **AND** they SHALL store ids, hashes, snippets or summaries, source labels, and decision metadata

### Requirement: Active recall strategy SHALL be replaceable

Macaca SHALL allow active recall strategy/provider replacement through configuration and registry.

#### Scenario: Custom active recall strategy selected

- **GIVEN** memory profile config selects a custom active recall id
- **WHEN** memory prefetch runs
- **THEN** Macaca SHALL instantiate the configured active recall capability
- **AND** upper application code SHALL continue calling the same memory/context facade

#### Scenario: Strategy cannot bypass scope policy

- **GIVEN** a custom active recall strategy is selected
- **WHEN** it queries memory
- **THEN** it SHALL still use scoped memory facade/provider APIs
- **AND** it SHALL NOT read another agent private memory unless policy explicitly grants access
