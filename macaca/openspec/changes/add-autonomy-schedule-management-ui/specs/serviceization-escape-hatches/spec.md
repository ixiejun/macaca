## ADDED Requirements

### Requirement: Schedule-management UI SHALL not use legacy direct scheduler escape hatches

The serviceization escape-hatch gates SHALL reject new production schedule
management code that uses legacy direct scheduler paths instead of the
serviceized Scheduler client boundary.

#### Scenario: New frontend caller uses legacy schedule routes

- **GIVEN** a production frontend file added for autonomy schedule management
- **WHEN** the file calls `/api/apps/{app_id}/schedules` or another legacy direct schedule-management route
- **THEN** the escape-hatch gate SHALL fail with file, line, token, and replacement guidance pointing to `/api/apps/{app_id}/autonomy/*`

#### Scenario: New Web route constructs a direct task scheduler

- **GIVEN** a production Web route added for autonomy schedule management
- **WHEN** the route constructs `macaca_task::TaskScheduler` or a concrete Scheduler provider directly
- **THEN** the escape-hatch gate SHALL fail with file, line, token, and replacement guidance pointing to the focused Scheduler client and serviceized command path

#### Scenario: Approved legacy compatibility route remains isolated

- **GIVEN** the existing legacy schedule compatibility routes remain in the codebase
- **WHEN** the escape-hatch gate scans production sources
- **THEN** it MAY allow the existing compatibility route definitions as named migration debt
- **AND** it SHALL reject new schedule-management UI or serviceized route callers that depend on those compatibility definitions
