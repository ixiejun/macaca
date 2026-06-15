## ADDED Requirements

### Requirement: Web And CLI SHALL Preserve Contracts Without Old Helpers

Web and CLI SHALL preserve stable transport response shapes and user-visible shell behavior through facade-backed commands, not through deprecated direct helpers or compatibility-only code paths.

#### Scenario: CLI command uses facade only
- **WHEN** CLI inspects or operates on services, sessions, traces, applications, packages, or approvals
- **THEN** it SHALL call SDK facade commands or focused clients
- **AND** it SHALL NOT import Web internals or old direct helper functions

#### Scenario: Web route preserves response shape through command adapter
- **WHEN** Web maps an HTTP request to a system operation
- **THEN** the route SHALL validate transport scope, construct a typed command, call a facade/client, and map the result back to the stable response shape
- **AND** it SHALL NOT call deprecated direct semantic helpers

## REMOVED Requirements

### Requirement: Macaca SHALL deprecate replaced direct presentation-owned semantic paths

**Reason**: The terminal state must delete replaced direct paths rather than mark them deprecated.

**Migration**: Replace all callers with facade-backed command handlers and remove old helper definitions.
