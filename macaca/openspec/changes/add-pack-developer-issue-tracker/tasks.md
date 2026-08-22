## 1. Supplier API Research And Scope

- [x] 1.1 Re-read architecture governance, microkernel boundaries, serviceization allowlist, design patterns, OpenSpec rules, and the industrial catalog umbrella proposal before implementation.
- [x] 1.2 Study GitHub Issues REST API for issues, comments, labels, assignees, milestones, state, locked state, reactions, timeline/events, and repository-scoped issue search.
- [x] 1.3 Study GitLab Issues API for project/group issues, labels, milestones, assignees, notes, discussions, links, related merge requests, state, and time tracking.
- [x] 1.4 Study Jira Cloud REST API for issues, projects, issue types, fields, comments, attachments, transitions, statuses, priorities, assignees, watchers, JQL search, and workflows.
- [x] 1.5 Study Linear GraphQL API for issues, comments, labels, teams, users, workflow states, projects, cycles, relations, attachments, and webhooks.
- [x] 1.6 Produce a supplier capability comparison memo mapping GitHub, GitLab, Jira, and Linear concepts into Macaca provider-neutral issue tracker DTOs and commands.
- [x] 1.7 Define explicit non-goals for concrete provider adapters, agile/release/support workflows, notification workflows, repository/CI execution, raw provider query pass-through, and provider-specific routing.
- [x] 1.8 Record GitNexus CRITICAL/HIGH findings as memo only before implementation commits.

## 2. Contract, Descriptor, And DTOs

- [x] 2.1 Define `pack.developer.issue.tracker.v1` descriptor metadata: pack id, family, lifecycle, stability, provider/project support, issue model, field schema support, workflow transition support, search support, comment support, attachment support, relation support, timeline support, webhook support, auth modes, command schemas, permission scopes, policy templates, resource budgets, approval requirements, data-governance class, SDK metadata, documentation link, compatibility, and diagnostics.
- [x] 2.2 Define provider-neutral DTOs for `IssueTrackerScope`, `IssueProject`, `IssueFieldSchema`, `IssueItem`, `IssueComment`, `IssueLabel`, `IssueMilestone`, `IssueWorkflowState`, `IssueTransitionPlan`, `IssueUpdatePlan`, `IssueSearchQuery`, `IssueRelation`, `IssueAttachment`, `IssueTimelineEvent`, and `IssueTrackerProviderCapability`.
- [x] 2.3 Define typed command/result DTOs for `issue_tracker.inspect_provider`, `issue_tracker.list_projects`, `issue_tracker.inspect_schema`, `issue_tracker.search_issues`, `issue_tracker.get_issue`, `issue_tracker.plan_create_issue`, `issue_tracker.create_issue_request`, `issue_tracker.plan_update_issue`, `issue_tracker.update_issue_request`, `issue_tracker.list_comments`, `issue_tracker.create_comment_request`, `issue_tracker.update_comment_request`, `issue_tracker.plan_transition`, `issue_tracker.transition_request`, `issue_tracker.manage_labels`, `issue_tracker.manage_assignees`, `issue_tracker.manage_relations`, `issue_tracker.get_attachment_handle`, and `issue_tracker.inspect_timeline`.
- [x] 2.4 Define typed success, paged result, partial result, validation issue, denied, unavailable, unsupported, conflict, stale-version, schema-mismatch, transition-denied, quota, timeout, cancellation, approval-required, and failure DTOs.
- [x] 2.5 Define stable descriptor hashing, provider capability hashing, project/schema hashing, workflow schema hashing, issue version hashing, field compatibility hashing, search compatibility hashing, transition-plan hashing, update-plan hashing, timeline cursor hashing, attachment handle hashing, and redaction metadata.
- [x] 2.6 Add descriptor and DTO compatibility tests for valid descriptors, rejected invalid descriptors, stable hashes, schema evolution, field schemas, workflow states, transition plans, update plans, search queries, comments, attachments, timelines, redaction profiles, and serde compatibility.

## 3. Admission, Permission, Policy, Resource, Entitlement, And Approval

- [x] 3.1 Implement manifest declaration validation for required and optional `pack.developer.issue.tracker.v1` declarations.
- [x] 3.2 Implement permission validation for `issue_tracker.provider.inspect`, `issue_tracker.project.read`, `issue_tracker.schema.read`, `issue_tracker.issue.read`, `issue_tracker.issue.create`, `issue_tracker.issue.update`, `issue_tracker.issue.transition`, `issue_tracker.comment.read`, `issue_tracker.comment.write`, `issue_tracker.label.manage`, `issue_tracker.assignee.manage`, `issue_tracker.relation.manage`, `issue_tracker.attachment.read`, and `issue_tracker.timeline.read`.
- [x] 3.3 Implement provider/project/team/space/issue/user scope checks for declared projects, private projects, security issues, customer data, identities, labels, milestones, cycles, states, relations, attachments, and denied scopes.
- [x] 3.4 Implement policy checks for field schema validation, required fields, allowed values, version preconditions, transition support, notification policy, identity resolution, comment visibility, attachment sensitivity, relation support, search query compatibility, and output redaction.
- [x] 3.5 Implement resource reservation for issue page size, search query cost, comment count, timeline event count, attachment size, field payload size, provider quota, network transfer, timeout, memory, storage, streaming output, and retained snapshots.
- [x] 3.6 Implement entitlement checks and structured unavailable/denied diagnostics for missing provider, disabled pack, missing credential reference, missing project permission, unsupported field schema, unsupported transition, unsupported comments, unsupported attachments, unsupported relations, disabled network, missing entitlement, provider quota, and host resource denial.
- [x] 3.7 Implement approval behavior for private/security/customer issues, comments visible to external users, terminal state transitions, assignee/watcher changes, relation changes to external handles, attachment access/export, and operations that trigger notifications or automations.
- [x] 3.8 Add tests proving denied, validation, quota, unavailable, conflict, stale-version, schema-mismatch, transition-denied, unsupported, and approval-required paths do not call concrete providers, mutate issues, create comments, transition states, notify users, or expose attachment bytes.

## 4. Service Provider And Runtime Integration

- [x] 4.1 Implement or bind the issue tracker service provider behind the service runtime; do not construct issue tracker providers from SDK, shell, kernel, or application code.
- [x] 4.2 Add a deterministic unavailable provider that returns typed unavailable/unsupported diagnostics and complete discovery metadata.
- [x] 4.3 Add mock provider support for provider inspection, projects, schema, issue search/get, create planning/request, update planning/request, comment listing/creation/update, transition planning/request, label management, assignee management, relation management, attachment handles, timeline inspection, and provider capability inspection.
- [x] 4.4 Add lifecycle, health, snapshot, shutdown, timeout, cancellation, bounded paging, timeline cursors, stale-version diagnostics, and rate-limit diagnostics.
- [x] 4.5 Add Strategy implementations for provider adapters, field validators, state transition validators, search translators, identity resolvers, notification policy, comment redaction, attachment handle providers, timeline readers, and unavailable behavior.
- [x] 4.6 Add side-effect safety support for idempotency keys, provider state validation, issue version preconditions, schema compatibility checks, transition preconditions, approval state, notification policy, and non-mutating plan commands.
- [x] 4.7 Add provider capability reporting for available, degraded, preview, unavailable, unsupported, retired, project-specific, field-schema-specific, workflow-specific, search-limited, comment-limited, attachment-limited, relation-limited, timeline-limited, network-limited, and quota-limited states.

## 5. SDK, WASM ABI, Application Framework, And Examples

- [x] 5.1 Extend SDK discovery for `pack.developer.issue.tracker.v1` with command schemas, provider/project support, issue model, field schema support, workflow support, search support, comment support, attachment support, relation support, timeline support, examples, availability, diagnostics, documentation link, provider class, capability hash, compatibility, and redaction profile.
- [x] 5.2 Extend application admission so required declarations block readiness when unavailable and optional declarations degrade explicitly with effective capability mementos.
- [x] 5.3 Add SDK command helper builders for all `issue_tracker.*` commands; helpers must only build canonical traced service calls and must never construct issue tracker clients, access credentials, call remote APIs, mutate issues, create comments, transition states, read raw attachments, or bypass policy.
- [x] 5.4 Extend WASM/app ABI descriptors so applications can discover issue tracker commands, declare permissions, receive unavailable diagnostics, and submit typed service calls through the canonical execution path.
- [x] 5.5 Add generic app-facing examples for inspecting schema, searching issues, getting an issue, planning issue creation, requesting issue creation, planning update, requesting update, listing comments, creating a comment, planning transition, requesting transition, managing relations, and getting attachment handles.
- [x] 5.6 Add unavailable-provider, missing-project-permission, schema-mismatch, transition-denied, stale-version, comment-redaction, attachment-denied, notification-approval, provider-quota, and network-denied examples that demonstrate diagnostics without provider names, credentials, private comments, customer data, attachment bytes, or workflow-specific conventions.

## 6. Trace, Audit, Replay, Security, And Gates

- [x] 6.1 Emit sanitized declaration, admission, provider-inspection, project-list, schema-inspection, issue-search, issue-get, create-plan, create-request, update-plan, update-request, comment-list, comment-create, comment-update, transition-plan, transition-request, label-management, assignee-management, relation-management, attachment-handle, timeline-inspection, policy, entitlement, resource, approval, health, snapshot, unavailable, and failure events.
- [x] 6.2 Ensure traces, audits, snapshots, SDK diagnostics, and examples exclude raw credentials, tokens, private comments, customer data, raw attachments, raw provider payloads, prompts, manifests, package bytes, private keys, signatures, and unbounded issue history.
- [x] 6.3 Add replay tests proving every `issue_tracker.*` command is trace-addressable through the canonical service path and that snapshots contain enough bounded metadata for recovery diagnostics.
- [x] 6.4 Add dependency gates proving kernel, SDK, shells, and generic application framework do not import concrete GitHub, GitLab, Jira, Linear, GraphQL/REST client wrappers, credential managers, notification clients, or provider adapters.
- [x] 6.5 Add no-direct-provider-call gates proving SDK helpers, WASM ABI handlers, app admission, web, CLI, and frontend paths route through descriptor-owned service commands.
- [x] 6.6 Add boundary tests proving optional provider absence returns structured unavailable diagnostics and never crashes, hangs, silently falls back, mutates issues, creates comments, transitions states, notifies users, retrieves raw attachments, contacts providers, or fakes success.
- [x] 6.7 Run `openspec validate add-pack-developer-issue-tracker --strict`, targeted cargo tests, boundary gates, file-size gates, and audit replay checks before marking implementation complete.

## 7. Developer Documentation

- [x] 7.1 Create `docs/developer-packs/developer/issue-tracker.md` with purpose, capability model, manifest declaration, required versus optional behavior, permissions, provider scopes, projects, issue types, fields, states, transitions, issues, comments, labels, assignees, milestones, cycles, relations, attachments, timelines, create/update/transition lifecycle, unavailable diagnostics, provider replacement, and operational limits.
- [x] 7.2 Document all command DTOs and result DTOs with field-level explanations, idempotency semantics, redaction behavior, pagination/streaming behavior, timeout/cancellation behavior, plan/request behavior, approval behavior, notification behavior, attachment retention behavior, and structured error codes.
- [x] 7.3 Document supplier/API mapping: GitHub Issues, GitLab Issues, Jira Cloud REST API, and Linear GraphQL concepts mapped to Macaca abstractions, including what is intentionally not exposed as OS semantics.
- [x] 7.4 Add generic examples for schema inspection, issue search, issue creation planning/request, update planning/request, comments, transitions, labels, assignees, relations, attachment handles, timeline inspection, and unavailable diagnostics using synthetic issue data only.
- [x] 7.5 Add conformance checklist and test guidance for provider authors: descriptor completeness, field schema validation, workflow transition validation, version conflicts, notification policy, comment redaction, attachment handle safety, relation support, timeline support, resource bounds, policy hooks, trace/audit events, unavailable behavior, snapshot/replay, and redaction.
- [x] 7.6 Cross-link the guide from SDK discovery metadata and the industrial pack catalog index before marking `add-pack-developer-issue-tracker` complete.
