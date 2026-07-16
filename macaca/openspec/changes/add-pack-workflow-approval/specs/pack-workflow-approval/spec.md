## ADDED Requirements

### Requirement: Macaca SHALL provide the Workflow Approval Pack as a serviceized capability

Macaca SHALL provide `pack.workflow.approval.v1` as a provider-neutral industrial pack for approval request, decision capture, policy binding, escalation, and evidence replay. The pack SHALL be declared by applications, resolved by admission/catalog services, and invoked only through typed service commands.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.workflow.approval.v1` as required and approval service provider is registered, healthy, entitled, and policy-admissible
- **THEN** admission SHALL expose `pack.workflow.approval.v1` in the effective capability set with command schemas, permission scopes, policy template, health, and replay metadata
- **AND** SDK discovery SHALL mark callable commands as available without exposing provider secrets or raw provider payloads

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.workflow.approval.v1` as required but provider, permission, entitlement, resource, or host support is absent
- **THEN** admission SHALL block readiness with structured unavailable or denied diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back, or fake success

#### Scenario: Optional declaration is unavailable
- **WHEN** an application declares `pack.workflow.approval.v1` as optional and the pack is unavailable
- **THEN** admission SHALL produce an explicit degraded effective capability report
- **AND** SDK command helpers SHALL refuse to build callable service calls for unavailable commands

### Requirement: Workflow Approval Pack commands SHALL use typed canonical service calls

Every `pack.workflow.approval.v1` operation SHALL be represented as a typed command/result DTO and SHALL traverse the canonical service runtime path with trace, policy, resource, entitlement, approval, health, snapshot, and structured error behavior.

#### Scenario: Command succeeds through service runtime
- **WHEN** a declared and policy-allowed command such as `approval.request_approval` is invoked
- **THEN** Macaca SHALL route the command through SDK/facade helpers into the service runtime and approval service provider
- **AND** it SHALL emit sanitized admission, policy, service-call, result, and replay events with stable trace identifiers

#### Scenario: Command is denied before side effects
- **WHEN** policy, permission, entitlement, approval, or resource checks reject a `pack.workflow.approval.v1` command
- **THEN** Macaca SHALL return a typed denied or quota result before invoking the concrete provider
- **AND** the audit trail SHALL include the bounded reason code without raw user data or provider payloads

#### Scenario: Command is unsupported by the active provider
- **WHEN** a descriptor exists but the active provider does not support a requested command
- **THEN** Macaca SHALL return a typed unsupported result with descriptor and provider capability diagnostics
- **AND** SDK discovery SHALL report the command as non-callable for the current effective capability set

### Requirement: Workflow Approval Pack SHALL expose concrete industrial metadata

`pack.workflow.approval.v1` SHALL expose descriptor metadata for command schemas, permission scopes, policy templates, resource budgets, SDK examples, lifecycle state, compatibility, health probes, snapshots, and unavailable diagnostics.

#### Scenario: Developer inspects the pack
- **WHEN** SDK discovery inspects `pack.workflow.approval.v1`
- **THEN** it SHALL return the command namespace `approval.*`, supported commands, permissions, policy templates, examples, lifecycle, availability, health, diagnostics, and compatibility metadata
- **AND** examples SHALL use generic handles or synthetic data rather than application-specific workflows

#### Scenario: Snapshot is recorded
- **WHEN** the service runtime records a `pack.workflow.approval.v1` snapshot
- **THEN** the snapshot SHALL include descriptor version, provider class, health, command availability, policy template hash, resource counters, and sanitized replay pointers
- **AND** it SHALL exclude raw secrets, credentials, prompts, manifests, package bytes, private keys, signatures, raw provider payloads, and unbounded output

### Requirement: Workflow Approval Pack implementation SHALL preserve Macaca boundaries

The `pack.workflow.approval.v1` implementation SHALL remain owned by approval service provider; the microkernel, SDK, shells, and generic application framework SHALL remain provider-neutral and free of application-specific or provider-specific routing branches.

#### Scenario: Boundary gates scan the implementation
- **WHEN** dependency, no-direct-provider-call, and canonical execution-path gates scan the implementation
- **THEN** they SHALL find no concrete provider imports in the microkernel, SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned service registrations and typed service commands

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, mock, or unavailable provider is selected for `pack.workflow.approval.v1`
- **THEN** callers SHALL observe the same provider-neutral command/result contract
- **AND** trace/audit evidence SHALL identify only sanitized provider class and descriptor metadata rather than provider-specific business branches

### Requirement: Workflow Approval Pack SHALL model durable approval lifecycles

`pack.workflow.approval.v1` SHALL model approval requests as durable resources with explicit state transitions, idempotency, eligibility checks, escalation, expiry, cancellation, and decision consumption.

#### Scenario: Approval request is created idempotently
- **WHEN** `approval.request_approval` is invoked twice with the same idempotency key and equivalent request payload
- **THEN** the second call SHALL return the existing `approval_id` and current state
- **AND** the audit trail SHALL record a bounded idempotent-replay event rather than creating a duplicate approval

#### Scenario: Conflicting duplicate request is rejected
- **WHEN** `approval.request_approval` is invoked with an existing idempotency key and a materially different subject, policy, requester, or approver constraint
- **THEN** Macaca SHALL return a typed `conflict` result before provider side effects
- **AND** the conflict event SHALL include only stable hashes and bounded reason codes

#### Scenario: Decision re-checks approver eligibility
- **WHEN** `approval.record_decision` is submitted by an actor whose eligibility was revoked after assignment
- **THEN** Macaca SHALL reject the decision with a typed `denied` result
- **AND** no protected side-effect gate SHALL be produced

#### Scenario: Escalation preserves previous evidence
- **WHEN** a pending approval reaches its escalation rule
- **THEN** Macaca SHALL move the approval to `escalated` or equivalent provider-neutral state with a new eligible approver set
- **AND** previous assignment, deadline, and escalation evidence SHALL remain replayable through sanitized references

#### Scenario: Cancellation races with decision
- **WHEN** cancellation and decision commands target the same pending approval concurrently
- **THEN** exactly one terminal state SHALL be committed
- **AND** the losing command SHALL return a typed `conflict` or `already_terminal` result with replay pointers

### Requirement: Workflow Approval Pack SHALL bind decisions to protected side effects

`pack.workflow.approval.v1` SHALL expose a provider-neutral decision gate that downstream services can verify before sensitive side effects proceed.

#### Scenario: Valid decision gate is consumed
- **WHEN** a downstream service asks to consume an approval decision for the same subject, tenant, application, session, task, trace lineage, and policy template hash
- **THEN** Macaca SHALL return a typed gate success result
- **AND** the decision SHALL be marked consumed when the gate policy is one-time use

#### Scenario: Mismatched protected subject is rejected
- **WHEN** a downstream service attempts to reuse an approval decision for a different protected subject or policy template hash
- **THEN** Macaca SHALL return a typed `denied` or `conflict` result
- **AND** the audit trail SHALL record the mismatch with bounded identifiers and hashes only

#### Scenario: Expired decision cannot be consumed
- **WHEN** a downstream service attempts to consume an approval decision after its expiry or after approver revocation invalidates it
- **THEN** Macaca SHALL reject the gate
- **AND** the protected side effect SHALL NOT be invoked

### Requirement: Workflow Approval Pack SHALL expose policy-filtered approval inspection

`pack.workflow.approval.v1` SHALL expose list and evidence inspection commands that reveal only approvals visible to the caller under policy.

#### Scenario: Pending approvals are listed with policy filtering
- **WHEN** `approval.list_pending` is invoked by a caller with limited visibility
- **THEN** Macaca SHALL return only visible approval summaries with stable pagination cursors
- **AND** hidden approval counts, raw subjects, provider payloads, and sensitive evidence SHALL NOT be leaked

#### Scenario: Evidence inspection returns sanitized references
- **WHEN** `approval.inspect_evidence` is invoked for a visible approval
- **THEN** Macaca SHALL return bounded evidence references, hashes, timestamps, decision metadata, and redacted summaries
- **AND** raw provider payloads, credentials, prompts, manifests, and unbounded comments SHALL NOT be returned
