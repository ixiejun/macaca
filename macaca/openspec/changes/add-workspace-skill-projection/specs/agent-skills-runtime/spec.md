## ADDED Requirements

### Requirement: Macaca-first skill discovery

The agent skills runtime SHALL scan user-global skill roots in this priority order: `~/.macaca/skills`, `~/.agent/skills`, `~/.claude/skills`, `~/.codex/skills`, `~/.hermes/skills`, and `~/.openclaw/skills`.

#### Scenario: Duplicate skill name exists in multiple user-global roots

- **GIVEN** the same skill name exists in `~/.macaca/skills` and `~/.codex/skills`
- **WHEN** the runtime builds a skill snapshot
- **THEN** the `~/.macaca/skills` copy is selected
- **AND** the lower-priority copy is filtered by precedence before prompt rendering

### Requirement: Workspace-local skill projection

The agent skills runtime SHALL materialize every policy-visible prompt skill into the active workspace under `available_skills/<stable-slug>/` when a workspace directory is available.

#### Scenario: Visible skill has relative scripts

- **GIVEN** a visible skill directory contains `SKILL.md` and `scripts/crypto.py`
- **WHEN** the runtime builds a skill snapshot with a workspace directory
- **THEN** `available_skills/<stable-slug>/SKILL.md` exists inside the workspace
- **AND** `available_skills/<stable-slug>/scripts/crypto.py` exists inside the workspace
- **AND** the prompt `<location>` points at the projected `SKILL.md`

### Requirement: Source path provenance

The agent skills runtime SHALL retain the original source `SKILL.md` path and base directory in each projected snapshot entry.

#### Scenario: Auditing projected skill usage

- **GIVEN** a visible skill is projected into the workspace
- **WHEN** a caller inspects the snapshot entry
- **THEN** the entry exposes the projected `location`
- **AND** the entry exposes the original source `source_location`
- **AND** file policy checks accept files under either the projected base directory or the source base directory
