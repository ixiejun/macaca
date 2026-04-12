## ADDED Requirements

### Requirement: AgenticLoop Shared Iteration Logic

The `AgenticLoop` SHALL extract shared iteration logic (LLM call, tool execution, message management) into a reusable `run_iteration()` method. The three public variants (`run`, `run_with_events`, `run_with_pause`) SHALL delegate to `run_iteration()` and add only their variant-specific behavior (event emission, pause checking).

#### Scenario: Code duplication reduced
- **GIVEN** the refactoring is complete
- **WHEN** measuring shared vs unique code in the three run variants
- **THEN** shared logic exists in exactly one place (`run_iteration`)
- **AND** each variant adds fewer than 50 lines of unique logic

#### Scenario: PausableAgenticLoop uses Notify instead of polling
- **GIVEN** the refactoring is complete
- **WHEN** the coordinator loop pauses waiting for a delegate result
- **THEN** it uses `tokio::sync::Notify` to wake
- **AND** no 100ms polling interval exists

#### Scenario: Behavior preserved after refactoring
- **GIVEN** the iteration logic is extracted
- **WHEN** existing tests run
- **THEN** all tests in `macaca-runtime` pass
- **AND** pause/resume behavior works identically
