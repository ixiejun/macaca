## ADDED Requirements

### Requirement: Macaca SHALL load agent profile files through a context provider

Macaca SHALL provide a profile-file context provider that converts configured agent profile files into `ContextCandidate` values for the context composer.

#### Scenario: Existing profile file becomes candidate

- **GIVEN** an agent profile root contains `AGENTS.md`
- **WHEN** a model request is composed for that agent
- **THEN** the profile provider SHALL load the file through the safe-load pipeline
- **AND** it SHALL emit a context candidate with source id, file kind, priority, trust, cache class, budget, and diagnostics

#### Scenario: Missing profile files are skipped

- **GIVEN** an agent profile root does not contain `SOUL.md`
- **WHEN** the profile provider scans profile files
- **THEN** composition SHALL continue
- **AND** the missing optional file SHALL NOT fail request assembly

### Requirement: Profile file kinds SHALL have default priority and target policy

Macaca SHALL define default policy for `AGENTS.md`, `SOUL.md`, `TOOLS.md`, `IDENTITY.md`, `USER.md`, `HEARTBEAT.md`, and `MEMORY.md`, while allowing replacement through policy abstractions.

#### Scenario: High-priority behavior files are selected first

- **GIVEN** `AGENTS.md`, `SOUL.md`, and `USER.md` all exist
- **WHEN** budget does not allow all profile files to be injected
- **THEN** default policy SHALL prefer `AGENTS.md` and `SOUL.md` before `USER.md`
- **AND** skipped files SHALL be recorded in the context report

#### Scenario: Heartbeat content can target heartbeat stage

- **GIVEN** `HEARTBEAT.md` exists
- **WHEN** the current composition target is a normal user turn
- **THEN** default policy MAY skip or lower priority for `HEARTBEAT.md`
- **AND** the decision SHALL be reportable

### Requirement: Profile loading SHALL enforce safety and budget boundaries

Profile file loading SHALL enforce root containment, maximum size, read diagnostics, and per-kind budget before candidate creation.

#### Scenario: Path escape is rejected

- **GIVEN** a profile source resolves outside the configured profile root
- **WHEN** the provider validates the file path
- **THEN** the file SHALL be rejected
- **AND** the report SHALL include a path escape skip reason

#### Scenario: Oversized file is truncated or skipped by policy

- **GIVEN** `TOOLS.md` exceeds the configured profile file budget
- **WHEN** the provider loads the file
- **THEN** the provider SHALL truncate or skip it according to policy
- **AND** the report SHALL record the size and decision

### Requirement: MEMORY.md SHALL be treated as memory seed or audit context, not direct long-term memory

Macaca SHALL NOT automatically persist `MEMORY.md` content into long-term vector memory during profile context loading.

#### Scenario: MEMORY.md is visible as profile context only

- **GIVEN** an agent profile root contains `MEMORY.md`
- **WHEN** the profile provider loads it
- **THEN** it MAY emit a bounded profile candidate
- **AND** it SHALL NOT write the file content to the memory facade or vector backend

#### Scenario: Memory import requires explicit memory workflow

- **GIVEN** a user wants `MEMORY.md` content imported into long-term memory
- **WHEN** no explicit memory import workflow has been invoked
- **THEN** the profile provider SHALL NOT promote the content automatically

### Requirement: Profile context decisions SHALL be auditable

Macaca SHALL include profile provider decisions in `ContextReport`.

#### Scenario: Report shows loaded and skipped profile files

- **GIVEN** a model request uses profile context
- **WHEN** its context report is inspected
- **THEN** the report SHALL include each considered profile file kind, source path or source id, selected/skipped status, estimated size, and decision reason
