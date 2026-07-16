## ADDED Requirements

### Requirement: Macaca SHALL provide the Workflow Delegation Pack as a serviceized capability

Macaca SHALL provide `pack.workflow.delegation.v1` as a provider-neutral industrial pack for agent delegation, role assignment, handoff, capacity, and result collection. The pack SHALL be declared by applications, resolved by admission/catalog services, and invoked only through typed service commands.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.workflow.delegation.v1` as required and delegation service provider is registered, healthy, entitled, and policy-admissible
- **THEN** admission SHALL expose `pack.workflow.delegation.v1` in the effective capability set with command schemas, permission scopes, policy template, health, and replay metadata
- **AND** SDK discovery SHALL mark callable commands as available without exposing provider secrets or raw provider payloads

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.workflow.delegation.v1` as required but provider, permission, entitlement, resource, or host support is absent
- **THEN** admission SHALL block readiness with structured unavailable or denied diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back, or fake success

#### Scenario: Optional declaration is unavailable
- **WHEN** an application declares `pack.workflow.delegation.v1` as optional and the pack is unavailable
- **THEN** admission SHALL produce an explicit degraded effective capability report
- **AND** SDK command helpers SHALL refuse to build callable service calls for unavailable commands

### Requirement: Workflow Delegation Pack commands SHALL use typed canonical service calls

Every `pack.workflow.delegation.v1` operation SHALL be represented as a typed command/result DTO and SHALL traverse the canonical service runtime path with trace, policy, resource, entitlement, approval, health, snapshot, and structured error behavior.

#### Scenario: Command succeeds through service runtime
- **WHEN** a declared and policy-allowed command such as `delegation.delegate` is invoked
- **THEN** Macaca SHALL route the command through SDK/facade helpers into the service runtime and delegation service provider
- **AND** it SHALL emit sanitized admission, policy, service-call, result, and replay events with stable trace identifiers

#### Scenario: Command is denied before side effects
- **WHEN** policy, permission, entitlement, approval, or resource checks reject a `pack.workflow.delegation.v1` command
- **THEN** Macaca SHALL return a typed denied or quota result before invoking the concrete provider
- **AND** the audit trail SHALL include the bounded reason code without raw user data or provider payloads

#### Scenario: Command is unsupported by the active provider
- **WHEN** a descriptor exists but the active provider does not support a requested command
- **THEN** Macaca SHALL return a typed unsupported result with descriptor and provider capability diagnostics
- **AND** SDK discovery SHALL report the command as non-callable for the current effective capability set

### Requirement: Workflow Delegation Pack SHALL expose concrete industrial metadata

`pack.workflow.delegation.v1` SHALL expose descriptor metadata for command schemas, permission scopes, policy templates, resource budgets, SDK examples, lifecycle state, compatibility, health probes, snapshots, and unavailable diagnostics.

#### Scenario: Developer inspects the pack
- **WHEN** SDK discovery inspects `pack.workflow.delegation.v1`
- **THEN** it SHALL return the command namespace `delegation.*`, supported commands, permissions, policy templates, examples, lifecycle, availability, health, diagnostics, and compatibility metadata
- **AND** examples SHALL use generic handles or synthetic data rather than application-specific workflows

#### Scenario: Snapshot is recorded
- **WHEN** the service runtime records a `pack.workflow.delegation.v1` snapshot
- **THEN** the snapshot SHALL include descriptor version, provider class, health, command availability, policy template hash, resource counters, and sanitized replay pointers
- **AND** it SHALL exclude raw secrets, credentials, prompts, manifests, package bytes, private keys, signatures, raw provider payloads, and unbounded output

### Requirement: Workflow Delegation Pack implementation SHALL preserve Macaca boundaries

The `pack.workflow.delegation.v1` implementation SHALL remain owned by delegation service provider; the microkernel, SDK, shells, and generic application framework SHALL remain provider-neutral and free of application-specific or provider-specific routing branches.

#### Scenario: Boundary gates scan the implementation
- **WHEN** dependency, no-direct-provider-call, and canonical execution-path gates scan the implementation
- **THEN** they SHALL find no concrete provider imports in the microkernel, SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned service registrations and typed service commands

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, mock, or unavailable provider is selected for `pack.workflow.delegation.v1`
- **THEN** callers SHALL observe the same provider-neutral command/result contract
- **AND** trace/audit evidence SHALL identify only sanitized provider class and descriptor metadata rather than provider-specific business branches

### Requirement: Workflow Delegation Pack SHALL enforce durable ownership and lease semantics

`pack.workflow.delegation.v1` SHALL represent delegation as durable work placement with atomic claim, lease renewal, expiry, handoff, cancellation, and terminal result collection.

#### Scenario: Delegated work is claimed atomically
- **WHEN** two eligible assignees attempt to accept the same queued delegation concurrently
- **THEN** exactly one claim SHALL become active
- **AND** the losing command SHALL return a typed `conflict` result with replay pointers and no provider payload leakage

#### Scenario: Lease renewal keeps ownership active
- **WHEN** the current owner invokes `delegation.renew_lease` before the renewal deadline and still satisfies policy, entitlement, and capacity checks
- **THEN** Macaca SHALL extend the lease and emit a bounded heartbeat event
- **AND** no other assignee SHALL be able to claim the work during the renewed lease window

#### Scenario: Expired lease returns work to queue
- **WHEN** an active owner fails to renew a lease before expiry
- **THEN** Macaca SHALL transition the delegation to a re-queueable state or configured failure state
- **AND** expiry evidence SHALL include owner identity reference, deadline, and sanitized reason code

#### Scenario: Release preserves partial evidence
- **WHEN** an owner releases delegated work before completion
- **THEN** Macaca SHALL remove active ownership, preserve bounded checkpoint/result references, and make the work eligible for reassignment according to policy
- **AND** raw work payloads and provider diagnostics SHALL NOT enter the audit event

### Requirement: Workflow Delegation Pack SHALL support capacity-aware placement

`pack.workflow.delegation.v1` SHALL expose capacity inspection and placement constraints without hardcoding agent, worker, provider, or application names.

#### Scenario: Capacity snapshot is inspected
- **WHEN** `delegation.inspect_capacity` is invoked
- **THEN** Macaca SHALL return policy-visible capacity counters, capability tags, queue depth bands, and availability state
- **AND** it SHALL exclude hidden assignments, raw prompts, credentials, provider payloads, and unbounded worker diagnostics

#### Scenario: Capacity exhaustion blocks placement
- **WHEN** a delegation request targets capability constraints whose eligible assignees have no available capacity
- **THEN** Macaca SHALL return a typed queued, quota, or unavailable result according to policy
- **AND** it SHALL NOT route to a hardcoded fallback worker or fake success

#### Scenario: Handoff validates new assignee eligibility
- **WHEN** `delegation.handoff` targets a new assignee that lacks declared capability, tenant access, resource budget, or policy permission
- **THEN** Macaca SHALL reject the handoff before side effects
- **AND** the original claim SHALL remain active unless the handoff policy explicitly releases it

### Requirement: Workflow Delegation Pack SHALL collect terminal results with replayable evidence

`pack.workflow.delegation.v1` SHALL normalize completed, failed, cancelled, and expired delegated work into bounded result envelopes.

#### Scenario: Completed delegation returns artifact references
- **WHEN** `delegation.collect_result` is invoked for completed work visible to the caller
- **THEN** Macaca SHALL return outcome, bounded summary, artifact references, checkpoint references, and trace links
- **AND** raw provider payloads and unbounded output SHALL NOT be returned

#### Scenario: Cancellation races with completion
- **WHEN** cancellation and completion are submitted for the same delegation concurrently
- **THEN** exactly one terminal state SHALL be committed
- **AND** the other command SHALL return `already_terminal` or `conflict` with replayable evidence

#### Scenario: Assignment listing is policy filtered
- **WHEN** `delegation.list_assignments` is invoked by a caller with limited visibility
- **THEN** Macaca SHALL return only visible assignments with stable pagination cursors
- **AND** hidden assignment counts, worker identities, and sensitive capacity details SHALL NOT be leaked
