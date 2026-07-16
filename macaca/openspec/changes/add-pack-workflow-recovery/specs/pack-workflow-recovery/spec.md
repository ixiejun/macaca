## ADDED Requirements

### Requirement: Macaca SHALL provide the Workflow Recovery Pack as a serviceized capability

Macaca SHALL provide `pack.workflow.recovery.v1` as a provider-neutral industrial pack for checkpoint discovery, failure classification, retry, repair, resume, and replay diagnostics. The pack SHALL be declared by applications, resolved by admission/catalog services, and invoked only through typed service commands.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.workflow.recovery.v1` as required and recovery service provider is registered, healthy, entitled, and policy-admissible
- **THEN** admission SHALL expose `pack.workflow.recovery.v1` in the effective capability set with command schemas, permission scopes, policy template, health, and replay metadata
- **AND** SDK discovery SHALL mark callable commands as available without exposing provider secrets or raw provider payloads

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.workflow.recovery.v1` as required but provider, permission, entitlement, resource, or host support is absent
- **THEN** admission SHALL block readiness with structured unavailable or denied diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back, or fake success

#### Scenario: Optional declaration is unavailable
- **WHEN** an application declares `pack.workflow.recovery.v1` as optional and the pack is unavailable
- **THEN** admission SHALL produce an explicit degraded effective capability report
- **AND** SDK command helpers SHALL refuse to build callable service calls for unavailable commands

### Requirement: Workflow Recovery Pack commands SHALL use typed canonical service calls

Every `pack.workflow.recovery.v1` operation SHALL be represented as a typed command/result DTO and SHALL traverse the canonical service runtime path with trace, policy, resource, entitlement, approval, health, snapshot, and structured error behavior.

#### Scenario: Command succeeds through service runtime
- **WHEN** a declared and policy-allowed command such as `recovery.classify_failure` is invoked
- **THEN** Macaca SHALL route the command through SDK/facade helpers into the service runtime and recovery service provider
- **AND** it SHALL emit sanitized admission, policy, service-call, result, and replay events with stable trace identifiers

#### Scenario: Command is denied before side effects
- **WHEN** policy, permission, entitlement, approval, or resource checks reject a `pack.workflow.recovery.v1` command
- **THEN** Macaca SHALL return a typed denied or quota result before invoking the concrete provider
- **AND** the audit trail SHALL include the bounded reason code without raw user data or provider payloads

#### Scenario: Command is unsupported by the active provider
- **WHEN** a descriptor exists but the active provider does not support a requested command
- **THEN** Macaca SHALL return a typed unsupported result with descriptor and provider capability diagnostics
- **AND** SDK discovery SHALL report the command as non-callable for the current effective capability set

### Requirement: Workflow Recovery Pack SHALL expose concrete industrial metadata

`pack.workflow.recovery.v1` SHALL expose descriptor metadata for command schemas, permission scopes, policy templates, resource budgets, SDK examples, lifecycle state, compatibility, health probes, snapshots, and unavailable diagnostics.

#### Scenario: Developer inspects the pack
- **WHEN** SDK discovery inspects `pack.workflow.recovery.v1`
- **THEN** it SHALL return the command namespace `recovery.*`, supported commands, permissions, policy templates, examples, lifecycle, availability, health, diagnostics, and compatibility metadata
- **AND** examples SHALL use generic handles or synthetic data rather than application-specific workflows

#### Scenario: Snapshot is recorded
- **WHEN** the service runtime records a `pack.workflow.recovery.v1` snapshot
- **THEN** the snapshot SHALL include descriptor version, provider class, health, command availability, policy template hash, resource counters, and sanitized replay pointers
- **AND** it SHALL exclude raw secrets, credentials, prompts, manifests, package bytes, private keys, signatures, raw provider payloads, and unbounded output

### Requirement: Workflow Recovery Pack implementation SHALL preserve Macaca boundaries

The `pack.workflow.recovery.v1` implementation SHALL remain owned by recovery service provider; the microkernel, SDK, shells, and generic application framework SHALL remain provider-neutral and free of application-specific or provider-specific routing branches.

#### Scenario: Boundary gates scan the implementation
- **WHEN** dependency, no-direct-provider-call, and canonical execution-path gates scan the implementation
- **THEN** they SHALL find no concrete provider imports in the microkernel, SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned service registrations and typed service commands

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, mock, or unavailable provider is selected for `pack.workflow.recovery.v1`
- **THEN** callers SHALL observe the same provider-neutral command/result contract
- **AND** trace/audit evidence SHALL identify only sanitized provider class and descriptor metadata rather than provider-specific business branches

### Requirement: Workflow Recovery Pack SHALL classify failures and build recovery plans

`pack.workflow.recovery.v1` SHALL normalize failures into typed classes and build provider-neutral recovery plans from policy, recovery points, retry budgets, and service-owned repair capabilities.

#### Scenario: Transient failure is classified for retry
- **WHEN** `recovery.classify_failure` receives bounded failure evidence indicating a transient provider, network, timeout, or resource condition
- **THEN** Macaca SHALL classify the failure as retryable when policy and retry budget permit
- **AND** the result SHALL include retry policy references, bounded reason codes, and replay pointers without raw provider payloads

#### Scenario: Permanent failure is terminalized
- **WHEN** failure evidence indicates a permanent schema, permission, unsupported command, or non-retryable provider condition
- **THEN** Macaca SHALL return a non-retryable classification and require `recovery.terminalize` or explicit repair policy before further action
- **AND** it SHALL NOT enter an infinite retry loop

#### Scenario: Recovery plan requires valid recovery point
- **WHEN** `recovery.build_plan` is invoked for failed work
- **THEN** Macaca SHALL verify recovery point integrity hash, compatibility version, owner service, trace lineage, and policy permission
- **AND** corrupted or incompatible recovery points SHALL produce typed diagnostics before retry, repair, or resume commands are allowed

#### Scenario: Retry budget exhaustion is explicit
- **WHEN** `recovery.retry` would exceed retry count, time, budget, quota, or backoff policy
- **THEN** Macaca SHALL return `quota_exceeded`, `retry_budget_exhausted`, or terminal diagnostics
- **AND** the failed workload SHALL NOT be silently retried

### Requirement: Workflow Recovery Pack SHALL repair, resume, and compensate through service boundaries

`pack.workflow.recovery.v1` SHALL execute recovery actions only through provider-neutral service commands and declared permissions.

#### Scenario: Repair action is policy gated
- **WHEN** `recovery.repair_state` attempts to modify recoverable state
- **THEN** Macaca SHALL verify repair permission, owner service support, trace lineage, recovery point compatibility, and resource budget before side effects
- **AND** denied repair SHALL NOT call the concrete provider

#### Scenario: Resume uses recovery point and original trace lineage
- **WHEN** `recovery.resume` resumes failed work after restart
- **THEN** Macaca SHALL use a validated recovery point, preserve original application/session/task/tenant identifiers, and create a new linked trace segment
- **AND** replay SHALL connect pre-failure and post-resume events through sanitized pointers

#### Scenario: Compensation reference is preserved
- **WHEN** `recovery.apply_compensation` records or invokes a compensating action
- **THEN** Macaca SHALL preserve ordered compensation references, side-effect approval evidence, and bounded outcome metadata
- **AND** application-specific compensation logic SHALL remain in the owning service or application-defined capability, not in generic OS code

#### Scenario: Terminalization requires evidence
- **WHEN** `recovery.terminalize` marks failed work unrecoverable
- **THEN** Macaca SHALL require policy authority, failure classification, recovery attempts or skipped-action evidence, and bounded terminal reason
- **AND** task/workflow services SHALL receive a structured terminal state instead of ambiguous blocked or reviewing states

### Requirement: Workflow Recovery Pack SHALL export sanitized replay diagnostics

`pack.workflow.recovery.v1` SHALL provide replay exports that explain recovery without leaking sensitive payloads.

#### Scenario: Replay export is sanitized
- **WHEN** `recovery.export_replay` is invoked for visible failed or recovered work
- **THEN** Macaca SHALL return event references, hashes, state transitions, policy decisions, retry counters, recovery point metadata, and bounded error codes
- **AND** raw prompts, secrets, manifests, package bytes, credentials, private keys, provider payloads, raw checkpoints, and unbounded output SHALL NOT be included

#### Scenario: Replay export is policy filtered
- **WHEN** a caller lacks visibility into parts of a recovery chain
- **THEN** Macaca SHALL omit or redact those segments while preserving a coherent explanation of visible recovery decisions
- **AND** hidden segment counts, provider identities, and sensitive artifact metadata SHALL NOT be inferable through pagination or diagnostics
