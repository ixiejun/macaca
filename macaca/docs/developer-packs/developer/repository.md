# Developer Repository Pack

`pack.developer.repository.v1` provides provider-neutral repository opening,
inspection, status, refs, history, diffs, staging plans, commit requests, remote
metadata, fetch, pull, push, merge planning, mutation validation, and provider
capability discovery.

The pack never binds a concrete VCS client, CLI, credential manager, or remote
hosting provider in OS-layer code. Applications exchange repository, ref, diff,
mutation-plan, sync-plan, and remote metadata handles.

## Manifest Declaration

```toml
[service_contract]
optional_packs = ["pack.developer.repository.v1"]
```

Unavailable optional declarations report
`developer_repository_provider_not_installed`. Required declarations block
readiness until a descriptor-compatible repository service provider is
installed.

## Permission Scopes

- `repository.local.read`, `repository.local.write`, `repository.status.read`,
  `repository.diff.read`, `repository.history.read`, and `repository.ref.read`.
- `repository.ref.write`, `repository.stage.write`,
  `repository.commit.create`, `repository.remote.read`,
  `repository.remote.fetch`, `repository.remote.push`,
  `repository.remote.metadata`, `repository.mutation.plan`,
  `repository.mutation.validate`, and `repository.provider.inspect`.

## Commands

- `repository.open`, `repository.inspect`, `repository.status`,
  `repository.list_refs`, `repository.inspect_history`, and
  `repository.diff`.
- `repository.stage_changes`, `repository.plan_commit`,
  `repository.create_commit_request`, and `repository.list_remotes`.
- `repository.fetch`, `repository.plan_pull`, `repository.plan_push`,
  `repository.push_request`, `repository.plan_merge`,
  `repository.validate_mutation`, `repository.inspect_remote_metadata`, and
  `repository.inspect_provider`.

## DTOs And Results

Core DTOs include `RepositoryHandle`, `RepositoryRemote`, `RepositoryRef`,
`RepositoryBranch`, `RepositoryTag`, `RepositoryCommit`,
`RepositoryStatusEntry`, `RepositoryDiff`, `RepositoryMutationPlan`,
`RepositorySyncPlan`, and `RepositoryProviderCapability`. Result statuses cover
success, paging, partial results, dry runs, denied, unavailable, unsupported,
conflict, divergence, dirty worktrees, protected refs, quota, timeout,
cancellation, approval required, and provider failure.

## Command DTO Details

Every command wrapper carries a `DeveloperCommandEnvelope`:

- `subject_ref`: repository, ref, branch, tag, commit, remote, diff, mutation
  plan, or sync plan subject.
- `parameters`: reference-only arguments such as `repository_ref`, `ref_ref`,
  `remote_ref`, `diff_ref`, `mutation_plan_ref`, `sync_plan_ref`,
  `object_hash`, and `approval_ref`.
- `cursor` and `page_size`: bounded pagination for refs, history, status
  entries, diffs, remotes, and remote metadata.
- `idempotency_key`: stable key for staging, commit request, fetch, pull plan,
  push plan, push request, merge plan, and mutation validation.

Result envelopes return `status`, optional `data`, optional paged data, and a
trace-safe error. Plan commands are non-mutating; request commands require
policy, resource, entitlement, and approval gates before repository state can
change. Rollback and recovery guidance is carried by mutation and sync plan
refs.

## Supplier/API Mapping

- Git object, ref, branch, tag, commit, status, diff, index, worktree, remote,
  fetch, pull, push, and merge concepts map to repository DTO handles.
- GitHub, GitLab, and Bitbucket repository, branch, commit, compare, protected
  branch, collaborator, permission, and remote metadata concepts map to
  normalized refs, remotes, mutation plans, and sync plans.
- Raw command pass-through, provider-specific pull-request workflows, concrete
  token handling, CI orchestration, and terminal execution are not OS semantics.

## Examples

Inspect repository status:

```json
{
  "subject_ref": "repository:demo",
  "parameters": { "workspace_ref": "workspace:demo" },
  "idempotency_key": "repo-demo-status"
}
```

Plan a protected push without executing it:

```json
{
  "subject_ref": "repository:demo",
  "parameters": {
    "remote_ref": "remote:origin",
    "source_ref": "ref:feature",
    "target_ref": "ref:main"
  },
  "idempotency_key": "repo-demo-push-plan"
}
```

Unavailable diagnostic:

```json
{
  "pack_id": "pack.developer.repository.v1",
  "required": false,
  "reason_code": "optional_pack_unresolved",
  "message": "developer_repository_provider_not_installed"
}
```

## App-Facing Example Matrix

Generic examples cover repository opening, status inspection, ref listing,
history inspection, diff inspection, commit planning, mutation validation, push
planning, push request planning, and remote metadata inspection. All examples
use synthetic repository, workspace, ref, remote, mutation-plan, and sync-plan
refs.

Diagnostic examples cover unavailable provider, missing repository permission,
unsupported VCS, dirty worktree, diverged ref, protected branch, missing
credential reference, network denied, and approval-required outcomes.
Diagnostics must use provider-neutral reason codes and must not include
provider names, credentials, real remotes, private source, raw diffs, tokens, or
repository-specific workflows.

## Provider Conformance

Provider authors must prove descriptor completeness, VCS and remote support,
auth redaction, dirty-state safety, object-id preconditions, protected-ref
checks, mutation validation, sync planning, resource bounds, policy hooks,
sanitized trace/audit events, unavailable behavior, snapshot/replay metadata,
and no credential, private remote URL, raw source, raw diff, token, or provider
payload leakage.

## Trace And Audit

Trace and audit events may include descriptor hashes, repository handle refs,
ref hashes, mutation-plan refs, sync-plan refs, bounded counters, status, and
trace-safe error codes. They must not include credentials, private remote URLs,
raw source, raw diffs, tokens, or provider payloads.

## Provider Replacement

Provider classes are descriptor labels such as `local-vcs`, `remote-vcs`,
`mutation-planner`, `mock`, and `unavailable`. Concrete libraries, CLIs, remote
clients, credential stores, and mutation executors stay behind service adapters.
