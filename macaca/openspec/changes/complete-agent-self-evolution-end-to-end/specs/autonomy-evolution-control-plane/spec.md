## ADDED Requirements

### Requirement: End-to-end live self-evolution execution

The autonomy evolution runtime SHALL provide a single traced execution bridge that runs live orchestration, target adapter dispatch, and live audit replay without requiring shell-owned manual sequencing.

#### Scenario: Skill package target is executed and audited

- **GIVEN** a live tick accepted for a `SkillPackage` target
- **AND** a governed Skill materialization operator command is supplied
- **WHEN** the runtime executes the end-to-end live self-evolution bridge
- **THEN** it SHALL call the Skill materialization operator through the Skill service boundary
- **AND** it SHALL return target execution evidence
- **AND** it SHALL replay live audit checkpoints for the run.

#### Scenario: OS-code proposal target is evaluated through service boundary

- **GIVEN** a live tick accepted for an `OsCodeProposal` target
- **AND** a governed OS-code proposal command is supplied
- **WHEN** the runtime executes the end-to-end live self-evolution bridge
- **THEN** it SHALL call the OS-code proposal adapter through the autonomy evolution service boundary
- **AND** it SHALL report the proposal decision
- **AND** it SHALL report that default source mutation was not performed.

#### Scenario: Unsupported target fails closed

- **GIVEN** a live tick accepted for a target without an installed target adapter
- **WHEN** the runtime executes the end-to-end live self-evolution bridge
- **THEN** it SHALL return a structured unavailable target outcome
- **AND** it SHALL not mark the end-to-end execution as accepted.
