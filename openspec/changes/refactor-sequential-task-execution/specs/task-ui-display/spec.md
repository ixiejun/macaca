## ADDED Requirements

### Requirement: Task Panel Sequence Number Display

The frontend Task Board panel SHALL display each task's `sequence_number` as a visible badge (format `#N`) positioned before the task title. Tasks within each agent group SHALL be rendered in ascending `sequence_number` order.

#### Scenario: Tasks displayed with sequence badges in order
- **GIVEN** agent "frontend" has 3 tasks with sequence_number 1, 2, 3
- **WHEN** the TaskBoardModal renders the "frontend" agent group
- **THEN** tasks are displayed top-to-bottom as #1, #2, #3
- **AND** each task card shows a `#N` badge before the title

#### Scenario: Sequence badge styling
- **GIVEN** a task with sequence_number 5
- **WHEN** the TaskCard renders
- **THEN** a monospace `#5` badge is displayed to the left of the task title
- **AND** the badge uses a distinct visual style (muted color, compact size)

### Requirement: Blocked Task Dependency Indicator

When a task has status `Blocked`, the TaskCard SHALL display a visual dependency indicator showing that the task is waiting on upstream dependencies. The indicator SHALL include a lock icon and a textual hint about the blocking reason when available.

#### Scenario: Blocked task shows dependency hint
- **GIVEN** a task with status Blocked and depends_on containing tasks from agent "architect"
- **WHEN** the TaskCard renders
- **THEN** a lock icon is displayed alongside the status badge
- **AND** a text hint such as "Waiting on dependencies" is shown

#### Scenario: Non-blocked task has no dependency indicator
- **GIVEN** a task with status Pending (not Blocked)
- **WHEN** the TaskCard renders
- **THEN** no lock icon or dependency hint is displayed

### Requirement: Priority Badge Replacement

The TaskCard SHALL replace the `PriorityBadge` component with a `SequenceBadge` component. The `PRIORITY_CONFIG` mapping and `PriorityBadge` function SHALL be removed from the codebase.

#### Scenario: No priority badge rendered
- **GIVEN** a task with priority 8 and sequence_number 2
- **WHEN** the TaskCard renders
- **THEN** a `#2` sequence badge is displayed
- **AND** no priority badge (LOW/MED/HIGH/CRIT) is rendered

### Requirement: Backend API Sorted Response

The `list_todos` and `list_agent_todos` API endpoints SHALL return TodoItem arrays sorted by `sequence_number` in ascending order. The frontend SHALL NOT perform client-side sorting on sequence_number.

#### Scenario: API returns sorted todos
- **GIVEN** 3 tasks for agent "backend" with sequence_number [3, 1, 2]
- **WHEN** `GET /api/apps/{app_id}/todos` is called
- **THEN** the response contains tasks ordered by sequence_number: [1, 2, 3]

#### Scenario: Agent-scoped API also sorted
- **GIVEN** agent "frontend" has tasks with sequence_number [2, 1, 3]
- **WHEN** `GET /api/apps/{app_id}/todos/{agent_name}` is called
- **THEN** the response contains tasks in order [1, 2, 3]

### Requirement: TodoItem Type Includes sequence_number

The frontend `TodoItem` TypeScript interface SHALL include a `sequence_number: number` field. The backend `TodoItem` JSON serialization SHALL always include the `sequence_number` field.

#### Scenario: Frontend type includes sequence_number
- **GIVEN** the backend returns a TodoItem with sequence_number 3
- **WHEN** the frontend parses the API response
- **THEN** `todoItem.sequence_number` equals 3
