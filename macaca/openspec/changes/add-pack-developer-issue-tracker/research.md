# Developer Issue Tracker Pack Research

## Purpose

This note records supplier/API research, Macaca provider-neutral mapping,
explicit non-goals, existing platform inventory, and GitNexus memo evidence for
`pack.developer.issue_tracker.v1`. Issue tracker support must expose issue,
comment, label, assignee, milestone, state, priority, relation, attachment,
workflow transition, query, webhook, and diagnostics through typed service
commands. It must not hardcode agile, release, support, notification,
repository, CI, or provider-native query workflows into OS-layer behavior.

## Source Baseline

- GitHub Issues and timeline REST APIs:
  <https://docs.github.com/rest/issues>
  and <https://docs.github.com/v3/issues/timeline>
- GitLab Issues API:
  <https://docs.gitlab.com/api/issues/>
- Jira Cloud REST API:
  <https://developer.atlassian.com/cloud/jira/platform/rest/v3/intro/>
  and
  <https://developer.atlassian.com/cloud/jira/platform/rest/v3/api-group-issues/>
- Linear API and webhooks:
  <https://linear.app/docs/api-and-webhooks>

## Supplier API Notes

- GitHub Issues contributes issues, comments, labels, assignees, milestones,
  state/locked state, reactions, timeline events, pull-request-as-issue
  behavior, and repository-scoped issue search. Macaca should model issue type
  and cross-resource identity explicitly.
- GitLab contributes project/group issues, labels, milestones, assignees,
  notes, discussions, related merge requests, state, time tracking, movement,
  promotion, and authorization checks. Macaca should model project/group scope
  and related development artifacts without coupling to repository commands.
- Jira Cloud contributes issues, projects, issue types, fields, comments,
  attachments, transitions, statuses, priorities, assignees, watchers, JQL
  search, workflows, and custom-field complexity. Macaca should normalize field
  schemas and workflow transitions without exposing raw JQL as OS semantics.
- Linear contributes GraphQL issues, comments, labels, teams, users, workflow
  states, projects, cycles, relations, attachments, and webhooks. Macaca should
  model webhook signatures and workflow-state capability generically.

## Macaca-Owned Abstractions

`pack.developer.issue_tracker.v1` should define `IssueTrackerProject`,
`IssueRecord`, `IssueFieldSchema`, `IssueComment`, `IssueLabel`,
`IssueAssignee`, `IssueMilestone`, `IssueState`, `IssuePriority`,
`IssueRelation`, `IssueAttachment`, `IssueTransition`, `IssueQuery`,
`IssueWebhook`, and `IssueTrackerProviderCapability`.

The DTOs must carry provider-neutral issue identity, project/team scope, field
schema, state/transition model, labels/milestones/assignees, comments,
attachments, relations to code/CI artifacts, query bounds, webhook cursors,
provider capability hashes, redaction profiles, and replay pointers. Raw
provider payloads, credentials, JQL/GraphQL pass-through, private comments, and
unbounded issue exports are rejected.

## Explicit Non-Goals

- Do not implement concrete GitHub, GitLab, Jira, Linear, notification,
  repository, CI, attachment storage, agile, release, or support providers in
  this research phase.
- Do not define scrum/kanban workflows, release trains, support queues,
  repository automation, CI actions, or application-specific triage behavior in
  OS layers.
- Do not expose raw provider query languages, provider field ids, webhook
  secrets, or provider-specific routing as stable SDK contracts.

## Existing Macaca Platform Inventory

- Generic descriptors, `SystemFacade`, trace-required service calls,
  unavailable/null-object behavior, policy/resource gates, persistence
  snapshots, repository/CI/notification adjacency, and secrets-reference handles
  provide reusable substrate.
- Current evidence does not prove issue-tracker DTOs, providers, SDK helpers,
  WASM ABI, tests, dependency gates, or developer docs.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
