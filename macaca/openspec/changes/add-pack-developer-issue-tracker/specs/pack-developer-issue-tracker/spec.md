## ADDED Requirements

### Requirement: Macaca SHALL provide the Developer Issue Tracker Pack as a serviceized capability

Macaca SHALL provide `pack.developer.issue.tracker.v1` as a provider-neutral industrial issue tracker capability for issue discovery, issue creation, field updates, comments, labels, assignees, milestones, cycles/sprints, workflow states, relations, attachments, provider capability inspection, and bounded event/timeline inspection. The pack SHALL be declared by applications, admitted through catalog and policy services, and invoked only through typed service commands.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.developer.issue.tracker.v1` as required and the issue tracker service provider is registered, healthy, entitled, scoped, and policy-admissible
- **THEN** admission SHALL expose `pack.developer.issue.tracker.v1` in the effective capability set with command schemas, permission scopes, project scope metadata, policy template hash, provider capability hash, health, and replay metadata
- **AND** SDK discovery SHALL mark callable `issue_tracker.*` commands as available without exposing provider secrets, raw provider payloads, private comments, attachment bytes, customer data, or application-specific workflow names

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.developer.issue.tracker.v1` as required but provider, credential reference, project permission, entitlement, resource, network, host support, or policy admission is absent
- **THEN** admission SHALL block readiness with structured unavailable or denied diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back, contact a concrete provider, mutate issues, notify users, or fake success

#### Scenario: Optional declaration degrades explicitly
- **WHEN** an application declares `pack.developer.issue.tracker.v1` as optional and the pack or a sub-capability is unavailable
- **THEN** admission SHALL produce a degraded effective capability memento naming unavailable commands and bounded reason codes
- **AND** SDK command helpers SHALL refuse to build callable service calls for unavailable commands while preserving discoverability and diagnostics

### Requirement: Issue tracker commands SHALL use typed canonical service calls

Every `pack.developer.issue.tracker.v1` operation SHALL be represented as a typed command/result DTO and SHALL traverse the canonical service runtime path with trace, policy, resource, entitlement, approval, health, snapshot, and structured error behavior. SDK, WASM ABI, shell, and application-framework helpers SHALL only build canonical service commands and SHALL NOT construct concrete issue tracker clients or call remote tracker APIs directly.

#### Scenario: Read command succeeds through service runtime
- **WHEN** a declared and policy-allowed command such as `issue_tracker.search_issues`, `issue_tracker.get_issue`, `issue_tracker.list_comments`, or `issue_tracker.inspect_timeline` is invoked
- **THEN** Macaca SHALL route the command through SDK/facade helpers into the service runtime and issue tracker service provider
- **AND** it SHALL emit sanitized admission, policy, service-call, result, and replay events with stable trace identifiers and bounded paging metadata

#### Scenario: Mutating command is planned before request
- **WHEN** an application wants to create, update, comment on, assign, label, relate, attach, or transition an issue
- **THEN** Macaca SHALL require the applicable planning command, validation diagnostics, idempotency key, notification policy, version precondition where available, approval state where required, and then a separate request command before side effects
- **AND** the plan SHALL be replay-addressable and SHALL NOT mutate the provider during planning

#### Scenario: Command is denied before side effects
- **WHEN** policy, permission, entitlement, approval, resource, quota, schema, version, transition, notification, or scope checks reject a `pack.developer.issue.tracker.v1` command
- **THEN** Macaca SHALL return a typed denied, validation, conflict, stale-version, schema-mismatch, transition-denied, approval-required, quota, unavailable, or unsupported result before invoking the concrete provider
- **AND** the audit trail SHALL include only bounded reason codes and sanitized handles

### Requirement: Issue tracker DTOs SHALL model provider-neutral issue tracker concepts

`pack.developer.issue.tracker.v1` SHALL define provider-neutral DTOs for scopes, projects, schemas, issues, comments, labels, milestones, cycles/sprints, workflow states, transitions, updates, search queries, relations, attachments, timeline events, and provider capabilities. Provider-specific fields SHALL be exposed only as bounded `adapter_metadata` guarded by capability hashes and SHALL NOT drive OS-layer routing branches.

#### Scenario: Provider schema is inspected
- **WHEN** `issue_tracker.inspect_schema` is invoked for a project, team, space, or repository scope
- **THEN** Macaca SHALL return provider-neutral `IssueProject`, `IssueFieldSchema`, `IssueLabel`, `IssueMilestone`, `IssueWorkflowState`, relation support, search compatibility, and provider capability metadata
- **AND** it SHALL include stable schema, workflow, field compatibility, and provider capability hashes for validation and replay

#### Scenario: Issue details are returned
- **WHEN** `issue_tracker.get_issue` returns an issue
- **THEN** the result SHALL use `IssueItem`, bounded field handles, state metadata, label handles, assignee handles, milestone/cycle handles, relation summaries, comment summaries, version hash, freshness metadata, and redaction class
- **AND** it SHALL NOT expose raw credentials, raw provider payloads, private comments, customer data, raw attachment bytes, or unbounded issue history

#### Scenario: Provider-specific capability exists
- **WHEN** an active provider supports a concept not present in the canonical DTO model
- **THEN** the provider MAY expose bounded `adapter_metadata` and compatibility diagnostics through `IssueTrackerProviderCapability`
- **AND** the OS, SDK, shell, and generic application framework SHALL NOT branch on provider names, workflow names, or provider-specific fields

### Requirement: Issue creation, updates, comments, and transitions SHALL be planned, requested, version-safe, notification-aware, and auditable

All issue tracker side effects SHALL use plan/request separation, field-schema validation, project scope validation, identity handle validation, provider capability validation, notification policy, idempotency, version preconditions where available, approval gates where required, and sanitized audit.

#### Scenario: Issue creation is requested
- **WHEN** `issue_tracker.plan_create_issue` validates required fields, issue type, project scope, labels, assignees, milestone or cycle, notification policy, quota, and approvals
- **THEN** `issue_tracker.create_issue_request` MAY use the validated plan handle and idempotency key to request issue creation
- **AND** Macaca SHALL record sanitized plan, request, provider capability hash, field schema hash, policy decision, audit reason, result handle, and replay pointer

#### Scenario: Issue update detects stale version
- **WHEN** `issue_tracker.update_issue_request` receives an update plan whose issue version hash or field schema hash no longer matches the provider state
- **THEN** Macaca SHALL return a typed stale-version or schema-mismatch result
- **AND** it SHALL NOT apply partial updates unless the command explicitly declares provider-supported partial semantics and policy allows them

#### Scenario: Workflow transition is requested
- **WHEN** `issue_tracker.plan_transition` validates the current state, target state, required fields, provider transition support, notification policy, approval state, and version preconditions
- **THEN** `issue_tracker.transition_request` MAY request the transition through the service provider
- **AND** terminal transitions, external notifications, or automation-triggering transitions SHALL be approval-gated when policy requires approval

#### Scenario: Comment is created or updated
- **WHEN** `issue_tracker.create_comment_request` or `issue_tracker.update_comment_request` is invoked
- **THEN** Macaca SHALL validate comment permission, visibility, notification policy, redaction class, content bounds, idempotency, and approval requirements
- **AND** traces, audits, snapshots, and SDK diagnostics SHALL use sanitized handles or bounded summaries rather than raw private comment text

### Requirement: Attachments, timelines, identities, and relations SHALL be bounded and policy-controlled

`pack.developer.issue.tracker.v1` SHALL treat attachments, timelines, identities, watchers/participants, assignees, issue relations, and external links as policy-controlled resources with explicit permissions, quotas, redaction, and provider capability checks.

#### Scenario: Attachment handle is requested
- **WHEN** `issue_tracker.get_attachment_handle` is invoked for an issue or comment attachment
- **THEN** Macaca SHALL validate attachment permission, sensitivity, size class, content type, retention, provider capability, network policy, and approval requirements
- **AND** it SHALL return a bounded attachment handle rather than raw attachment bytes in traces, audits, snapshots, examples, or diagnostics

#### Scenario: Timeline is inspected
- **WHEN** `issue_tracker.inspect_timeline` is invoked
- **THEN** Macaca SHALL return bounded `IssueTimelineEvent` records with event kind, actor handle, timestamp, changed-field handles, automation flag, redaction class, and cursor metadata
- **AND** it SHALL enforce event count, page size, query cost, redaction, and replay bounds

#### Scenario: Relations or assignees are changed
- **WHEN** `issue_tracker.manage_relations` or `issue_tracker.manage_assignees` is invoked
- **THEN** Macaca SHALL validate identity handles, issue handles, relation support, notification policy, approval requirements, and provider capability before side effects
- **AND** it SHALL return structured unsupported or denied diagnostics when the active provider or policy does not allow the operation

### Requirement: Issue Tracker Pack SHALL enforce permissions, scopes, resources, entitlements, approvals, and redaction

`pack.developer.issue.tracker.v1` SHALL enforce explicit permission scopes for provider inspection, project reading, schema reading, issue reading, issue creation, issue updates, workflow transitions, comment reading, comment writing, label management, assignee management, relation management, attachment reading, and timeline reading. Every command SHALL carry application id, tenant id, session id, task id, trace id, provider scope, project/team/space handle, and actor handle when available.

#### Scenario: Permission is missing
- **WHEN** an application invokes an `issue_tracker.*` command without the required permission scope
- **THEN** Macaca SHALL return a typed denied result before provider invocation
- **AND** the denied result SHALL identify the missing permission scope using sanitized identifiers

#### Scenario: Resource budget is exceeded
- **WHEN** a search, comment listing, timeline inspection, attachment handle request, or mutation exceeds page size, query cost, payload size, attachment size, timeout, memory, storage, network, provider quota, or snapshot retention budgets
- **THEN** Macaca SHALL return typed quota, timeout, cancellation, or resource-denied diagnostics
- **AND** it SHALL preserve replayable audit evidence without raw provider output

#### Scenario: Sensitive operation requires approval
- **WHEN** policy marks private/security/customer issues, external comments, terminal transitions, assignee/watcher changes, external relations, attachment access/export, or notification-triggering operations as approval-required
- **THEN** Macaca SHALL return an approval-required result until a valid approval token is supplied
- **AND** no issue mutation, comment, transition, notification, relation change, or attachment retrieval SHALL happen before approval

### Requirement: Issue Tracker Pack SHALL expose industrial metadata and developer documentation

`pack.developer.issue.tracker.v1` SHALL expose descriptor metadata for command schemas, permission scopes, policy templates, resource budgets, approval rules, redaction profiles, provider capability hashes, SDK examples, lifecycle state, compatibility, health probes, snapshots, unavailable diagnostics, and documentation links. The implementation SHALL include detailed developer documentation at `docs/developer-packs/developer/issue-tracker.md`.

#### Scenario: Developer inspects the pack
- **WHEN** SDK discovery inspects `pack.developer.issue.tracker.v1`
- **THEN** it SHALL return command namespace `issue_tracker.*`, command schemas, permissions, provider/project support, field schema support, workflow support, search support, comment support, attachment support, relation support, timeline support, examples, lifecycle, availability, health, diagnostics, compatibility metadata, redaction profiles, and documentation link
- **AND** examples SHALL use synthetic projects, users, issue handles, comments, labels, attachments, and timelines rather than provider names, real tokens, customer data, or application-specific workflows

#### Scenario: Developer documentation is complete
- **WHEN** the pack implementation is marked complete
- **THEN** `docs/developer-packs/developer/issue-tracker.md` SHALL document manifest declarations, required versus optional behavior, permissions, provider scopes, project schema, issue types, fields, states, transitions, issues, comments, labels, assignees, milestones, cycles, relations, attachments, timelines, command DTOs, result DTOs, idempotency, pagination, timeout/cancellation, redaction, notification policy, approvals, unavailable diagnostics, provider replacement, trace/audit interpretation, conformance tests, and supplier/API mapping
- **AND** the guide SHALL be linked from SDK discovery metadata and the industrial pack catalog index

### Requirement: Issue Tracker Pack SHALL be traceable, auditable, replayable, and sanitized

`pack.developer.issue.tracker.v1` SHALL emit sanitized trace and audit events for declaration, admission, provider inspection, project listing, schema inspection, issue search, issue inspection, create planning, create request, update planning, update request, comment listing, comment creation, comment update, transition planning, transition request, label management, assignee management, relation management, attachment handle creation, timeline inspection, policy decisions, service-call lifecycle, failures, unavailable states, and snapshots.

#### Scenario: Snapshot is recorded
- **WHEN** the service runtime records a `pack.developer.issue.tracker.v1` snapshot
- **THEN** the snapshot SHALL include descriptor version, provider capability hashes, project/schema hashes, workflow schema hashes, command availability, provider health, policy template hash, resource counters, bounded issue/status summaries, and sanitized replay pointers
- **AND** it SHALL exclude raw credentials, tokens, private comments, customer data, raw attachments, raw provider payloads, prompts, manifests, package bytes, private keys, signatures, and unbounded issue history

#### Scenario: Replay reconstructs command evidence
- **WHEN** replay inspects a past `issue_tracker.*` command
- **THEN** Macaca SHALL reconstruct descriptor version, command DTO hash, policy decision, resource decision, approval state, provider capability hash, plan handle where applicable, result classification, and sanitized provider class metadata
- **AND** replay SHALL NOT require raw provider payloads, private comment text, attachment bytes, credentials, or application-specific workflow code

### Requirement: Issue Tracker implementation SHALL preserve Macaca boundaries

The `pack.developer.issue.tracker.v1` implementation SHALL remain owned by issue tracker service providers and service-runtime contracts. The microkernel, SDK, shells, and generic application framework SHALL remain provider-neutral and free of application-specific, supplier-specific, workflow-specific, or query-language-specific routing branches.

#### Scenario: Boundary gates scan the implementation
- **WHEN** dependency, no-direct-provider-call, canonical execution-path, and serviceization gates scan the implementation
- **THEN** they SHALL find no concrete GitHub, GitLab, Jira, Linear, REST, GraphQL, credential-manager, notification-client, or provider-adapter imports in the microkernel, SDK helpers, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned service registrations and typed service commands

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, mock, or unavailable provider is selected for `pack.developer.issue.tracker.v1`
- **THEN** callers SHALL observe the same provider-neutral command/result contract
- **AND** trace/audit evidence SHALL identify only sanitized provider class, descriptor metadata, capability hashes, and bounded diagnostics rather than provider-specific business branches
