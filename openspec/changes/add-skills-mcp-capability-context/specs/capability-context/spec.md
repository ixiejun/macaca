## ADDED Requirements

### Requirement: Macaca SHALL expose skills and MCP as capability context

Macaca SHALL expose skill, MCP, and runtime tool capabilities to models through context providers that emit compact capability candidates.

#### Scenario: Skill snapshot becomes capability context

- **GIVEN** a skill snapshot exists for an agent
- **WHEN** a model request is composed
- **THEN** the skill context provider SHALL emit compact capability candidates for visible skills
- **AND** the candidates SHALL include name, description, location or reference, namespace, trust, and usage discipline

#### Scenario: MCP registry becomes capability context

- **GIVEN** MCP tools, resources, or prompts are available to an agent
- **WHEN** a model request is composed
- **THEN** the MCP context provider SHALL emit compact capability candidates
- **AND** it SHALL include source, namespace, trust, and usage constraints

### Requirement: Capability context SHALL use progressive disclosure

Capability context SHALL expose compact indexes by default and SHALL NOT inject full skill bodies or MCP resource contents unless explicitly selected by policy or on-demand loading.

#### Scenario: Skill body is not injected by default

- **GIVEN** a visible skill has a long `SKILL.md`
- **WHEN** the skill appears in capability context
- **THEN** the default context SHALL include only compact catalog metadata
- **AND** it SHALL NOT include the full `SKILL.md` body

#### Scenario: MCP resource content is not injected by default

- **GIVEN** an MCP server exposes a resource
- **WHEN** capability context is composed
- **THEN** the context SHALL include at most compact resource metadata by default
- **AND** full resource content SHALL require explicit dynamic loading policy

### Requirement: Capability names SHALL be namespaced and deduplicated

Macaca SHALL namespace capabilities and record collisions to avoid confusing models and tool routers.

#### Scenario: Duplicate tool names are namespaced

- **GIVEN** two MCP servers expose a tool with the same local name
- **WHEN** capability context is built
- **THEN** Macaca SHALL preserve unique capability ids through namespace or source-qualified identifiers
- **AND** collision diagnostics SHALL be recorded

#### Scenario: Higher-priority duplicate skill wins

- **GIVEN** skill runtime snapshot resolves duplicate skills according to source precedence
- **WHEN** skill capability context is built
- **THEN** only the resolved visible skill SHALL be emitted as a capability candidate

### Requirement: Skills SHALL depend on capability ids instead of concrete MCP internals

Macaca SHALL allow skills to declare capability dependencies through stable capability identifiers or categories, not through hardcoded MCP transport internals.

#### Scenario: Skill declares browser capability

- **GIVEN** a skill declares dependency on a browser automation capability
- **WHEN** capability context is composed
- **THEN** the dependency SHALL be represented as a capability id or category
- **AND** the skill provider SHALL NOT directly bind to a concrete MCP server implementation

#### Scenario: Missing dependency is reported

- **GIVEN** a visible skill declares a required capability that is unavailable
- **WHEN** capability context is built
- **THEN** the skill capability SHALL be filtered or annotated according to policy
- **AND** the report SHALL include the missing dependency reason

### Requirement: MCP resources and prompts SHALL be untrusted dynamic context by default

MCP resources and prompts SHALL be treated as external dynamic content unless explicitly trusted by policy.

#### Scenario: MCP prompt is fenced

- **GIVEN** an MCP server exposes a prompt template
- **WHEN** it is included in model-visible context
- **THEN** it SHALL be marked dynamic or untrusted unless policy explicitly promotes it
- **AND** it SHALL be fenced from system instructions

#### Scenario: MCP resource cannot bypass context policy

- **GIVEN** an MCP resource returns content
- **WHEN** that content is considered for context injection
- **THEN** it SHALL pass budget, redaction, trust, and report policy
- **AND** it SHALL NOT directly mutate LLM request messages

### Requirement: Capability context SHALL be auditable

Macaca SHALL report selected, skipped, filtered, and collided capabilities in `ContextReport`.

#### Scenario: Report lists capability decisions

- **GIVEN** skill and MCP providers contribute capability candidates
- **WHEN** context composition completes
- **THEN** the context report SHALL include selected/skipped capabilities, source kind, namespace, trust level, estimated size, and decision reason
