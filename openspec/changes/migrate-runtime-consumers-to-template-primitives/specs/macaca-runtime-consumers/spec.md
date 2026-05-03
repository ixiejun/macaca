## ADDED Requirements

### Requirement: Upper Consumers Use Runtime Template Entrypoints

Upper crates SHALL use non-deprecated runtime template entrypoints instead of deprecated runtime execution wrappers.

#### Scenario: Integration dry-run executes with events

- **WHEN** integration dry-run runs the agentic loop with execution events
- **THEN** it calls `AgenticLoop::execute_with_events` rather than `AgenticLoop::run_with_events`.

### Requirement: Deprecated Runtime Execution APIs Remain Compatibility-Only

Repository consumers SHALL NOT call deprecated runtime execution APIs outside `macaca-runtime` compatibility wrappers.

#### Scenario: Deprecated usage scan

- **WHEN** repository upper consumers are scanned for `run_with_events` or `run_with_pause` calls
- **THEN** no executable upper-crate call sites are found.

### Requirement: Web Resume Signal Isolation

`macaca-web` SHALL use a local, generic resume signal type for coordinator and goal resume messages instead of directly importing `macaca_runtime::agentic_loop::ResumeReason`.

#### Scenario: Goal completion resumes coordinator

- **WHEN** a goal completion notification resumes a waiting coordinator session
- **THEN** `macaca-web` sends its local resume signal through the active session channel while preserving the existing output text semantics.

### Requirement: Runtime Compatibility Types Remain Available

`macaca-runtime` SHALL keep deprecated execution wrappers and `ResumeReason` available for external migration searches and compatibility.

#### Scenario: External compatibility

- **WHEN** external code still imports `macaca_runtime::agentic_loop::ResumeReason`
- **THEN** the type remains available even though repository web code no longer imports it directly.
