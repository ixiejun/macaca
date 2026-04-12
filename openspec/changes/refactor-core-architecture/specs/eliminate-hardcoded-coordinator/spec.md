## ADDED Requirements

### Requirement: Dynamic Entry Agent Resolution

The system SHALL resolve the entry agent name dynamically from the application manifest (`entry_agent` field) instead of hardcoding `"coordinator"`. If the manifest does not specify an entry agent, the system SHALL fall back to the first agent that has the `delegate_task` tool capability.

#### Scenario: Custom entry agent from manifest
- **GIVEN** an app manifest with `entry_agent: "orchestrator"`
- **WHEN** a chat request arrives
- **THEN** the system routes to the "orchestrator" agent, not "coordinator"

#### Scenario: Fallback when entry_agent not specified
- **GIVEN** an app manifest without `entry_agent` field
- **WHEN** a chat request arrives
- **THEN** the system selects the first agent with `delegate_task` tool
- **AND** no hardcoded `"coordinator"` string is used

### Requirement: No Hardcoded Agent Names in OS Crates

The `macaca-kernel`, `macaca-task`, `macaca-runtime`, `macaca-proto`, and `macaca-web` crates SHALL NOT contain hardcoded agent name strings (e.g., `"coordinator"`, `"planner"`, `"backend"`, `"frontend"`, `"architect"`). Agent names SHALL always be resolved from configuration or passed as parameters.

#### Scenario: Grep finds no hardcoded agent names
- **GIVEN** the refactoring is complete
- **WHEN** `grep -r '"coordinator"' macaca/crates/` is run
- **THEN** zero matches are found in non-test, non-comment code
