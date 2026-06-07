## ADDED Requirements

### Requirement: Single Source of Truth for Shared Types

The `macaca-proto` crate SHALL be the single source of truth for all shared type definitions. Other crates (especially `macaca-kernel`) SHALL NOT define duplicate types with the same name. Crates that need these types SHALL re-export from `macaca-proto`.

#### Scenario: TaskId defined only in macaca-proto
- **GIVEN** the type consolidation is complete
- **WHEN** searching for `struct TaskId` across all crates
- **THEN** only `macaca-proto/src/types.rs` contains the definition
- **AND** `macaca-kernel` uses `pub use macaca_proto::TaskId`

#### Scenario: DelegatedTask unified definition
- **GIVEN** the type consolidation is complete
- **WHEN** searching for `struct DelegatedTask` across all crates
- **THEN** only `macaca-proto/src/orchestration.rs` contains the definition
- **AND** all fields from both previous definitions are merged into one struct
