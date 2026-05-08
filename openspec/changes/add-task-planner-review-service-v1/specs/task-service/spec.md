## ADDED Requirements

### Requirement: Macaca SHALL provide a Task Service facade for planner, worker, review, and resume orchestration

Macaca SHALL expose a Task Service boundary that owns goal decomposition, task claim, review, coordinator resume, task lifecycle snapshots, and task lifecycle events without making `macaca-web` the long-term orchestration owner.

#### Scenario: Task Service receives a goal creation command

- **WHEN** a caller submits a typed goal creation command with application, session, and trace context
- **THEN** the Task Service SHALL record or emit a goal-ready lifecycle event
- **AND** the Task Service SHALL prepare the goal for decomposition through its task orchestration boundary
- **AND** the Task Service SHALL NOT require the caller to construct planner or worker provider state directly

#### Scenario: Task Service can query session-scoped task board state

- **WHEN** a caller requests the task board for an application and session
- **THEN** the Task Service SHALL return only the session-scoped task board view
- **AND** it SHALL preserve current task ordering and compatibility response shape
- **AND** it SHALL NOT silently fall back to application-wide task scanning

### Requirement: Macaca SHALL model task lifecycle operations as typed commands

Macaca SHALL model task orchestration operations as typed commands so goal decomposition, task claim, review, and coordinator resume can be traced and audited consistently.

#### Scenario: Task claim uses a typed command

- **WHEN** a caller requests task claim through the Task Service
- **THEN** the Task Service SHALL accept a typed claim command with application, session, task, and trace context
- **AND** it SHALL emit structured lifecycle events for claim start and completion

#### Scenario: Review uses a typed command

- **WHEN** a caller submits a task review command
- **THEN** the Task Service SHALL accept the review decision, summary, and trace context as a typed command
- **AND** it SHALL emit structured review lifecycle events
- **AND** it SHALL preserve current review semantics for pass, retry, and fail outcomes

#### Scenario: Resume uses a typed command

- **WHEN** a caller requests coordinator resume after goal or review completion
- **THEN** the Task Service SHALL emit a structured resume signal or event
- **AND** it SHALL preserve current coordinator resume behavior

### Requirement: Macaca SHALL emit audit-friendly task lifecycle events and snapshots

Macaca SHALL emit structured events and deterministic snapshots for task lifecycle actions so Web, CLI, and future service consumers can observe and replay task state safely.

#### Scenario: Task lifecycle events are emitted

- **WHEN** the Task Service emits goal-ready, task-claimed, review-needed, review-completed, goal-completed, or coordinator-resume events
- **THEN** each event SHALL include stable identifiers, operation name, session scope, trace context when available, and structured payload
- **AND** the service SHALL log the same key lifecycle node

#### Scenario: Snapshot is deterministic

- **WHEN** a task service snapshot is requested
- **THEN** the snapshot SHALL be deterministic and sorted by stable identifiers
- **AND** it SHALL preserve enough lifecycle state for inspection and future adapter migration

### Requirement: Macaca SHALL keep Web as an adapter instead of a task orchestration owner

Macaca SHALL migrate `macaca-web` toward a thin adapter role for task orchestration, while keeping current user-visible behavior intact during migration.

#### Scenario: Web routes delegate through the task service seam

- **WHEN** a Web route needs planner, worker, review, or resume behavior
- **THEN** the route SHALL translate transport input into typed task service commands or adapter calls
- **AND** the route SHALL NOT define new task semantics
- **AND** current SSE/EventLog delivery SHALL remain behaviorally compatible during migration

#### Scenario: Existing task and resume flows remain visible

- **WHEN** the serviceization change is applied
- **THEN** existing goal -> planner -> task -> worker -> review -> coordinator resume behavior SHALL continue to satisfy Route C regression expectations
- **AND** task board session-scoped fetch SHALL remain intact
- **AND** `/api/chat/v2`, trace, driver, skill/MCP, and resume regression paths SHALL not regress

### Requirement: Macaca SHALL keep deprecated compatibility entry points searchable

Macaca SHALL keep superseded task orchestration entry points in the repository as deprecated wrappers instead of deleting them immediately.

#### Scenario: Deprecated wrappers remain available

- **WHEN** old public task orchestration entry points are still referenced in the codebase
- **THEN** the deprecated wrappers SHALL remain present for migration searchability
- **AND** they SHALL delegate to the new canonical Task Service boundary where applicable

