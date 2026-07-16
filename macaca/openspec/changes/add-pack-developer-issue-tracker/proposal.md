# Change: Add Developer Issue Tracker Pack

## Why

Developers need `pack.developer.issue.tracker.v1` as an industrial issue/ticket
capability for issue discovery, issue creation, field updates, comments,
labels, assignees, milestones, projects, sprints/cycles, links, attachments,
workflow state transitions, search, events, and provider diagnostics. It must
not be a thin wrapper around one issue tracker or an application-specific
project-management workflow.

Issue trackers store product plans, private customer data, security reports,
user identities, comments, attachments, and workflow state. Mutating issues can
notify people, change SLA state, trigger automations, or expose sensitive data.
Macaca must expose issue tracker operations through provider-neutral typed
commands with field schemas, transition validation, permission gates, approval
rules, resource limits, trace/audit events, snapshots, replay, and structured
unavailable diagnostics.

## Research And Supplier/API Baseline

Official references considered for this pack:

- GitHub REST API Issues documentation covers issues, comments, labels,
  assignees, milestones, state, locked state, reactions, timelines/events, and
  repository-scoped issue search. Reference:
  https://docs.github.com/en/rest/issues
- GitLab Issues API covers project/group issues, labels, milestones, assignees,
  state, notes, discussions, award emoji, links, related merge requests, and
  time tracking. Reference: https://docs.gitlab.com/api/issues/
- Jira Cloud REST API covers issues, fields, issue types, comments,
  attachments, transitions, statuses, priorities, assignees, watchers, search
  with JQL, projects, and workflows. Reference:
  https://developer.atlassian.com/cloud/jira/platform/rest/v3/
- Linear GraphQL API covers issues, comments, labels, projects, cycles,
  workflow states, teams, users, relations, attachments, and webhooks. Reference:
  https://developers.linear.app/docs/graphql/working-with-the-graphql-api

Macaca maps these supplier concepts into provider-neutral issue, work item,
field, state, transition, comment, label, milestone, cycle, project, relation,
attachment, search, event, and capability DTOs. Concrete tracker clients,
queries, tokens, automations, and workflow-specific semantics remain behind
replaceable providers.

## What Changes

- Add provider-neutral `pack.developer.issue.tracker.v1` under the `developer`
  family.
- Define command namespace `issue_tracker.*` for:
  - provider/project/space/team discovery
  - issue schema and field metadata inspection
  - issue search/list/get
  - issue creation planning and creation requests
  - issue update planning and update requests
  - comment creation/update/listing
  - label, assignee, milestone, project, cycle/sprint operations
  - workflow transition planning and transition requests
  - issue links/relations and attachment handle operations
  - event/timeline inspection and provider capability inspection
- Define DTOs for tracker scope, issue project, issue type, issue field schema,
  issue/work item, comments, labels, users, milestones, cycles, workflow states,
  transitions, update plans, search queries, relations, attachments, timelines,
  provider capabilities, and diagnostics.
- Define permission scopes, policy defaults, identity/project gates,
  notification/automation awareness, approval rules, entitlement checks,
  structured unavailable behavior, SDK discovery, developer documentation,
  trace/audit events, snapshots, replay, and boundary gates.
- Require detailed developer documentation at
  `docs/developer-packs/developer/issue-tracker.md` before implementation
  completion.

## Impact

- Affected specs: `pack-developer-issue-tracker`,
  `developer-pack-industrial-capability-catalog`, `sdk-system-facade`,
  `service-runtime`, `unified-execution-path`.
- Affected code later: provider-neutral protocol DTOs, pack descriptors,
  admission validators, SDK discovery and command builders, issue tracker
  service provider or unavailable provider, runtime-host provider adapters,
  trace/audit schemas, replay tests, dependency-boundary gates, and developer
  documentation.
- Non-goals: no concrete GitHub/GitLab/Jira/Linear provider implementation in
  this proposal; no application-specific agile/release/support workflow; no
  provider-name routing in OS layers; no raw tokens, private comments,
  attachments, customer data, or provider payloads in observability; no
  SDK/shell/kernel provider construction; no fake success when provider,
  project scope, entitlement, permission, field schema, transition support, or
  host support is absent.
