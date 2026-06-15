## ADDED Requirements

### Requirement: Shells SHALL Have No Execution Or Construction Ownership

Web and CLI SHALL only parse input, map DTOs, call SDK/focused clients, render outputs, expose SSE/GenUI/trace/approval/diagnostic surfaces, and subscribe to events. They SHALL NOT construct framework agents, own execution loops, hold provider anchors, parse task/tool semantics, or expose old route wrappers.

#### Scenario: Web shell invokes execution through facade
- **WHEN** Web handles chat, session recovery, delegation, YAML, WASM, or GenUI execution
- **THEN** it SHALL call `SystemFacade` or focused SDK clients
- **AND** it SHALL only adapt HTTP/SSE/DTO concerns and subscribe to trace/service events

#### Scenario: Shell construction path is rejected
- **WHEN** shell production source calls framework construction APIs or stores construction ports
- **THEN** the shell terminal gate SHALL fail with runtime-host/framework service replacement guidance

### Requirement: Shell Local Loop State SHALL Be Service-Owned

Pause, resume, checkpoint identity, loop wakeup, worker-loop handles, scheduler handles, task delegation, and session execution lifecycle SHALL be owned by execution-control/task/agent-execution services. Shells SHALL only expose subscriptions and presentation state.

#### Scenario: Session recovery uses service state
- **WHEN** a session recovers after restart or refresh
- **THEN** recovery SHALL query service snapshots/events through the facade
- **AND** it SHALL NOT depend on shell-owned local loop handles or channels for semantic correctness

## REMOVED Requirements

### Requirement: Macaca SHALL guard deprecated presentation-owned semantic paths

**Reason**: Guarding deprecated paths while retaining definitions is a migration rule. Terminal state requires deletion.

**Migration**: Move callers to facade/service clients, delete the old helpers, and replace the guard with a zero-debt gate.
