## MODIFIED Requirements

### Requirement: Runtime Host Heartbeat Lane

The runtime-host autonomy supervisor SHALL register native Heartbeat profiles
from sanitized application declarations through the Heartbeat service boundary.
For manifest-declared heartbeat agents, it SHALL register one profile per valid
enabled agent and SHALL NOT collapse all agents into a single application-level
profile.

#### Scenario: Per-agent profiles are registered
- **GIVEN** an admitted application manifest declares multiple enabled heartbeat agents
- **WHEN** runtime-host starts or registers application heartbeat policy
- **THEN** it creates one Heartbeat profile per valid enabled agent
- **AND** each profile uses an application-agent wake scope key
- **AND** no Scheduler job, Scheduler target, or shell-owned timer is created

### Requirement: Heartbeat Agent Dispatch

Runtime-host SHALL dispatch an accepted per-agent Heartbeat wake only to the
manifest declaration that matches the accepted profile id or wake scope key.
Legacy application-scoped wakes MAY dispatch all enabled declarations for
compatibility.

#### Scenario: One agent profile accepts a wake
- **GIVEN** an application has two heartbeat agent declarations
- **WHEN** Heartbeat accepts the profile wake for one agent
- **THEN** runtime-host dispatches only that agent through Agent Execution
- **AND** the dispatch metadata includes the heartbeat run id, audit id, profile id, and scope key
