## ADDED Requirements
### Requirement: Later task completion SHALL record governed Skill outcome telemetry
The system SHALL record bounded Skill usage outcome telemetry through the Skill
service when a task completes after governed Skills were visible to the
executing agent, without treating the outcome counter as proof of optimization
by itself.

#### Scenario: Successful task increments successful task telemetry
- **GIVEN** a session has a cached Skill snapshot for an agent
- **AND** the Skill governance snapshot contains an `Active` record whose name
  is visible in that cached snapshot
- **WHEN** Agent Execution returns a completed result for that session and agent
- **THEN** the Web adapter SHALL send a `SuccessfulTask` usage command through
  the Skill service boundary
- **AND** the command SHALL include only bounded session, agent, trace, skill,
  source, provenance, and evidence identifiers
- **AND** it SHALL NOT copy raw prompts, raw provider payloads, full Skill
  bodies, package bytes, credentials, or application-specific task content

#### Scenario: Missing snapshot does not fail task completion
- **GIVEN** Agent Execution returns a completed result
- **AND** no cached Skill snapshot exists for the session and agent
- **WHEN** the task outcome telemetry adapter runs
- **THEN** it SHALL log a bounded skip reason
- **AND** it SHALL NOT change the Agent Execution result
- **AND** it SHALL NOT infer activation, task success telemetry, or optimization
  from absent snapshot evidence
