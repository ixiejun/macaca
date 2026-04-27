## ADDED Requirements

### Requirement: Standard AgentSkills Knowledge Skills

The system SHALL support AgentSkills-compatible knowledge skills as directories containing a `SKILL.md` file with YAML frontmatter and markdown instructions.

#### Scenario: Load a valid standard skill
- **GIVEN** a skill directory contains `SKILL.md` with `name` and `description`
- **WHEN** the skill runtime scans the directory
- **THEN** the skill is loaded as a knowledge skill
- **AND** its prompt-facing catalog entry includes name, description, and location

#### Scenario: Reject invalid skill metadata
- **GIVEN** a `SKILL.md` file is missing a non-empty `name`
- **WHEN** the skill runtime scans the directory
- **THEN** the file is skipped
- **AND** the skip reason is observable in diagnostics or logs

### Requirement: Skill Source Precedence

The system SHALL discover skills from workspace, project/application, user, Macaca central, bundled, and extra configured directories with deterministic precedence.

#### Scenario: Higher-precedence skill overrides same-name skill
- **GIVEN** two skill sources contain a skill with the same `name`
- **WHEN** the skill runtime builds the catalog
- **THEN** the higher-precedence source wins
- **AND** only one catalog entry for that skill name is visible to the agent

#### Scenario: Missing optional source does not fail startup
- **GIVEN** a configured skill source directory does not exist
- **WHEN** the skill runtime builds the catalog
- **THEN** startup continues
- **AND** the missing directory contributes zero skills

### Requirement: Per-Agent Skill Policy

The system SHALL support application-level defaults and per-agent skill visibility policies.

#### Scenario: Agent allowlist limits visible skills
- **GIVEN** an agent has a non-empty skill allowlist
- **WHEN** the agent run is created
- **THEN** only skills named in the allowlist are included in that agent's skill snapshot

#### Scenario: Agent inherits application default policy
- **GIVEN** an application defines default skill policy
- **AND** an agent does not define its own policy
- **WHEN** the agent run is created
- **THEN** the agent uses the application default policy

### Requirement: Skill Metadata Gating

The system SHALL filter skills according to supported metadata gates before prompt injection.

#### Scenario: Missing required binary filters skill
- **GIVEN** a skill requires a binary that is not available
- **WHEN** the skill runtime evaluates eligibility
- **THEN** the skill is excluded from the model prompt
- **AND** the filter reason is `missing_bin`

#### Scenario: Missing required environment filters skill
- **GIVEN** a skill requires an environment variable that is neither present nor configured as a skill secret
- **WHEN** the skill runtime evaluates eligibility
- **THEN** the skill is excluded from the model prompt
- **AND** no secret value is rendered into the prompt

#### Scenario: OpenClaw metadata is accepted
- **GIVEN** a skill uses `metadata.openclaw.requires.bins`
- **AND** no `metadata.macaca` block is present
- **WHEN** the skill runtime parses metadata
- **THEN** the OpenClaw metadata is treated as compatible gating metadata

### Requirement: Skill Snapshot Stability

The system SHALL freeze a per-agent skill snapshot at session or run creation and reuse it for resume, retry, review, and refresh flows.

#### Scenario: Existing session keeps original snapshot
- **GIVEN** an agent session has a skill snapshot
- **AND** the underlying skill files change on disk
- **WHEN** the session resumes
- **THEN** the agent uses the original snapshot
- **AND** the prompt-visible skill catalog remains stable

#### Scenario: New session sees updated skills
- **GIVEN** a skill file changes on disk
- **WHEN** a new session is created
- **THEN** the new session may build a new snapshot containing the updated skill metadata

### Requirement: Skill Catalog Prompt Injection

The system SHALL inject a compatible `<available_skills>` catalog into every traced framework agent prompt.

#### Scenario: Worker agent receives skill catalog
- **GIVEN** an application has visible skills
- **WHEN** a worker agent is constructed through the traced framework entry
- **THEN** the worker system prompt includes `<available_skills>`
- **AND** the catalog includes only skills visible to that worker

#### Scenario: Planner and coordinator receive skill catalog
- **GIVEN** an application has visible skills
- **WHEN** planner or coordinator agents are constructed through traced framework entries
- **THEN** their system prompts include their own per-agent skill catalogs

### Requirement: Progressive Skill Disclosure

The system SHALL expose skill metadata first and require agents to read skill instructions before applying a matching skill.

#### Scenario: Agent reads skill before using it
- **GIVEN** an agent task matches a visible skill description
- **WHEN** the agent decides to use the skill
- **THEN** it reads the skill `SKILL.md` location before applying instructions from that skill

#### Scenario: Relative resources are resolved safely
- **GIVEN** a skill instruction references a relative resource path
- **WHEN** the agent reads that resource
- **THEN** the path is resolved against the skill base directory
- **AND** access outside the skill root is blocked or rejected

### Requirement: Skill Runtime Security

The system SHALL enforce safety boundaries while discovering and exposing skills.

#### Scenario: Symlink escape is rejected
- **GIVEN** a skill candidate resolves outside its configured source root
- **WHEN** discovery evaluates the candidate
- **THEN** the candidate is skipped
- **AND** a `path_escape` reason is recorded

#### Scenario: Oversized skill file is rejected
- **GIVEN** a `SKILL.md` file exceeds the configured maximum size
- **WHEN** discovery evaluates the candidate
- **THEN** the candidate is skipped
- **AND** an `oversized` reason is recorded

### Requirement: Skill Runtime Observability

The system SHALL persist and stream trace events for skill runtime decisions and usage.

#### Scenario: Catalog build is visible
- **WHEN** an agent run builds a skill catalog
- **THEN** the EventLog records a skill catalog event with visible and filtered counts

#### Scenario: Skill usage is visible
- **GIVEN** an agent reads a visible skill `SKILL.md`
- **WHEN** the read occurs through framework tools
- **THEN** the agent trace records a skill usage event
- **AND** refreshing the browser reloads that event from persisted history

### Requirement: Knowledge Skills and Executable Skills Remain Separate

The system SHALL keep AgentSkills knowledge skills separate from YAML executable skill tools.

#### Scenario: YAML executable skill still loads as a tool
- **GIVEN** an application has a YAML executable skill definition
- **WHEN** the application starts
- **THEN** the executable skill is loaded through the executable skill registry
- **AND** it is not treated as a `SKILL.md` knowledge skill

#### Scenario: Knowledge skill is not executed as a tool
- **GIVEN** an application has a `SKILL.md` knowledge skill
- **WHEN** the application starts
- **THEN** the skill appears in the knowledge skill catalog
- **AND** no executable tool is registered solely because the `SKILL.md` exists
