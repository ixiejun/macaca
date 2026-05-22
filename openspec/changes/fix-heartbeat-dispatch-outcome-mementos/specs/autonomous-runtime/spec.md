## MODIFIED Requirements
### Requirement: Runtime Host dispatches heartbeat agents and records terminal outcomes
Runtime Host SHALL own the generic strategy that maps accepted heartbeat wakes to
manifest-declared Agent Execution commands. After dispatch finishes, Runtime Host
SHALL report the terminal dispatch outcome to Heartbeat through a typed service
command carrying trace context, run id, terminal state, stable reason code, and
bounded sanitized metadata.

#### Scenario: Agent execution evidence failure marks heartbeat failed
- **WHEN** Runtime Host dispatches a heartbeat agent and Agent Execution returns without required completion evidence
- **THEN** Runtime Host SHALL report the Heartbeat run as `Failed`
- **AND** Heartbeat operations history SHALL expose the failure through sanitized metadata

#### Scenario: Agent execution evidence success marks heartbeat succeeded
- **WHEN** Runtime Host dispatches a heartbeat agent and completion evidence is verified
- **THEN** Runtime Host SHALL report the Heartbeat run as `Succeeded`
- **AND** Heartbeat operations history SHALL expose the verified evidence key without raw execution output
