## ADDED Requirements

### Requirement: Macaca SHALL provide Developer Repository Pack as a serviceized capability

Macaca SHALL provide `pack.developer.repository.v1` as a provider-neutral
industrial pack for repository opening, inspection, status, refs, branches,
tags, commit history, diffs, staging, commit planning, commit requests, remote
listing, fetch, pull planning, push planning, push requests, merge/rebase-like
planning, mutation validation, remote metadata inspection, provider capability
inspection, and unavailable diagnostics. Applications SHALL declare the pack in
manifests, admission SHALL resolve it into effective capabilities, and all
operations SHALL run through typed service commands.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.developer.repository.v1` as required and a repository service provider is registered, healthy, entitled, workspace-compatible, VCS-compatible, remote-compatible where requested, mutation-compatible where requested, quota-compatible, and policy-admissible
- **THEN** admission SHALL expose the pack in the effective capability set with command schemas, VCS support, local operation support, remote operation support, auth modes, protocol support, mutation support, branch protection metadata support, permission scopes, policy templates, resource limits, health, diagnostics, documentation links, and replay metadata
- **AND** SDK discovery SHALL mark callable commands as available without exposing credentials, access tokens, private remote URLs, raw source files, full raw diffs, raw provider payloads, raw manifests, package bytes, private keys, signatures, or unbounded history

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.developer.repository.v1` as required but provider, workspace trust, VCS support, remote support, credential reference, path/ref scope, permission, entitlement, approval, resource budget, network support, or host support is absent
- **THEN** admission SHALL block readiness with structured unavailable, denied, unsupported, validation, dirty-worktree, diverged, protected-ref, approval-required, conflict, quota, timeout, or failure diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back, instantiate another provider implicitly, mutate repository state, contact remote hosts, or fake success

#### Scenario: Optional declaration is unavailable
- **WHEN** an application declares `pack.developer.repository.v1` as optional and the pack is unavailable
- **THEN** admission SHALL produce an explicit degraded effective capability report
- **AND** SDK command helpers SHALL return Null Object unavailable diagnostics instead of creating callable service calls

### Requirement: Repository commands SHALL use typed canonical service calls

Every `pack.developer.repository.v1` operation SHALL be represented as a typed
command/result DTO and SHALL traverse the canonical service runtime path with
trace, policy, workspace/path/ref scope checks, resource, entitlement, approval,
health, snapshot, redaction, replay, and structured error behavior.

#### Scenario: Repository is opened
- **WHEN** `repository.open` is invoked with a workspace handle, repository root handle, VCS type, and requested capability profile
- **THEN** Macaca SHALL validate workspace trust, declared root scope, VCS support, provider capability, entitlement, and resource budget before provider access
- **AND** it SHALL return a repository handle, current head, dirty-state summary, provider capability hash, health, and replay pointer

#### Scenario: Status is inspected
- **WHEN** `repository.status` is invoked with a repository handle and path scope
- **THEN** Macaca SHALL validate read permission, repository scope, path exclusions, provider capability, redaction, and resource limits
- **AND** it SHALL return typed status entries with path handles, index/worktree state, conflict stage, submodule state, sensitivity class, and dirty-state diagnostics

#### Scenario: Diff is inspected
- **WHEN** `repository.diff` is invoked with staged/unstaged/ref/range selector and result limits
- **THEN** Macaca SHALL validate diff permission, path scope, object-id selectors, redaction, max diff size, provider capability, and resource budget
- **AND** it SHALL return diff summaries, file changes, hunk handles, stats, binary/generated markers, secret-risk flags, and replay pointer without exposing full raw diffs in observability

#### Scenario: Command is denied before provider call
- **WHEN** policy, workspace trust, path/ref scope, permission, entitlement, approval, resource, network, credential reference, object-id, dirty-state, protected-ref, or redaction checks reject a `repository.*` command
- **THEN** Macaca SHALL return a typed denied, approval-required, validation, dirty-worktree, protected-ref, diverged, conflict, quota, timeout, unavailable, or unsupported result before invoking the concrete provider, mutating repository state, or contacting remote hosts
- **AND** audit evidence SHALL include bounded reason codes without raw credentials, private remote URLs, raw source files, full raw diffs, raw provider payloads, or unbounded history

### Requirement: Repository DTOs SHALL model repositories, remotes, refs, branches, tags, commits, status, diffs, mutation plans, sync plans, and provider capability

`pack.developer.repository.v1` SHALL define portable DTOs for repository
handles, remotes, refs, branches, tags, commits, status entries, diffs, mutation
plans, sync plans, provider capabilities, result pages, partial results, and
diagnostics. Provider-specific fields SHALL remain bounded adapter metadata and
SHALL NOT become OS-layer routing branches.

#### Scenario: Developer inspects repository schema
- **WHEN** SDK schemas expose `RepositoryHandle`
- **THEN** the schema SHALL identify repository id, workspace handle, VCS type, trust state, root handle, default branch handle, current head, dirty state, provider capability hash, and health
- **AND** raw absolute paths and private remote URLs SHALL be redacted according to policy

#### Scenario: Developer inspects refs and branches
- **WHEN** SDK schemas expose `RepositoryRef` and `RepositoryBranch`
- **THEN** the schemas SHALL include ref handle, name handle, ref kind, target object id, tracking ref, protection state, upstream status, ahead/behind counts, merge base, divergence status, and last observed timestamp
- **AND** provider-specific branch ids SHALL NOT be required for portable application logic

#### Scenario: Developer inspects commit and diff schemas
- **WHEN** SDK schemas expose `RepositoryCommit` and `RepositoryDiff`
- **THEN** the schemas SHALL include object ids, parent ids, author/committer handles, message handle, timestamp, signature state, tree hash, change stats, base/target selectors, file change handles, hunk handles, binary/generated markers, secret-risk flags, and redaction profile
- **AND** raw commit messages and raw diff hunks SHALL be represented by handles or bounded/redacted snippets in observability

#### Scenario: Provider reports capability limits
- **WHEN** SDK discovery inspects the active repository provider
- **THEN** Macaca SHALL report VCS types, local operations, remote operations, auth modes, protocol support, branch protection metadata, signature support, mutation support, max repository size, rate limits, lifecycle, health, and capability hash
- **AND** callers SHALL use this metadata instead of provider-name branches

### Requirement: Repository mutations SHALL be planned, validated, approval-aware, and auditable

`pack.developer.repository.v1` SHALL separate planning, validation, and mutation
request commands for staging, commit creation, push, merge, rebase-like,
cherry-pick, revert, and other state-changing operations. Mutations SHALL
require write permissions, current object ids, idempotency keys, policy checks,
approval when required, and audit records.

#### Scenario: Commit is planned without mutation
- **WHEN** `repository.plan_commit` is invoked with selected changes, commit message handle, author handle, signing policy, and expected index state
- **THEN** Macaca SHALL validate staging state, path scope, author policy, message policy, signing capability, dirty-state constraints, and resource limits
- **AND** it SHALL return a mutation plan, affected paths, expected object ids, risk flags, required approvals, idempotency key, and recovery notes without creating a commit

#### Scenario: Commit creation request is validated
- **WHEN** `repository.create_commit_request` is invoked with a validated commit plan
- **THEN** Macaca SHALL verify current index/worktree object ids, permissions, approval state, signing policy, and provider mutation capability before mutation
- **AND** it SHALL emit sanitized audit evidence with plan handle, resulting commit handle when successful, and replay pointer

#### Scenario: Mutation validation is non-mutating
- **WHEN** `repository.validate_mutation` is invoked for staged changes, commit plan, push plan, merge plan, or pull plan
- **THEN** Macaca SHALL validate preconditions, dirty state, divergence, protected refs, object ids, conflicts, remote requirements, and approvals
- **AND** it SHALL return typed validation results without mutating repository state or contacting remote hosts unless explicitly declared by the command and policy

#### Scenario: History rewrite requires approval
- **WHEN** a plan includes force-like push, rebase, reset, branch/tag deletion, destructive conflict resolution, or protected ref update
- **THEN** Macaca SHALL return approval-required before mutation unless a valid approval token is supplied
- **AND** trace/audit evidence SHALL record approval state, affected refs, expected object ids, and result code without exposing raw source or credentials

### Requirement: Remote operations SHALL be explicit, bounded, credential-safe, and policy-controlled

`pack.developer.repository.v1` SHALL treat fetch, pull planning, push planning,
push requests, and remote metadata inspection as network-sensitive operations.
Remote operations SHALL require remote scope, credential references, network
permission, resource budgets, redacted diagnostics, and approval for protected
or destructive writes.

#### Scenario: Remotes are listed
- **WHEN** `repository.list_remotes` is invoked
- **THEN** Macaca SHALL validate remote-read permission and redaction policy
- **AND** it SHALL return remote handles, redacted URL handles, provider class, fetch/push capability, credential reference status, default branch, and permission state without raw credentials or private remote URLs

#### Scenario: Fetch is executed
- **WHEN** `repository.fetch` is invoked with remote handle, ref selectors, credential reference, timeout, and resource budget
- **THEN** Macaca SHALL validate network permission, remote scope, credential reference, provider capability, transfer budget, cancellation behavior, and redaction before contacting the remote provider
- **AND** it SHALL return updated remote refs, fetched object summary, rate/quota diagnostics, and replay pointer

#### Scenario: Push is planned before request
- **WHEN** `repository.plan_push` is invoked with source refs, target refs, remote handle, force policy, and expected object ids
- **THEN** Macaca SHALL validate upstream state, ahead/behind counts, divergence, protected branch policy, remote permission, credential reference, and approval requirements without pushing
- **AND** it SHALL return a sync plan, protected-ref diagnostics, required approvals, network requirements, and replay pointer

#### Scenario: Push request is protected
- **WHEN** `repository.push_request` targets protected refs, force-like updates, or diverged refs
- **THEN** Macaca SHALL require a validated sync plan, remote push permission, valid credential reference, and approval when policy requires it
- **AND** it SHALL return protected-ref, diverged, denied, approval-required, or success diagnostics without exposing credentials or raw provider payloads

### Requirement: Repository Pack SHALL enforce permissions, scopes, resource limits, entitlements, approvals, and redaction

`pack.developer.repository.v1` SHALL define permission scopes for local reads,
local writes, status, diff, history, refs, staging, commit creation, remote
reads, fetch, push, remote metadata, mutation planning, mutation validation, and
provider inspection. Policy SHALL run before side effects and SHALL account for
workspace trust, path scope, ref scope, remote scope, credential references,
network access, protected refs, dirty state, divergence, provider quota, output
size, approval, and redaction.

#### Scenario: Path is outside repository scope
- **WHEN** a command targets a file path outside declared repository roots or inside denied path scopes
- **THEN** Macaca SHALL return a typed denied result before provider access
- **AND** the concrete provider SHALL NOT receive the out-of-scope path

#### Scenario: Remote permission is missing
- **WHEN** an application can read local repository status but lacks `repository.remote.fetch` or `repository.remote.push`
- **THEN** remote commands SHALL return typed denied results before contacting remote hosts
- **AND** audit evidence SHALL identify the missing scope by stable code

#### Scenario: Resource limits reject repository operation
- **WHEN** status, diff, history, fetch, push planning, mutation validation, or remote metadata inspection exceeds repository size, file count, diff size, history page size, ref count, transfer estimate, timeout, memory, storage, network, provider quota, output, or snapshot limits
- **THEN** Macaca SHALL return typed quota, timeout, cancellation, or partial-result diagnostics
- **AND** it SHALL emit bounded resource counters and stable reason codes

### Requirement: Repository Pack SHALL expose industrial metadata and developer documentation

`pack.developer.repository.v1` SHALL expose descriptor metadata for VCS types,
local operations, remote operations, auth modes, protocol support, mutation
support, branch protection metadata, command schemas, permission scopes, policy
templates, resource budgets, approval requirements, lifecycle state,
compatibility, health probes, snapshots, unavailable diagnostics, redaction
profiles, SDK examples, provider capability hashes, and developer documentation.

#### Scenario: Developer inspects the pack
- **WHEN** SDK discovery inspects `pack.developer.repository.v1`
- **THEN** it SHALL return command namespace `repository.*`, VCS types, local operation support, remote operation support, auth modes, protocol support, mutation support, branch protection metadata support, supported commands, permissions, policy templates, examples, lifecycle, availability, health, diagnostics, compatibility, redaction profile, provider capability hash, and documentation links
- **AND** examples SHALL use generic handles and synthetic object ids rather than application-specific workflows, provider names, credentials, real remotes, private source code, or repository-specific conventions

#### Scenario: Developer documentation is published
- **WHEN** the pack implementation is marked complete
- **THEN** `docs/developer-packs/developer/repository.md` SHALL document manifest declaration, required versus optional behavior, permissions, repository handles, workspace/path/ref scopes, refs, branches, tags, commits, status entries, diffs, remotes, sync plans, mutation plans, protected branch diagnostics, credential references, network policy, unavailable diagnostics, provider replacement, trace/audit interpretation, operational limits, and conformance tests
- **AND** SDK discovery metadata and the industrial catalog index SHALL link to that guide

### Requirement: Repository Pack SHALL be traceable, auditable, replayable, and sanitized

`pack.developer.repository.v1` SHALL emit sanitized trace/audit events and
bounded snapshots for declaration, admission, repository opening, inspection,
status, refs, history, diff, staging, commit planning, commit requests, remote
listing, fetch, pull planning, push planning, push requests, merge planning,
mutation validation, remote metadata, provider inspection, policy/resource
decisions, provider calls, unavailable states, and replay.

#### Scenario: Snapshot is recorded
- **WHEN** the service runtime records a repository pack snapshot
- **THEN** the snapshot SHALL include descriptor version, provider capability hashes, VCS type, current head hash, branch/ref summary, dirty-state summary, remote capability hashes, command availability, provider health, policy template hash, resource counters, bounded mutation-plan summaries, and sanitized replay pointers
- **AND** it SHALL exclude raw credentials, access tokens, private remote URLs, raw source, full raw diffs, raw provider payloads, manifests, package bytes, private keys, signatures, and unbounded history

#### Scenario: Mutation request is audited
- **WHEN** `repository.stage_changes`, `repository.create_commit_request`, `repository.push_request`, or merge/rebase-like mutation requests run
- **THEN** Macaca SHALL emit sanitized audit events with repository handle, mutation plan handle, affected path/ref handles, expected object ids, approval status, credential reference status where relevant, result code, and replay pointer
- **AND** raw source, full raw diffs, credentials, and raw provider payloads SHALL NOT enter audit records

#### Scenario: Remote operation is audited
- **WHEN** `repository.fetch`, `repository.plan_push`, `repository.push_request`, or `repository.inspect_remote_metadata` runs
- **THEN** Macaca SHALL emit sanitized audit events with remote handle, redacted provider class, ref selectors, network policy decision, credential reference status, resource counters, result code, and replay pointer
- **AND** private remote URLs, access tokens, and raw provider payloads SHALL NOT enter audit records

### Requirement: Repository Pack implementation SHALL preserve Macaca boundaries

The `pack.developer.repository.v1` implementation SHALL remain owned by
repository service providers behind the service runtime. The microkernel, SDK,
shells, and generic application framework SHALL remain provider-neutral and free
of application-specific or provider-specific routing branches.

#### Scenario: Boundary gates scan the implementation
- **WHEN** dependency, no-direct-provider-call, and canonical execution-path gates scan the implementation
- **THEN** they SHALL find no concrete Git library, Git CLI, GitHub client, GitLab client, Bitbucket client, SSH client, credential manager, terminal client, or provider adapter imports in the microkernel, SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned service registrations and typed service commands

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, mock, or unavailable provider is selected for `pack.developer.repository.v1`
- **THEN** callers SHALL observe the same provider-neutral command/result contract
- **AND** trace/audit evidence SHALL identify only sanitized provider class, descriptor metadata, capability hash, and result codes rather than provider-specific business branches

#### Scenario: SDK helper builds service call only
- **WHEN** an SDK helper such as `sdk.packs.developer.repository.plan_push(command)` is used
- **THEN** the helper SHALL build a canonical traced service call with command DTO, permission metadata, repository handle, path/ref scope, resource limits, redaction profile, and replay context
- **AND** it SHALL NOT construct providers, run Git commands, access credentials, contact remote hosts, mutate repository state, route by provider name, or bypass policy
