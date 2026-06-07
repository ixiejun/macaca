## ADDED Requirements

### Requirement: Macaca SHALL expose scoped Memory service commands

Macaca SHALL expose a Memory Service contract with typed commands for remember, recall/search, prefetch, forget/delete, status, and snapshot. Each command that reads or mutates memory SHALL carry explicit application, session, agent, memory scope, trace, visibility, policy, and limit context.

#### Scenario: Agent-private recall is requested

- **WHEN** a caller requests memory recall for an agent-private scope
- **THEN** the Memory Service SHALL require application id, session id, agent name or id, trace context, and bounded recall limits
- **AND** it SHALL search only the scoped agent-private memory surface
- **AND** it SHALL NOT silently fall back to application-wide or global recall

#### Scenario: Session-shared prefetch is requested

- **WHEN** a caller requests session-shared memory prefetch
- **THEN** the Memory Service SHALL require application id, session id, trace context, visibility, policy, and bounded limits
- **AND** it SHALL return only memory candidates valid for that session-shared scope

### Requirement: Macaca SHALL preserve replaceable vector topology semantics

Macaca SHALL preserve the abstract topology that an application maps to a database-like namespace and an agent maps to a collection-like namespace while allowing the underlying vector backend implementation to be replaced.

#### Scenario: Memory backend reports topology

- **WHEN** a Memory Service snapshot or status request is made
- **THEN** the service SHALL report provider-neutral topology labels for application namespace and agent collection
- **AND** it SHALL NOT hardcode a single vendor name or vendor-specific API into the service contract

### Requirement: Macaca SHALL expose Memory governance status and snapshots

Macaca SHALL expose memory service status and snapshot data that include provider id, capability set, topology labels, health, governance counts, audit ids, and tombstone/promotion state summaries without dumping memory content by default.

#### Scenario: Governance snapshot is requested

- **WHEN** a caller requests a Memory Service snapshot
- **THEN** the service SHALL return deterministic governance and capability metadata
- **AND** memory item body, embedding vector, secret, and raw user content SHALL be omitted by default

### Requirement: Macaca SHALL emit audit-friendly Memory service events and logs

Macaca SHALL emit structured logs and events for memory remember, recall, prefetch, forget, status, snapshot, governance promotion, and tombstone operations.

#### Scenario: Memory recall completes

- **WHEN** a memory recall operation completes
- **THEN** the Memory Service SHALL emit a structured event with operation, scope, trace id, result count, provider id, policy outcome, and sanitized diagnostics
- **AND** the event SHALL NOT include raw recalled content unless a future explicit debug policy permits it

### Requirement: Macaca SHALL keep deprecated Memory compatibility wrappers searchable

Macaca SHALL keep superseded memory runtime and backend access paths present as deprecated wrappers until all consumers are migrated to Memory Service.

#### Scenario: Old Web memory runtime remains during migration

- **WHEN** old Web or framework code still references a direct memory runtime path
- **THEN** the path SHALL remain searchable and marked deprecated
- **AND** new production paths SHALL prefer Memory Service clients and adapters
