# Developer Repository Pack Research

## Purpose

This note records supplier/API research, Macaca provider-neutral mapping,
explicit non-goals, existing platform inventory, and GitNexus memo evidence for
`pack.developer.repository.v1`. Repository support must expose repository,
object, ref, branch, tag, worktree, index, status, diff, commit, fetch, pull,
push, merge, rebase, reset, remote, protected-branch, pull-request, and
permission metadata through typed service commands. It must not expose raw Git
commands, provider APIs, release workflows, CI orchestration, or terminal
execution as OS semantics.

## Source Baseline

- Git official docs:
  <https://git-scm.com/docs/git-pull>
  and <https://git-scm.com/docs>
- GitHub repository, refs, commits, statuses, and pull request APIs:
  <https://docs.github.com/rest/repos/repos>
  <https://docs.github.com/rest/git/refs>
  <https://docs.github.com/v3/repos/commits>
  <https://docs.github.com/rest/commits/statuses>
  <https://docs.github.com/en/rest/pulls/pulls>
- GitLab APIs for protected branches and related repository capabilities:
  <https://docs.gitlab.com/api/protected_branches/>
- Bitbucket Cloud REST API repositories:
  <https://developer.atlassian.com/cloud/bitbucket/rest/>
  and
  <https://developer.atlassian.com/cloud/bitbucket/rest/api-group-repositories/>

## Supplier API Notes

- Git contributes object model, refs, branches, tags, worktrees, index/staging,
  status, diff, commit, fetch, pull, push, merge, rebase, reset, remotes, and
  protocol behavior. Macaca should model repository operations as typed command
  plans with policy and rollback/recovery metadata, not raw command strings.
- GitHub REST contributes repositories, contents, branches, commits, refs, tags,
  compare, pull requests, checks/statuses, collaborators, permissions, and
  protected branches. Macaca should separate Git object/ref operations from
  platform review/status metadata.
- GitLab contributes projects/repositories, repository files, branches, commits,
  merge requests, protected branches, pipeline/status metadata, and permission
  models. Macaca should normalize protected-branch and merge-request capability.
- Bitbucket Cloud contributes workspaces, repositories, refs, branches, commits,
  pull requests, source/contents, OAuth scopes, and permission metadata.

## Macaca-Owned Abstractions

`pack.developer.repository.v1` should define `RepositoryHandle`,
`RepositoryObject`, `RepositoryRef`, `RepositoryBranch`, `RepositoryTag`,
`RepositoryWorktree`, `RepositoryIndex`, `RepositoryStatus`,
`RepositoryDiff`, `RepositoryCommit`, `RepositoryRemote`,
`RepositoryOperationPlan`, `RepositoryPullRequest`, `RepositoryProtection`,
`RepositoryPermission`, and `RepositoryProviderCapability`.

The DTOs must carry repository identity, object/ref hashes, worktree/index
state, diff bounds, commit metadata, remote capability, operation preconditions,
conflict/merge/rebase/reset risk, protected-branch policy, permission state,
provider capability hashes, redaction profiles, and replay pointers. Raw Git
command strings, credentials, provider payloads, private source content beyond
declared handles, and unbounded diffs/logs are rejected.

## Explicit Non-Goals

- Do not implement concrete Git CLI/libgit2, GitHub, GitLab, Bitbucket,
  pull-request, release, CI, terminal, credential, or storage providers in this
  research phase.
- Do not define PR/release workflows, code review policies, CI orchestration,
  deployment behavior, or application-specific repository automation in OS
  layers.
- Do not expose raw Git commands, provider API requests, branch names as
  hardcoded routing, or provider-specific ids as stable OS semantics.

## Existing Macaca Platform Inventory

- Generic descriptors, `SystemFacade`, trace-required service calls,
  unavailable/null-object behavior, policy/resource gates, persistence
  snapshots, terminal/CI/code/issue-tracker adjacency, and secrets-reference
  handles provide reusable substrate.
- Current evidence does not prove repository DTOs, providers, SDK helpers, WASM
  ABI, tests, dependency gates, or developer docs.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
