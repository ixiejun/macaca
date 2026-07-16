# Developer Repository Pack Design

## Context

`pack.developer.repository.v1` exposes repository operations as a Macaca OS
serviceized capability. It lets applications inspect and coordinate repository
state without embedding Git CLI calls, Git libraries, GitHub/GitLab/Bitbucket
SDKs, credentials, or provider-specific repository workflows into generic OS
layers.

Repository operations are side-effectful and security-sensitive. Reading status
can reveal private paths; diffs can contain secrets; commits mutate history;
fetch/pull/push contact remote hosts; force pushes and rebases can rewrite shared
state. The pack therefore models local state, remote state, plans, validations,
requests, approvals, redaction, trace/audit evidence, and replayable snapshots
explicitly.

## Supplier Capability Matrix

| Supplier/platform | Industrial capability | Macaca abstraction |
| --- | --- | --- |
| Git | Object ids, refs, branches, tags, worktrees, index/staging, status, diff, commits, fetch, pull, push, merge, rebase, reset, remotes | Repository handle, ref/branch/tag/commit DTOs, worktree status, diff summary, commit plan, sync plan, mutation request |
| GitHub REST API | Repositories, contents, branches, commits, refs, tags, compare, pulls, checks/statuses, collaborators, branch protection | Remote repository metadata, provider capability, protected branch policy, remote refs, pull request linkage as metadata |
| GitLab API | Projects/repositories, branches, commits, merge requests, repository files, protected branches, pipelines/statuses, permissions | Remote project metadata, protected ref policy, remote status/check metadata, provider capability |
| Bitbucket Cloud REST API | Workspaces, repositories, refs, branches, commits, pull requests, source/contents | Remote workspace/repository metadata, branch/ref metadata, commit metadata, provider capability |

The pack exposes provider-neutral contracts. Provider adapters may use a Git
library, a host CLI, a remote API, or a mock/unavailable implementation, but
callers see the same DTOs and structured results.

## Goals

- Provide stable pack id `pack.developer.repository.v1` and command namespace
  `repository.*`.
- Support repository opening/binding, metadata inspection, status, staging
  inspection, diff, refs, branches, tags, history, commits, remote listing,
  fetch, pull planning, push planning, push request, commit creation request,
  merge/rebase/cherry-pick/revert planning, validation, worktree safety, remote
  metadata inspection, and provider capability inspection.
- Preserve safety with workspace handles, path scopes, credential references,
  network policy, branch protection metadata, dirty-state checks, approvals,
  dry-run/plan commands, rollback/recovery guidance, redaction, and audit.
- Keep concrete VCS and remote hosting providers behind replaceable service
  providers.
- Require developer documentation at
  `docs/developer-packs/developer/repository.md`.

## Non-Goals

- Do not implement concrete Git, GitHub, GitLab, Bitbucket, SSH, credential, or
  hosting-provider adapters in this proposal.
- Do not define application-specific PR, issue, release, code-review, CI,
  deployment, or branching workflows.
- Do not expose raw credentials, access tokens, private remote URLs, full raw
  diffs, raw source content, raw provider payloads, prompts, manifests, package
  bytes, private keys, signatures, or unbounded history in observability.
- Do not silently perform destructive operations such as reset, force push,
  rebase, merge conflict resolution, or branch deletion.
- Do not use repository provider names as OS-layer routing logic.

## Ownership And Boundaries

- Pack id: `pack.developer.repository.v1`.
- Family: `developer`.
- Backing service owner: repository service provider.
- SDK surface: `sdk.packs.developer.repository`.
- Command namespace: `repository.*`.
- Microkernel owns identity, policy facade, resource primitives, service-call
  evidence, trace/audit primitives, and registry metadata only.
- Application framework owns manifest declarations, app-scoped permissions, and
  effective capability projection.
- Runtime host owns provider adapter registration, credential bridge
  composition, host capability bridges, decorators, and sanitized diagnostics
  through approved composition roots.

## Command Surface

| Command | Purpose | Required behavior |
| --- | --- | --- |
| `repository.open` | Bind an existing local or remote-backed repository handle | Validates workspace scope, trust, provider capability, and entitlement |
| `repository.inspect` | Inspect repository metadata, VCS type, roots, current head, remotes, and health | Returns bounded metadata and redacted remote identifiers |
| `repository.status` | Inspect worktree/index state | Returns typed status entries and dirty-state diagnostics |
| `repository.list_refs` | List branches, tags, remotes, and detached head state | Returns bounded refs with protection/tracking metadata |
| `repository.inspect_history` | Inspect commit history by ref/range/path | Returns bounded commit pages and redacted metadata |
| `repository.diff` | Inspect staged/unstaged/ref/range diff | Returns diff summaries, hunk handles, stats, and redaction metadata |
| `repository.stage_changes` | Request staging/unstaging selected changes | Requires write permission, path scope, approval when risky, and audit |
| `repository.plan_commit` | Plan a commit from staged or selected changes | Validates author policy, message policy, signing policy, and dirty state |
| `repository.create_commit_request` | Request commit creation from a validated plan | Requires approval where policy demands and emits mutation audit |
| `repository.list_remotes` | Inspect configured remotes | Returns redacted remote handles and capability metadata |
| `repository.fetch` | Fetch remote refs/tags metadata | Requires network/remote permission, credential reference, quota, and cancellation |
| `repository.plan_pull` | Plan pull/merge/rebase from remote updates | Validates dirty state, conflicts, branch policy, and merge strategy |
| `repository.plan_push` | Plan push request and protected branch impact | Validates upstream, divergence, branch protection, remote permission, and force policy |
| `repository.push_request` | Request pushing refs to remote | Requires validated plan, network permission, approval for protected/force writes, and audit |
| `repository.plan_merge` | Plan merge/cherry-pick/revert/rebase-like operation | Requires conflict prediction, policy, and approval for history rewriting |
| `repository.validate_mutation` | Validate staged changes, commit plan, push plan, or merge plan without mutation | Must not mutate repository state |
| `repository.inspect_remote_metadata` | Inspect remote platform metadata such as default branch, protection, checks, permissions, and linked review objects | Returns bounded provider-neutral metadata |
| `repository.inspect_provider` | Inspect VCS, remote, auth, mutation, protocol, and policy support | Returns sanitized capability metadata |

Every command must define typed command DTOs, typed success results, typed
partial/paged results, validation results, typed denied/unavailable/unsupported/
conflict/diverged/quota/timeout/cancellation/approval-required/failure results,
redaction profile, idempotency semantics for mutations, and replay metadata.

## DTO Model

Core DTOs:

- `RepositoryHandle`: repository id, workspace handle, VCS type, trust state,
  root handle, default branch handle, current head, dirty state, provider
  capability hash, and health.
- `RepositoryRemote`: remote handle, redacted URL handle, provider class,
  fetch/push capability, credential reference, default branch, and permission
  state.
- `RepositoryRef`: ref handle, name handle, ref kind, target object id, tracking
  ref, protection state, upstream status, and last observed timestamp.
- `RepositoryBranch`: branch handle, ref handle, upstream ref, ahead/behind
  counts, protection state, merge base, and divergence status.
- `RepositoryTag`: tag handle, target object id, annotated flag, signature
  metadata handle, and creation metadata.
- `RepositoryCommit`: commit handle, object id, parent object ids, author/committer
  handles, message handle, timestamp, signature state, tree hash, change stats,
  and redaction class.
- `RepositoryStatusEntry`: path handle, status kind, index state, worktree state,
  rename/copy metadata, conflict stage, submodule state, and sensitivity class.
- `RepositoryDiff`: base ref/object, target ref/object, file changes, hunk
  handles, stats, binary/generated markers, secret-risk flags, and redaction
  profile.
- `RepositoryMutationPlan`: plan handle, mutation kind, affected refs, affected
  paths, expected object ids, conflict prediction, branch protection impact,
  required approvals, idempotency key, and rollback/recovery notes.
- `RepositorySyncPlan`: remote handle, source refs, target refs, ahead/behind,
  divergence, protected branch impact, network requirements, credential
  reference, force policy, and validation diagnostics.
- `RepositoryProviderCapability`: VCS types, local operations, remote
  operations, auth modes, protocol support, branch protection metadata,
  signature support, mutation support, max repository size, rate limits,
  lifecycle, and health.

Provider-specific extensions may appear only as bounded `adapter_metadata`
behind capability hashes and must not drive OS-layer routing.

## Permission, Policy, Resource, Entitlement, And Approval Model

Permission scopes:

- `repository.local.read`
- `repository.local.write`
- `repository.status.read`
- `repository.diff.read`
- `repository.history.read`
- `repository.ref.read`
- `repository.ref.write`
- `repository.stage.write`
- `repository.commit.create`
- `repository.remote.read`
- `repository.remote.fetch`
- `repository.remote.push`
- `repository.remote.metadata`
- `repository.mutation.plan`
- `repository.mutation.validate`
- `repository.provider.inspect`

Policy defaults:

- Every command is scoped to application id, tenant id, session id, task id,
  trace id, workspace handle, repository handle, and path/ref scope when
  available.
- Local reads are limited to declared repository roots and denied for excluded
  paths, secrets, credentials, generated artifacts, vendor directories, or
  protected files unless explicitly permitted.
- Remote operations require network permission, remote scope, credential
  reference, resource budget, and sanitized remote diagnostics.
- Mutating commands require explicit write permission, validation, current
  object ids, idempotency key, and audit reason.
- Protected branch writes, force-like pushes, history rewrites, branch/tag
  deletion, conflict resolution, and broad path changes require approval.
- Raw credentials, private remote URLs, raw source, full raw diffs, raw provider
  payloads, and unbounded history are forbidden in observability.

## SDK Discovery And Developer Documentation

SDK discovery returns pack id, family, version, lifecycle, command schemas, VCS
types, local operation support, remote operation support, auth modes, protocol
support, mutation support, branch protection metadata support, permission
scopes, policy templates, resource limits, approval rules, provider capability
hashes, health, compatibility, diagnostics, examples, redaction profiles, and
documentation links.

The developer guide at `docs/developer-packs/developer/repository.md` must
cover:

- manifest declaration and optional/required behavior
- repository handles, workspace/path scopes, trust state, and VCS types
- refs, branches, tags, commits, object ids, status entries, diffs, remotes,
  sync plans, mutation plans, and provider capability DTOs
- fetch/pull/push planning, protected branch diagnostics, divergence, dirty
  state, conflicts, staging, commit creation, and mutation validation
- permission scopes, approvals, credential references, network policy,
  unavailable diagnostics, provider replacement, trace/audit interpretation, and
  conformance tests

Examples must use synthetic repository handles and fake object ids. They must
not include provider names, real remote URLs, credentials, private source code,
business workflows, or repository-specific conventions.

## Trace, Audit, Health, Snapshot, And Replay

Required sanitized events:

- `repository_pack_declared`
- `repository_pack_admission_validated`
- `repository_opened`
- `repository_inspected`
- `repository_status_reported`
- `repository_refs_listed`
- `repository_history_inspected`
- `repository_diff_inspected`
- `repository_changes_staged`
- `repository_commit_planned`
- `repository_commit_requested`
- `repository_remotes_listed`
- `repository_fetched`
- `repository_pull_planned`
- `repository_push_planned`
- `repository_push_requested`
- `repository_merge_planned`
- `repository_mutation_validated`
- `repository_remote_metadata_inspected`
- `repository_provider_inspected`
- `repository_pack_policy_decision`
- `repository_pack_service_call_requested`
- `repository_pack_service_call_succeeded`
- `repository_pack_service_call_failed`
- `repository_pack_unavailable`
- `repository_pack_snapshot_recorded`

Snapshots include descriptor version, provider capability hashes, VCS type,
current head hash, branch/ref summary, dirty-state summary, remote capability
hashes, command availability, provider health, policy template hash, resource
counters, bounded mutation-plan summaries, and sanitized replay pointers.
Snapshots must exclude raw credentials, private remote URLs, raw source, full
raw diffs, raw provider payloads, manifests, package bytes, private keys,
signatures, and unbounded history.

## Design Patterns

- **Facade**: SDK clients expose discovery and command builders only.
- **Command**: every operation is a typed command/result DTO.
- **Strategy**: VCS adapters, remote provider adapters, auth adapters, diff
  providers, mutation validators, sync planners, and unavailable behavior are
  replaceable.
- **Decorator**: trace, policy, entitlement, resource, approval, metering, path
  scope, credential redaction, network policy, and mutation safety wrap service
  calls.
- **Specification**: admission validates repository scope, VCS support, command
  availability, permissions, remote capability, object ids, ref protection, and
  compatibility.
- **Observer**: repository status, mutation requests, remote sync events, health,
  trace, and audit events are subscribable.
- **Memento**: repository snapshots, object ids, mutation plans, sync plans,
  validation records, dirty-state summaries, and replay pointers preserve
  recovery state.
- **Abstract Factory**: concrete providers are created only by approved
  runtime-host composition roots.

## Risks And Mitigations

- Risk: repository pack becomes a Git CLI pass-through. Mitigation: typed DTOs,
  plan/validate/request split, redaction, and provider-neutral semantics.
- Risk: remote operations leak credentials or URLs. Mitigation: credential
  references, redacted remote handles, network policy, and observability
  exclusions.
- Risk: destructive operations mutate shared state. Mitigation: protected branch
  diagnostics, current object-id checks, approval, mutation plans, and audit.
- Risk: provider workflow semantics leak into OS. Mitigation: remote PR/MR/check
  data is metadata only; workflow packs own workflow orchestration.
- Risk: SDK helpers become a second execution path. Mitigation: helpers build
  canonical service commands and never call Git or remote APIs directly.
