# Developer Issue Tracker Pack

`pack.developer.issue.tracker.v1` provides provider-neutral project listing,
schema inspection, issue search, issue retrieval, issue creation and update
planning, comment operations, workflow transitions, label and assignee
management, relation management, attachment handles, timeline inspection, and
provider capability discovery.

The pack models issue-tracking behavior through typed service commands. It does
not bind concrete SaaS APIs, credentials, notification rules, or repository
workflows in OS-layer code.

## Manifest Declaration

```toml
[service_contract]
optional_packs = ["pack.developer.issue.tracker.v1"]
```

Unavailable optional declarations report
`developer_issue_tracker_provider_not_installed`. Required declarations block
readiness until a descriptor-compatible issue tracker provider is installed.

## Permission Scopes

- `issue_tracker.provider.inspect`, `issue_tracker.project.read`,
  `issue_tracker.schema.read`, and `issue_tracker.issue.read`.
- `issue_tracker.issue.create`, `issue_tracker.issue.update`,
  `issue_tracker.issue.transition`, `issue_tracker.comment.read`,
  `issue_tracker.comment.write`, `issue_tracker.label.manage`,
  `issue_tracker.assignee.manage`, `issue_tracker.relation.manage`,
  `issue_tracker.attachment.read`, and `issue_tracker.timeline.read`.

## Commands

- `issue_tracker.inspect_provider`, `issue_tracker.list_projects`,
  `issue_tracker.inspect_schema`, `issue_tracker.search_issues`, and
  `issue_tracker.get_issue`.
- `issue_tracker.plan_create_issue`, `issue_tracker.create_issue_request`,
  `issue_tracker.plan_update_issue`, and `issue_tracker.update_issue_request`.
- `issue_tracker.list_comments`, `issue_tracker.create_comment_request`,
  `issue_tracker.update_comment_request`, `issue_tracker.plan_transition`,
  `issue_tracker.transition_request`, `issue_tracker.manage_labels`,
  `issue_tracker.manage_assignees`, `issue_tracker.manage_relations`,
  `issue_tracker.get_attachment_handle`, and
  `issue_tracker.inspect_timeline`.

## DTOs And Results

Core DTOs include `IssueTrackerScope`, `IssueProject`, `IssueFieldSchema`,
`IssueItem`, `IssueComment`, `IssueLabel`, `IssueMilestone`,
`IssueWorkflowState`, `IssueTransitionPlan`, `IssueUpdatePlan`,
`IssueSearchQuery`, `IssueRelation`, `IssueAttachment`,
`IssueTimelineEvent`, and `IssueTrackerProviderCapability`. Result statuses
cover success, paging, partial results, denied, unavailable, unsupported,
conflict, stale versions, schema mismatches, transition denial, quota, timeout,
cancellation, approval required, and provider failure.

## Command DTO Details

Every command wrapper carries a `DeveloperCommandEnvelope`:

- `subject_ref`: provider scope, project, schema, issue, comment, transition
  plan, update plan, search query, relation, attachment, or timeline subject.
- `parameters`: reference-only arguments such as `project_ref`, `issue_ref`,
  `schema_ref`, `field_summary_ref`, `comment_ref`, `transition_plan_ref`,
  `update_plan_ref`, `attachment_ref`, and `approval_ref`.
- `cursor` and `page_size`: bounded pagination for issues, comments,
  attachments, relations, and timeline events.
- `idempotency_key`: stable key for create, update, comment, transition,
  label, assignee, and relation mutation requests.

Result envelopes return `status`, optional `data`, optional paged data, and a
trace-safe error. Plan commands are non-mutating; request commands require
policy, version preconditions, notification checks, and approval when visible
side effects may occur.

## Supplier/API Mapping

- GitHub Issues issue, label, milestone, assignee, comment, event, attachment,
  and project metadata concepts map to issue tracker DTOs.
- GitLab Issues project, issue, label, milestone, note, relation, and state
  transition concepts map to normalized projects, issues, comments, relations,
  and timeline events.
- Jira Cloud REST field schema, issue type, transition, comment, attachment,
  and workflow state concepts map to schema, field, transition, and attachment
  refs.
- Linear GraphQL issue, team, workflow state, label, comment, relation, and
  attachment concepts map to the same provider-neutral model.
- Provider-specific workflow names, notification behavior, raw comments,
  attachment bytes, credentials, and customer-specific fields remain adapters.

## Examples

Search issues:

```json
{
  "subject_ref": "project:demo",
  "parameters": { "search_query_ref": "issue-search:open" },
  "cursor": "cursor:start",
  "page_size": 25
}
```

Plan a transition:

```json
{
  "subject_ref": "issue:demo",
  "parameters": {
    "issue_ref": "issue:demo",
    "target_state_ref": "state:review"
  },
  "idempotency_key": "issue-demo-transition-plan"
}
```

Unavailable diagnostic:

```json
{
  "pack_id": "pack.developer.issue.tracker.v1",
  "required": false,
  "reason_code": "optional_pack_unresolved",
  "message": "developer_issue_tracker_provider_not_installed"
}
```

## App-Facing Example Matrix

Generic examples cover schema inspection, issue search, issue retrieval,
creation planning, creation request planning, update planning, update request
planning, comment listing, comment creation, transition planning, transition
request planning, relation management, and attachment-handle retrieval. All
examples use synthetic project, issue, schema, transition, relation, comment,
and attachment refs.

Diagnostic examples cover unavailable provider, missing project permission,
schema mismatch, transition denied, stale version, comment redaction,
attachment denied, notification approval, provider quota, and network denied
outcomes. Diagnostics must use provider-neutral reason codes and must not
include provider names, credentials, private comments, customer data,
attachment bytes, notification payloads, or workflow-specific conventions.

## Provider Conformance

Provider authors must prove descriptor completeness, field schema validation,
workflow transition validation, version conflict handling, notification policy,
comment redaction, attachment handle safety, relation support, timeline
support, resource bounds, policy hooks, sanitized trace/audit events,
unavailable behavior, snapshot/replay metadata, and no raw comments, customer
data, credentials, attachment bytes, notification payloads, or provider payload
leakage.

## Trace And Audit

Trace and audit events may include issue refs, project refs, schema hashes,
transition-plan refs, update-plan refs, attachment handles, timeline refs,
status, and trace-safe error codes. They must not include raw comments,
credentials, private attachment payloads, notifications, or provider payloads.

## Provider Replacement

Provider classes are descriptor labels such as `issue-model`,
`comment-attachment`, `workflow-transition`, `mock`, and `unavailable`.
Concrete issue APIs, comment stores, attachment stores, notification channels,
and workflow engines stay behind service adapters.
