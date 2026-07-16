## 1. Supplier API Research And Scope

- [x] 1.1 Re-read architecture governance, microkernel boundaries, serviceization allowlist, design patterns, OpenSpec rules, and the industrial catalog umbrella proposal before implementation.
- [x] 1.2 Study Git official docs for object model, refs, branches, tags, worktrees, index/staging, status, diff, commit, fetch, pull, push, merge, rebase, reset, remotes, and protocol behavior.
- [x] 1.3 Study GitHub REST API docs for repositories, contents, branches, commits, refs, tags, compare, pulls, checks/statuses, collaborators, permissions, and protected branches.
- [x] 1.4 Study GitLab API docs for projects/repositories, repository files, branches, commits, merge requests, protected branches, pipeline/status metadata, and permissions.
- [x] 1.5 Study Bitbucket Cloud REST API docs for workspaces, repositories, refs, branches, commits, pull requests, source/contents, and permission metadata.
- [x] 1.6 Produce a supplier capability comparison memo mapping Git, GitHub, GitLab, and Bitbucket concepts into Macaca provider-neutral repository DTOs and commands.
- [x] 1.7 Define explicit non-goals for concrete provider adapters, PR/release workflows, CI orchestration, terminal execution, raw Git command pass-through, and provider-specific routing.
- [x] 1.8 Record GitNexus CRITICAL/HIGH findings as memo only before implementation commits.

## 2. Contract, Descriptor, And DTOs

- [x] 2.1 Define `pack.developer.repository.v1` descriptor metadata: pack id, family, lifecycle, stability, VCS support, local operation support, remote operation support, auth modes, protocol support, mutation support, branch protection support, command schemas, permission scopes, policy templates, resource budgets, approval requirements, data-governance class, SDK metadata, documentation link, compatibility, and diagnostics.
- [x] 2.2 Define provider-neutral DTOs for `RepositoryHandle`, `RepositoryRemote`, `RepositoryRef`, `RepositoryBranch`, `RepositoryTag`, `RepositoryCommit`, `RepositoryStatusEntry`, `RepositoryDiff`, `RepositoryMutationPlan`, `RepositorySyncPlan`, and `RepositoryProviderCapability`.
- [x] 2.3 Define typed command/result DTOs for `repository.open`, `repository.inspect`, `repository.status`, `repository.list_refs`, `repository.inspect_history`, `repository.diff`, `repository.stage_changes`, `repository.plan_commit`, `repository.create_commit_request`, `repository.list_remotes`, `repository.fetch`, `repository.plan_pull`, `repository.plan_push`, `repository.push_request`, `repository.plan_merge`, `repository.validate_mutation`, `repository.inspect_remote_metadata`, and `repository.inspect_provider`.
- [x] 2.4 Define typed success, paged result, partial result, dry-run/plan result, validation issue, denied, unavailable, unsupported, conflict, diverged, dirty-worktree, protected-ref, quota, timeout, cancellation, approval-required, and failure DTOs.
- [x] 2.5 Define stable descriptor hashing, provider capability hashing, repository identity hashing, object-id normalization, ref state hashing, dirty-state hashing, diff hashing, mutation-plan hashing, sync-plan hashing, remote metadata hashing, and redaction metadata.
- [x] 2.6 Add descriptor and DTO compatibility tests for valid descriptors, rejected invalid descriptors, stable hashes, schema evolution, ref states, dirty states, diff summaries, sync plans, mutation plans, redaction profiles, and serde compatibility.

## 3. Admission, Permission, Policy, Resource, Entitlement, And Approval

- [x] 3.1 Implement manifest declaration validation for required and optional `pack.developer.repository.v1` declarations.
- [x] 3.2 Implement permission validation for `repository.local.read`, `repository.local.write`, `repository.status.read`, `repository.diff.read`, `repository.history.read`, `repository.ref.read`, `repository.ref.write`, `repository.stage.write`, `repository.commit.create`, `repository.remote.read`, `repository.remote.fetch`, `repository.remote.push`, `repository.remote.metadata`, `repository.mutation.plan`, `repository.mutation.validate`, and `repository.provider.inspect`.
- [ ] 3.3 Implement workspace/path/ref scope checks for declared repository roots, excluded paths, secret files, credentials, generated artifacts, vendor directories, protected files, protected refs, and remote scopes.
- [ ] 3.4 Implement policy checks for VCS support, repository trust, dirty-state safety, object-id preconditions, branch protection, commit author/signing policy, message policy, network policy, credential reference, force policy, merge/rebase strategy, and output redaction.
- [ ] 3.5 Implement resource reservation for repository size, file count, status entries, diff size, history page size, remote refs, fetch/push transfer estimate, timeout, memory, storage, network, provider quota, streaming output, and retained snapshots.
- [ ] 3.6 Implement entitlement checks and structured unavailable/denied diagnostics for missing provider, disabled pack, missing workspace trust, missing path/ref permission, unsupported VCS, absent remote support, missing credential reference, missing entitlement, disabled network, unsupported mutation, protected branch, diverged refs, dirty worktree, and host resource denial.
- [ ] 3.7 Implement approval behavior for commit creation, staging protected files, push to protected refs, force-like pushes, history rewrites, branch/tag deletion, destructive resets, conflict resolution, remote network writes, and broad repository mutations.
- [ ] 3.8 Add tests proving denied, validation, quota, unavailable, conflict, diverged, dirty-worktree, protected-ref, and approval-required paths do not call concrete providers or mutate repository state.

## 4. Service Provider And Runtime Integration

- [ ] 4.1 Implement or bind the repository service provider behind the service runtime; do not construct repository providers from SDK, shell, kernel, or application code.
- [x] 4.2 Add a deterministic unavailable provider that returns typed unavailable/unsupported diagnostics and complete discovery metadata.
- [ ] 4.3 Add mock provider support for repository open/inspect, status, refs, history, diff, staging, commit planning, commit request, remotes, fetch, pull planning, push planning, push request, merge planning, mutation validation, remote metadata inspection, and provider capability inspection.
- [ ] 4.4 Add lifecycle, health, snapshot, shutdown, timeout, cancellation, bounded streaming, paged results, and dirty/stale state diagnostics.
- [ ] 4.5 Add Strategy implementations for VCS adapters, remote API adapters, auth adapters, diff providers, sync planners, mutation validators, branch protection inspectors, and unavailable behavior.
- [ ] 4.6 Add mutation safety support for current object-id verification, dirty-state checks, conflict prediction, protected-ref checks, approval state, rollback/recovery guidance, and non-mutating validation.
- [ ] 4.7 Add provider capability reporting for available, degraded, preview, unavailable, unsupported, retired, VCS-specific, remote-specific, auth-specific, mutation-limited, protected-ref-limited, network-limited, and quota-limited states.

## 5. SDK, WASM ABI, Application Framework, And Examples

- [x] 5.1 Extend SDK discovery for `pack.developer.repository.v1` with command schemas, VCS support, local operations, remote operations, auth modes, protocol support, mutation support, protected branch support, examples, availability, diagnostics, documentation link, provider class, capability hash, compatibility, and redaction profile.
- [x] 5.2 Extend application admission so required declarations block readiness when unavailable and optional declarations degrade explicitly with effective capability mementos.
- [x] 5.3 Add SDK command helper builders for all `repository.*` commands; helpers must only build canonical traced service calls and must never construct Git clients, run Git commands, create remote API clients, access credentials, or bypass policy.
- [ ] 5.4 Extend WASM/app ABI descriptors so applications can discover repository commands, declare permissions, receive unavailable diagnostics, and submit typed service calls through the canonical execution path.
- [x] 5.5 Add generic app-facing examples for opening a repository, inspecting status, listing refs, inspecting history, diffing changes, planning a commit, validating a mutation, planning a push, requesting a push, and inspecting remote metadata.
- [x] 5.6 Add unavailable-provider, missing-repository-permission, unsupported-VCS, dirty-worktree, diverged-ref, protected-branch, missing-credential-reference, network-denied, and approval-required examples that demonstrate diagnostics without provider names, credentials, real remotes, private source code, or repository-specific workflows.

## 6. Trace, Audit, Replay, Security, And Gates

- [ ] 6.1 Emit sanitized declaration, admission, repository-open, inspect, status, refs, history, diff, staging, commit-plan, commit-request, remote-list, fetch, pull-plan, push-plan, push-request, merge-plan, mutation-validation, remote-metadata, provider-inspection, policy, entitlement, resource, approval, health, snapshot, unavailable, and failure events.
- [ ] 6.2 Ensure traces, audits, snapshots, SDK diagnostics, and examples exclude raw credentials, access tokens, private remote URLs, raw source files, full raw diffs, raw provider payloads, prompts, manifests, package bytes, private keys, signatures, and unbounded history.
- [ ] 6.3 Add replay tests proving every `repository.*` command is trace-addressable through the canonical service path and that snapshots contain enough bounded metadata for recovery diagnostics.
- [ ] 6.4 Add dependency gates proving kernel, SDK, shells, and generic application framework do not import concrete Git libraries, Git CLIs, GitHub/GitLab/Bitbucket clients, SSH clients, credential managers, terminal clients, or provider adapters.
- [ ] 6.5 Add no-direct-provider-call gates proving SDK helpers, WASM ABI handlers, app admission, web, CLI, and frontend paths route through descriptor-owned service commands.
- [ ] 6.6 Add boundary tests proving optional provider absence returns structured unavailable diagnostics and never crashes, hangs, silently falls back, mutates repository state, contacts remote hosts, or fakes success.
- [ ] 6.7 Run `openspec validate add-pack-developer-repository --strict`, targeted cargo tests, boundary gates, file-size gates, and audit replay checks before marking implementation complete.

## 7. Developer Documentation

- [x] 7.1 Create `docs/developer-packs/developer/repository.md` with purpose, capability model, manifest declaration, required versus optional behavior, permissions, repository handles, workspace/path/ref scopes, refs, branches, tags, commits, status entries, diffs, remotes, sync plans, mutation plans, protected branch diagnostics, unavailable diagnostics, provider replacement, and operational limits.
- [x] 7.2 Document all command DTOs and result DTOs with field-level explanations, idempotency semantics, redaction behavior, pagination/streaming behavior, timeout/cancellation behavior, dry-run/plan behavior, approval behavior, rollback/recovery behavior, and structured error codes.
- [x] 7.3 Document supplier/API mapping: Git, GitHub REST API, GitLab API, and Bitbucket Cloud REST API concepts mapped to Macaca abstractions, including what is intentionally not exposed as OS semantics.
- [x] 7.4 Add generic examples for status, refs, history, diff, commit planning, fetch, pull planning, push planning, protected branch diagnostics, mutation validation, remote metadata, and unavailable diagnostics using synthetic repositories only.
- [x] 7.5 Add conformance checklist and test guidance for provider authors: descriptor completeness, VCS support, remote support, auth redaction, dirty-state safety, object-id checks, branch protection, mutation validation, resource bounds, policy hooks, trace/audit events, unavailable behavior, snapshot/replay, and redaction.
- [x] 7.6 Cross-link the guide from SDK discovery metadata and the industrial pack catalog index before marking `add-pack-developer-repository` complete.
