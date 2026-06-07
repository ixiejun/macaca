## ADDED Requirements

### Requirement: AppState Field Grouping

The `AppState` struct SHALL organize its fields into semantically grouped sub-structs. The top-level AppState SHALL have no more than 10 direct fields. Related state SHALL be grouped into: `PersistenceState`, `LoopState`, `SessionState`, and `AppConfig`.

#### Scenario: AppState field count reduced
- **GIVEN** the refactoring is complete
- **WHEN** counting direct fields of `AppState`
- **THEN** there are 10 or fewer fields
- **AND** each sub-struct groups 3-6 related fields

#### Scenario: All handlers compile with new access paths
- **GIVEN** field access paths change (e.g., `state.todo_store` to `state.persist.todo_store`)
- **WHEN** `cargo check` runs
- **THEN** compilation succeeds with zero errors
