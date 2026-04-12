## ADDED Requirements

### Requirement: Routes Module Decomposition

The `macaca-web` crate SHALL decompose `routes.rs` into focused modules: `chat_orchestrator.rs` (SSE chat streaming), `loop_manager.rs` (PlanLoop/WorkerLoop lifecycle), `sse.rs` (event conversion/broadcast), and `session.rs` (session CRUD/reconstruction). The `routes.rs` file SHALL contain only route registration and thin handler functions, not exceeding 800 lines.

#### Scenario: Routes file size after decomposition
- **GIVEN** the refactoring is complete
- **WHEN** `routes.rs` line count is measured
- **THEN** it contains fewer than 800 lines
- **AND** all extracted modules are accessible via `pub(crate)` functions

#### Scenario: No behavior change after decomposition
- **GIVEN** the routes are decomposed into separate modules
- **WHEN** the same HTTP requests are made
- **THEN** all API responses and SSE event streams are identical to pre-refactoring behavior
