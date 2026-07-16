# Change: Add Developer Repository Pack

## Why

Developers need `pack.developer.repository.v1` as an industrial repository
capability for local repository state, remote repository metadata, refs,
branches, tags, commits, status, diffs, staging, commit creation, fetch, pull,
push, merge/rebase planning, worktree safety, protected branch policy, and
remote provider diagnostics. It must not be a thin wrapper around `git` commands
or a GitHub/GitLab-specific workflow.

Repository operations can read private source, mutate worktrees, publish commits,
rewrite history, contact remote hosts, consume credentials, and trigger external
side effects. Macaca must expose these operations as provider-neutral typed
commands with workspace scope, credential redaction, policy gates, explicit
approvals, resource limits, trace/audit records, snapshots, replay, and
structured unavailable behavior.

## Research And Supplier/API Baseline

Official references considered for this pack:

- Git documentation describes the distributed object model, refs, branches,
  tags, remotes, worktrees, status, diff, add, commit, fetch, pull, push, merge,
  rebase, reset, and protocol behavior. References:
  https://git-scm.com/docs and https://git-scm.com/book/en/v2
- GitHub REST API documentation covers repositories, contents, branches,
  commits, pulls, refs, tags, compare, statuses/checks, collaborators, and
  protected branch concepts. Reference:
  https://docs.github.com/en/rest
- GitLab REST API documentation covers projects/repositories, branches, commits,
  merge requests, repository files, protected branches, pipeline/status
  metadata, and permissions. Reference:
  https://docs.gitlab.com/api/
- Bitbucket Cloud REST API documentation covers repositories, refs, branches,
  commits, pull requests, source/contents, and workspace/project-scoped
  repository metadata. Reference:
  https://developer.atlassian.com/cloud/bitbucket/rest/

Macaca maps these supplier/platform capabilities into provider-neutral
repository DTOs and service commands. Concrete Git libraries, host CLIs,
GitHub/GitLab/Bitbucket clients, credentials, and platform workflows remain
behind replaceable providers.

## What Changes

- Add provider-neutral `pack.developer.repository.v1` under the `developer`
  family.
- Define command namespace `repository.*` for:
  - repository binding/opening and metadata inspection
  - local status, staging, diff, refs, branches, tags, commits, and history
  - remote listing, fetch, pull planning, push planning, and push requests
  - merge/rebase/cherry-pick/revert planning and validation
  - commit creation request and signing metadata
  - worktree safety checks and dirty-state diagnostics
  - remote platform metadata inspection
  - provider capability inspection
- Define DTOs for repository handles, remotes, refs, branches, tags, commits,
  object ids, status entries, diffs, staged changes, commit plans, sync plans,
  merge/rebase plans, push requests, protected branch diagnostics, remote
  metadata, provider capabilities, and structured diagnostics.
- Define permission scopes, policy defaults, host/workspace gates, credential
  redaction, approval rules, entitlement checks, structured unavailable behavior,
  SDK discovery, developer documentation, trace/audit events, snapshots, replay,
  and boundary gates.
- Require detailed developer documentation at
  `docs/developer-packs/developer/repository.md` before implementation
  completion.

## Impact

- Affected specs: `pack-developer-repository`,
  `developer-pack-industrial-capability-catalog`, `sdk-system-facade`,
  `service-runtime`, `unified-execution-path`.
- Affected code later: provider-neutral protocol DTOs, pack descriptors,
  admission validators, SDK discovery and command builders, repository service
  provider or unavailable provider, runtime-host provider adapters, trace/audit
  schemas, replay tests, dependency-boundary gates, and developer documentation.
- Non-goals: no concrete Git CLI/libgit2/JGit/GitHub/GitLab/Bitbucket provider
  implementation in this proposal; no application-specific PR/release workflow;
  no provider-name routing in OS layers; no raw credentials, tokens, remotes, or
  full diffs in observability; no SDK/shell/kernel provider construction; no
  fake success when provider, workspace, entitlement, permission, remote access,
  or host support is absent.
