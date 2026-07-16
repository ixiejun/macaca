# Developer Issue Tracker Pack Design

## Context

`pack.developer.issue.tracker.v1` exposes issue and ticket systems as a Macaca
OS serviceized capability. It lets applications discover, read, create, update,
comment on, label, assign, relate, and transition work items without embedding
GitHub, GitLab, Jira, Linear, provider tokens, query languages, or
application-specific project workflows into generic OS layers.

Issue trackers are notification and workflow systems. A simple state change can
notify users, alter SLA state, trigger automation, or expose sensitive comments.
The pack therefore treats changes as typed plans and requests with provider
field-schema validation, transition validation, notification policy, approval,
redaction, trace/audit evidence, and provider replacement.

## Supplier Capability Matrix

| Supplier/platform | Industrial capability | Macaca abstraction |
| --- | --- | --- |
| GitHub Issues | Issues, labels, milestones, assignees, comments, state, locked state, reactions, events/timeline | Issue item, label, milestone, assignee, comment, state, event timeline, repository/project scope |
| GitLab Issues | Project/group issues, labels, milestones, assignees, notes/discussions, state, links, time tracking, related merge requests | Issue item, note/comment thread, label, milestone, relation, time metadata, provider capability |
| Jira Cloud | Issues, projects, issue types, fields, priorities, comments, attachments, transitions, statuses, watchers, JQL search, workflows | Field schema, issue type, workflow state, transition plan, search profile, attachment handle, watcher metadata |
| Linear GraphQL | Issues, comments, labels, teams, users, workflow states, projects, cycles, relations, attachments, webhooks | Team/project scope, issue item, cycle/sprint, workflow state, relation, webhook/event capability |

The pack exposes provider-neutral contracts. Provider adapters translate to
REST, GraphQL, or remote APIs. OS layers must not branch on provider names,
workflow names, issue types, or project-specific fields.

## Goals

- Provide stable pack id `pack.developer.issue.tracker.v1` and command namespace
  `issue_tracker.*`.
- Support provider/project/team discovery, issue schema inspection, issue
  search/list/get, creation planning, creation requests, update planning, update
  requests, comments, labels, assignees, milestones, projects, cycles/sprints,
  workflow states, transition planning, transition requests, relations, attachment
  handles, event/timeline inspection, and provider capability inspection.
- Preserve safety with field-schema validation, transition validation,
  notification policy, user identity handles, attachment redaction, approval
  tokens, quotas, and audit.
- Keep concrete issue tracker providers behind replaceable service providers.
- Require developer documentation at
  `docs/developer-packs/developer/issue-tracker.md`.

## Non-Goals

- Do not implement concrete GitHub, GitLab, Jira, Linear, OAuth, webhook, or
  provider clients in this proposal.
- Do not define application-specific support, agile, release, incident, product,
  sprint, SLA, approval, or escalation workflows.
- Do not execute repository, CI, chat, email, or notification workflows; those
  belong to separate packs/services and may be linked by handles.
- Do not expose raw credentials, access tokens, private comments, customer data,
  raw attachments, raw provider payloads, prompts, manifests, package bytes,
  private keys, signatures, or unbounded issue history in observability.
- Do not silently transition states, add watchers, notify users, or mutate
  issues without typed request, policy checks, and approval where required.

## Ownership And Boundaries

- Pack id: `pack.developer.issue.tracker.v1`.
- Family: `developer`.
- Backing service owner: issue tracker service provider.
- SDK surface: `sdk.packs.developer.issue.tracker`.
- Command namespace: `issue_tracker.*`.
- Microkernel owns identity, policy facade, resource primitives, service-call
  evidence, trace/audit primitives, and registry metadata only.
- Application framework owns manifest declarations, app-scoped permissions, and
  effective capability projection.
- Runtime host owns provider adapter registration, credential bridge
  composition, network bridges, decorators, and sanitized diagnostics through
  approved composition roots.

## Command Surface

| Command | Purpose | Required behavior |
| --- | --- | --- |
| `issue_tracker.inspect_provider` | Inspect provider/project/team capability | Returns sanitized capability, auth, field, workflow, quota, and health metadata |
| `issue_tracker.list_projects` | List accessible issue projects/spaces/teams | Requires project permission and bounded paging |
| `issue_tracker.inspect_schema` | Inspect issue types, fields, states, transitions, labels, priorities, and relation support | Returns provider-neutral schema and compatibility metadata |
| `issue_tracker.search_issues` | Search/list issues by project, query, state, assignee, label, milestone, cycle, time, or relation | Requires query validation, paging, and redaction |
| `issue_tracker.get_issue` | Inspect one issue/work item | Returns bounded issue details, fields, comments summary, relations, and freshness |
| `issue_tracker.plan_create_issue` | Plan issue creation with type, fields, labels, assignees, project, milestone/cycle, and notification policy | Validates required fields, identities, labels, transitions, quotas, and approvals |
| `issue_tracker.create_issue_request` | Request issue creation from a validated plan | Requires write permission, idempotency key, provider state, and audit |
| `issue_tracker.plan_update_issue` | Plan field/label/assignee/milestone/cycle/project updates | Validates schema, concurrency, notifications, and approvals |
| `issue_tracker.update_issue_request` | Request applying a validated update plan | Requires write permission, version preconditions, and audit |
| `issue_tracker.list_comments` | List bounded issue comments/notes/discussions | Requires comment-read permission and redaction |
| `issue_tracker.create_comment_request` | Request creating a comment | Requires comment permission, notification policy, content redaction, and audit |
| `issue_tracker.update_comment_request` | Request updating/deleting/redacting comments where supported | Requires ownership/policy validation and audit |
| `issue_tracker.plan_transition` | Plan workflow state transition | Validates current state, allowed transitions, required fields, notifications, and approvals |
| `issue_tracker.transition_request` | Request a validated transition | Requires transition permission and audit |
| `issue_tracker.manage_labels` | Add/remove labels or inspect label metadata | Requires label permission and schema validation |
| `issue_tracker.manage_assignees` | Add/remove assignees/watchers/participants where supported | Requires identity validation and notification policy |
| `issue_tracker.manage_relations` | Link/unlink related issues, blockers, duplicates, commits, PRs, or external handles | Requires relation permission and provider support |
| `issue_tracker.get_attachment_handle` | Create/read attachment handle | Requires attachment permission, retention, and redaction |
| `issue_tracker.inspect_timeline` | Inspect events, changes, reactions, transitions, and automation metadata | Returns bounded timeline events |

Every command must define typed command DTOs, typed success results, typed
partial/paged results, validation results, typed denied/unavailable/unsupported/
conflict/stale-version/quota/timeout/cancellation/approval-required/failure
results, redaction profile, idempotency semantics for side effects, and replay
metadata.

## DTO Model

Core DTOs:

- `IssueTrackerScope`: provider scope handle, project/team/space handle,
  credential reference, network policy, permission state, rate limit profile,
  and health.
- `IssueProject`: project/team/space handle, name handle, key handle, visibility,
  issue type support, workflow schema hash, field schema hash, and lifecycle.
- `IssueFieldSchema`: field handle, name handle, type, required flag,
  allowed values, sensitivity class, update semantics, provider mapping hash,
  and validation rules.
- `IssueItem`: issue handle, project handle, issue key handle, title handle,
  body handle, type, state, priority, labels, assignees, reporter handle,
  milestone/cycle/project fields, relation summary, comment summary, version
  hash, freshness, and redaction class.
- `IssueComment`: comment handle, issue handle, author handle, body handle,
  visibility, version hash, created/updated timestamps, reaction summary, and
  redaction class.
- `IssueLabel`: label handle, name handle, color/classification metadata,
  description handle, scope, and archived state.
- `IssueMilestone`: milestone/cycle/sprint handle, name handle, timeframe,
  state, project/team scope, and provider capability hash.
- `IssueWorkflowState`: state handle, name handle, category, terminal flag,
  allowed transitions, required fields, approval policy, and notification class.
- `IssueTransitionPlan`: plan handle, issue handle, from state, to state,
  required field updates, notification policy, required approvals, idempotency
  key, and validation diagnostics.
- `IssueUpdatePlan`: plan handle, issue handle, field changes, label changes,
  assignee changes, relation changes, attachment operations, version
  preconditions, notification policy, approvals, and validation diagnostics.
- `IssueSearchQuery`: project scope, query mode, structured filters, provider
  query handle, pagination, sort, redaction profile, and compatibility hash.
- `IssueRelation`: relation handle, source issue, target handle, relation kind,
  direction, provider support, and redaction class.
- `IssueAttachment`: attachment handle, issue/comment handle, name handle, size
  class, content type, checksum handle, retention, download capability, and
  sensitivity class.
- `IssueTimelineEvent`: event handle, issue handle, event kind, actor handle,
  timestamp, changed fields, automation flag, provider metadata hash, and
  redaction class.
- `IssueTrackerProviderCapability`: provider kind, issue model, field schema
  support, workflow transition support, search support, comment support,
  attachment support, relation support, timeline support, webhook support, auth
  modes, rate limits, lifecycle, and health.

Provider-specific extensions may appear only as bounded `adapter_metadata`
behind capability hashes and must not drive OS-layer routing.

## Permission, Policy, Resource, Entitlement, And Approval Model

Permission scopes:

- `issue_tracker.provider.inspect`
- `issue_tracker.project.read`
- `issue_tracker.schema.read`
- `issue_tracker.issue.read`
- `issue_tracker.issue.create`
- `issue_tracker.issue.update`
- `issue_tracker.issue.transition`
- `issue_tracker.comment.read`
- `issue_tracker.comment.write`
- `issue_tracker.label.manage`
- `issue_tracker.assignee.manage`
- `issue_tracker.relation.manage`
- `issue_tracker.attachment.read`
- `issue_tracker.timeline.read`

Policy defaults:

- Every command is scoped to application id, tenant id, session id, task id,
  trace id, provider scope, project/team/space handle, issue handle, and actor
  handle when available.
- Create/update/transition/comment commands require plan/request separation,
  idempotency key, version preconditions where available, field-schema
  validation, notification policy, credential reference, and audit reason.
- Sensitive/private projects, security issues, customer data, external
  notifications, assignee changes, watcher changes, comments, and terminal state
  transitions may require approval.
- Attachments and comments require redaction and bounded output. Raw attachment
  bytes and private comment text must not enter observability.
- Remote operations require network permission, provider quota, rate limits,
  timeout, cancellation, and structured unavailable behavior.

## SDK Discovery And Developer Documentation

SDK discovery returns pack id, family, version, lifecycle, command schemas,
provider/project support, issue model, field schema support, workflow support,
search support, comment support, attachment support, relation support, timeline
support, permission scopes, policy templates, resource limits, approval rules,
provider capability hashes, health, compatibility, diagnostics, examples,
redaction profiles, and documentation links.

The developer guide at `docs/developer-packs/developer/issue-tracker.md` must
cover:

- manifest declaration and optional/required behavior
- provider scopes, projects, issue types, fields, states, transitions, issues,
  comments, labels, assignees, milestones, cycles, relations, attachments,
  timelines, and provider capabilities
- create/update/comment/transition plan and request lifecycle
- field schema validation, version conflicts, notification policy, identity
  handles, network policy, approvals, quotas, unavailable diagnostics, provider
  replacement, trace/audit interpretation, and conformance tests

Examples must use synthetic projects, users, and issue handles. They must not
include provider names, real tokens, private comments, customer data, attachment
bytes, or workflow-specific conventions.

## Trace, Audit, Health, Snapshot, And Replay

Required sanitized events:

- `issue_tracker_pack_declared`
- `issue_tracker_pack_admission_validated`
- `issue_tracker_provider_inspected`
- `issue_tracker_projects_listed`
- `issue_tracker_schema_inspected`
- `issues_searched`
- `issue_inspected`
- `issue_create_planned`
- `issue_create_requested`
- `issue_update_planned`
- `issue_update_requested`
- `issue_comments_listed`
- `issue_comment_created`
- `issue_comment_updated`
- `issue_transition_planned`
- `issue_transition_requested`
- `issue_labels_managed`
- `issue_assignees_managed`
- `issue_relations_managed`
- `issue_attachment_handle_created`
- `issue_timeline_inspected`
- `issue_tracker_pack_policy_decision`
- `issue_tracker_pack_service_call_requested`
- `issue_tracker_pack_service_call_succeeded`
- `issue_tracker_pack_service_call_failed`
- `issue_tracker_pack_unavailable`
- `issue_tracker_pack_snapshot_recorded`

Snapshots include descriptor version, provider capability hashes, project/schema
hashes, workflow schema hashes, command availability, provider health, policy
template hash, resource counters, bounded issue/status summaries, and sanitized
replay pointers. Snapshots must exclude raw credentials, tokens, private
comments, customer data, raw attachments, raw provider payloads, manifests,
package bytes, private keys, signatures, and unbounded issue history.

## Design Patterns

- **Facade**: SDK clients expose discovery and command builders only.
- **Command**: every operation is a typed command/result DTO.
- **Strategy**: issue provider adapters, field validators, state transition
  validators, search translators, notification policy, redaction, and
  unavailable behavior are replaceable.
- **Decorator**: trace, policy, entitlement, resource, approval, metering,
  network policy, credential redaction, comment/attachment redaction, and
  mutation safety wrap service calls.
- **Specification**: admission validates provider scope, project support,
  command availability, permissions, field schema, workflow transition,
  provider state, quota, and compatibility.
- **Observer**: issue changes, comments, transitions, timeline events, health,
  trace, and audit events are subscribable.
- **Memento**: issue version hashes, create/update/transition plans, field schema
  hashes, timeline cursors, snapshots, and replay pointers preserve recovery
  state.
- **Abstract Factory**: concrete issue tracker providers are created only by
  approved runtime-host composition roots.

## Risks And Mitigations

- Risk: issue tracker pack becomes a Jira/GitHub workflow wrapper. Mitigation:
  provider-neutral issue/field/state/transition DTOs and Strategy adapters.
- Risk: comments or attachments leak private data. Mitigation: handles,
  redaction, bounded snippets, and strict observability exclusions.
- Risk: state changes trigger notifications or automations unexpectedly.
  Mitigation: plan/request split, notification policy, approval, and audit.
- Risk: field schemas differ too much across providers. Mitigation: explicit
  field schema DTO, compatibility hashes, validation diagnostics, and provider
  capability metadata.
- Risk: SDK helpers become a second execution path. Mitigation: helpers build
  canonical service commands and never call issue tracker APIs directly.
