## ADDED Requirements

### Requirement: Privileged Side Effects Pass A Fail-Closed Guard Chain

The system SHALL execute every privileged side effect only through a shared
OS-layer `SideEffectGuard` decorator. Guarded effects include process execution,
filesystem read/write, store mutation, external dispatch, and payment/chain
operations. The guard SHALL enforce the canonical order: require trace, then
policy decision, then entitlement/budget, then resource reservation, then execute,
then audit write. The guard SHALL be provider-neutral, carry no
application-specific logic, and be replaceable through injected policy,
entitlement, and resource ports (Strategy pattern).

#### Scenario: Side effect without trace is refused
- **WHEN** a guarded handler is invoked without a valid `TraceContext`
- **THEN** the guard SHALL return a structured `Denied` result before any side
  effect runs
- **AND** it SHALL emit a sanitized audit event recording the refusal

#### Scenario: Policy denial precedes the effect
- **WHEN** the injected policy port denies an operation
- **THEN** the underlying handler SHALL NOT be executed
- **AND** the guard SHALL return a structured `Denied` result with a rule
  identifier and log the denial at a key execution node

### Requirement: Readiness Gates Are Fail-Closed

The system SHALL represent readiness signals that authorize a side effect
(entitlement readiness, package readiness, evidence acceptance, verified terminal
success) by a shared tri-state type gated fail-closed. Only an explicit ready
state SHALL proceed; unknown or not-ready states SHALL return a structured
denied/failure. Deserialization defaults for these signals SHALL be the
non-authorizing state.

#### Scenario: Unknown readiness denies the effect
- **WHEN** a readiness signal is `Unknown` (e.g. a missing field after
  deserialization) at a mutating operation
- **THEN** the operation SHALL be denied rather than allowed
- **AND** the default value for the readiness field SHALL NOT be the authorizing
  state

#### Scenario: Evidence gate defaults closed
- **WHEN** a skill experience evidence gate or verified-terminal-success flag is
  absent from an input document
- **THEN** it SHALL default to rejected/false and block promotion

### Requirement: Code Execution And Filesystem Access Are Guarded And Bounded

The system SHALL run shell/script/process execution and filesystem read/write in
OS services behind the guard chain with a working-directory / path allow-list,
canonicalized paths that reject `..` traversal, a configurable timeout that kills
the child process on expiry, and byte-bounded captured output. Full command lines
and captured output SHALL be sanitized before entering any result, log, or trace.

#### Scenario: Path traversal is rejected
- **WHEN** a guarded filesystem or skill operation targets a path that escapes its
  allowed root (including via a non-existent `..` path)
- **THEN** the operation SHALL return a structured denied result and perform no IO

#### Scenario: Execution timeout reclaims the child
- **WHEN** a guarded process exceeds its configured timeout
- **THEN** the child process SHALL be killed and a structured timeout failure
  returned
- **AND** captured stdout/stderr SHALL be truncated to the configured byte bound
