## ADDED Requirements

### Requirement: Context composition SHALL use the composer facade for framework web model calls

Macaca SHALL route macaca-web framework model preparation through `ContextFacade` + `assemble_context_providers` so provider families remain configuration-driven.

#### Scenario: Framework chat uses configured provider families

- **GIVEN** a framework `ReActAgent` is constructed through `macaca-web` with merged `ContextConfig`
- **WHEN** the agent issues a model call
- **THEN** context assembly SHALL run the composer pipeline before the context engine stage
- **AND** a `ContextReport` SHALL record composer plan metadata including stable and dynamic candidate fingerprints when candidates were selected

---

### Requirement: Workspace active recall SHALL respect tombstoned memory identifiers

Macaca SHALL prevent tombstoned workspace memory rows from appearing in active recall hits when a `TombstoneIndex` is wired for that agent session.

#### Scenario: Tombstone suppresses recalled row

- **GIVEN** a memory id is recorded in the shared tombstone registry used by workspace memory tools
- **AND** active vector recall is enabled with an in-process `TestMemoryManager`
- **WHEN** recall runs for a query that would otherwise return that row
- **THEN** the row SHALL NOT appear in recall results
- **AND** snapshot errors on the tombstone index SHALL fail open (recall continues) with operator-visible warnings

---

### Requirement: Profile files SHALL be loaded with path confinement and Markdown hardening

Macaca SHALL load agent profile Markdown with root confinement, byte caps, optional leading YAML frontmatter stripping, and basic content scanning.

#### Scenario: Heartbeat file is omitted when disabled

- **GIVEN** `AgentProfileContextConfig.inject_heartbeat` is `false`
- **AND** `HEARTBEAT.md` exists on disk
- **WHEN** `ProfileFileContextProvider` contributes
- **THEN** no profile candidate SHALL be emitted for `HEARTBEAT.md`

#### Scenario: Well-formed frontmatter is removed from body

- **GIVEN** a profile file begins with a `---` delimited YAML block closed by `---`
- **WHEN** the profile loader reads the file
- **THEN** the composed profile candidate body SHALL exclude that frontmatter block

---

### Requirement: MCP capability context SHALL remain metadata-only by default

Macaca SHALL surface MCP capabilities as compact, fenced, untrusted summaries without embedding resource or prompt bodies in capability index context.

#### Scenario: Capability provider stays metadata-first

- **GIVEN** MCP servers are registered
- **WHEN** `McpContextProvider` contributes
- **THEN** emitted context SHALL use the compact catalog representation
- **AND** trust SHALL remain explicitly untrusted for the capability index block

---

### Requirement: External context transports SHALL validate opaque payloads before candidacy

Macaca SHALL treat external/remote context bytes as untrusted and validate structural limits before mapping to `ContextCandidate`.

#### Scenario: Oversized external payload is rejected

- **GIVEN** an `OpaqueExternalPayload` exceeds configured maxima
- **WHEN** `validate_opaque_external_payload` runs
- **THEN** validation SHALL return a structured `ContextDecisionReport` error
- **AND** no `ContextCandidate` SHALL be constructed from the payload without remediation
