## ADDED Requirements

### Requirement: Macaca SHALL provide the Workflow Review Pack as a serviceized capability

Macaca SHALL provide `pack.workflow.review.v1` as a provider-neutral industrial pack for review request, finding capture, fix loop, re-review, approval, and terminal-state closure. The pack SHALL be declared by applications, resolved by admission/catalog services, and invoked only through typed service commands.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.workflow.review.v1` as required and review service provider is registered, healthy, entitled, and policy-admissible
- **THEN** admission SHALL expose `pack.workflow.review.v1` in the effective capability set with command schemas, permission scopes, policy template, health, and replay metadata
- **AND** SDK discovery SHALL mark callable commands as available without exposing provider secrets or raw provider payloads

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.workflow.review.v1` as required but provider, permission, entitlement, resource, or host support is absent
- **THEN** admission SHALL block readiness with structured unavailable or denied diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back, or fake success

#### Scenario: Optional declaration is unavailable
- **WHEN** an application declares `pack.workflow.review.v1` as optional and the pack is unavailable
- **THEN** admission SHALL produce an explicit degraded effective capability report
- **AND** SDK command helpers SHALL refuse to build callable service calls for unavailable commands

### Requirement: Workflow Review Pack commands SHALL use typed canonical service calls

Every `pack.workflow.review.v1` operation SHALL be represented as a typed command/result DTO and SHALL traverse the canonical service runtime path with trace, policy, resource, entitlement, approval, health, snapshot, and structured error behavior.

#### Scenario: Command succeeds through service runtime
- **WHEN** a declared and policy-allowed command such as `review.request_review` is invoked
- **THEN** Macaca SHALL route the command through SDK/facade helpers into the service runtime and review service provider
- **AND** it SHALL emit sanitized admission, policy, service-call, result, and replay events with stable trace identifiers

#### Scenario: Command is denied before side effects
- **WHEN** policy, permission, entitlement, approval, or resource checks reject a `pack.workflow.review.v1` command
- **THEN** Macaca SHALL return a typed denied or quota result before invoking the concrete provider
- **AND** the audit trail SHALL include the bounded reason code without raw user data or provider payloads

#### Scenario: Command is unsupported by the active provider
- **WHEN** a descriptor exists but the active provider does not support a requested command
- **THEN** Macaca SHALL return a typed unsupported result with descriptor and provider capability diagnostics
- **AND** SDK discovery SHALL report the command as non-callable for the current effective capability set

### Requirement: Workflow Review Pack SHALL expose concrete industrial metadata

`pack.workflow.review.v1` SHALL expose descriptor metadata for command schemas, permission scopes, policy templates, resource budgets, SDK examples, lifecycle state, compatibility, health probes, snapshots, and unavailable diagnostics.

#### Scenario: Developer inspects the pack
- **WHEN** SDK discovery inspects `pack.workflow.review.v1`
- **THEN** it SHALL return the command namespace `review.*`, supported commands, permissions, policy templates, examples, lifecycle, availability, health, diagnostics, and compatibility metadata
- **AND** examples SHALL use generic handles or synthetic data rather than application-specific workflows

#### Scenario: Snapshot is recorded
- **WHEN** the service runtime records a `pack.workflow.review.v1` snapshot
- **THEN** the snapshot SHALL include descriptor version, provider class, health, command availability, policy template hash, resource counters, and sanitized replay pointers
- **AND** it SHALL exclude raw secrets, credentials, prompts, manifests, package bytes, private keys, signatures, raw provider payloads, and unbounded output

### Requirement: Workflow Review Pack implementation SHALL preserve Macaca boundaries

The `pack.workflow.review.v1` implementation SHALL remain owned by review service provider; the microkernel, SDK, shells, and generic application framework SHALL remain provider-neutral and free of application-specific or provider-specific routing branches.

#### Scenario: Boundary gates scan the implementation
- **WHEN** dependency, no-direct-provider-call, and canonical execution-path gates scan the implementation
- **THEN** they SHALL find no concrete provider imports in the microkernel, SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned service registrations and typed service commands

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, mock, or unavailable provider is selected for `pack.workflow.review.v1`
- **THEN** callers SHALL observe the same provider-neutral command/result contract
- **AND** trace/audit evidence SHALL identify only sanitized provider class and descriptor metadata rather than provider-specific business branches

### Requirement: Workflow Review Pack SHALL model review rounds, findings, and subject revisions

`pack.workflow.review.v1` SHALL represent reviews as durable requests with review rounds, immutable findings, subject revision hashes, remediation requests, and closure gates.

#### Scenario: Review request records subject revision
- **WHEN** `review.request_review` is invoked for a subject reference
- **THEN** Macaca SHALL store the subject revision hash or revision reference in the review request
- **AND** later closure checks SHALL compare the reviewed revision to the current subject revision when supplied by the caller

#### Scenario: Finding is recorded immutably
- **WHEN** `review.record_finding` records a finding for an active review round
- **THEN** Macaca SHALL create an immutable finding record with severity, category, status, bounded summary, evidence reference, and reviewer identity reference
- **AND** raw reviewed artifacts, provider payloads, and unbounded comments SHALL NOT be stored in trace or audit events

#### Scenario: Fix request links findings to remediation evidence
- **WHEN** `review.request_fix` is invoked for one or more open findings
- **THEN** Macaca SHALL create a fix request linked to those finding identifiers and expected evidence type
- **AND** the review SHALL remain blocked when policy marks any linked finding as blocking

#### Scenario: Re-review preserves prior history
- **WHEN** `review.request_rereview` is invoked after fix evidence is submitted
- **THEN** Macaca SHALL create a new review round while preserving previous findings, decisions, and evidence references
- **AND** no previous round SHALL be overwritten or hidden from replay

### Requirement: Workflow Review Pack SHALL enforce review closure gates

`pack.workflow.review.v1` SHALL provide a review closure gate that downstream task or workflow services can verify before marking reviewed work terminal.

#### Scenario: Blocking findings prevent closure
- **WHEN** `review.evaluate_gate` is invoked and unresolved blocking findings remain visible under policy
- **THEN** Macaca SHALL return a blocked gate result listing bounded finding references and reason codes
- **AND** downstream services SHALL NOT mark the reviewed unit as approved or terminal based on that gate

#### Scenario: Approval becomes stale after subject change
- **WHEN** a subject revision hash changes after approval and the policy does not allow carry-forward
- **THEN** Macaca SHALL mark the approval or review outcome as stale for closure purposes
- **AND** it SHALL require a new review round before a successful closure gate

#### Scenario: Dismissal requires policy authority
- **WHEN** `review.dismiss` attempts to dismiss a finding or review outcome
- **THEN** Macaca SHALL verify dismissal permission, reviewer eligibility, dismissal reason, and trace context before changing state
- **AND** unauthorized dismissal SHALL return `denied` before provider side effects

#### Scenario: Concurrent approval and blocking finding are deterministic
- **WHEN** `review.approve` and `review.record_finding` with blocking severity are submitted concurrently for the same review round
- **THEN** Macaca SHALL commit a deterministic ordering and return typed conflict or blocked diagnostics to the losing or invalidated command
- **AND** replay SHALL reproduce the same gate outcome

### Requirement: Workflow Review Pack SHALL expose policy-filtered review inspection

`pack.workflow.review.v1` SHALL let callers inspect only review summaries, findings, and evidence visible under policy.

#### Scenario: Findings are listed with filters
- **WHEN** `review.list_findings` is invoked with severity, status, or round filters
- **THEN** Macaca SHALL return policy-visible finding summaries with stable pagination cursors
- **AND** hidden finding counts, sensitive subject content, raw provider payloads, and unbounded comments SHALL NOT be leaked

#### Scenario: Closed review remains replayable
- **WHEN** `review.close_review` succeeds
- **THEN** Macaca SHALL preserve review request, rounds, findings, outcomes, closure gate evidence, and sanitized audit pointers
- **AND** replay SHALL prove why the review reached its terminal state without requiring provider-specific payloads
