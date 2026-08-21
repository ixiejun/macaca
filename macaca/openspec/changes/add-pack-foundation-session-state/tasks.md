## 1. Supplier API Research And Scope

- [x] 1.1 Read and summarize Web `sessionStorage` concepts for origin/tab
  partitioning, page-session lifetime, key-value storage, and session isolation.
- [x] 1.2 Read and summarize Android SavedStateHandle and save UI state guidance
  for process-death restoration and ViewModel state.
- [x] 1.3 Read and summarize Apple UIKit/SwiftUI state restoration and
  `NSUserActivity` concepts for restoring app/scene state on launch.
- [x] 1.4 Read and summarize Temporal event history and Continue-As-New concepts
  for replayable histories, run ids, workflow ids, and latest-state handoff.
- [x] 1.5 Read and summarize Redis/server-backed session store concepts for TTL,
  invalidation, serialization, and distributed recovery.
- [x] 1.6 Convert the supplier comparison into Macaca-owned abstractions and
  explicitly reject workflow/task semantics and shell-owned state repair.
- [x] 1.7 Record GitNexus CRITICAL/HIGH findings as memo only before
  implementation commits.

## 2. Contract, Descriptor, And DTOs

- [x] 2.1 Define `pack.foundation.session.state.v1` descriptor metadata:
  lifecycle, stability, service ids, command namespace, command schemas,
  permission scopes, policy template, resource template, SDK metadata, docs
  link, health, snapshot, and unavailable diagnostics.
- [x] 2.2 Define command DTOs for `session_state.get`, `session_state.put`,
  `session_state.delete`, `session_state.merge_patch`,
  `session_state.list_keys`, `session_state.create_checkpoint`,
  `session_state.list_checkpoints`, `session_state.restore_checkpoint`,
  `session_state.compare_checkpoint`, `session_state.compact_history`,
  `session_state.clear_session`, `session_state.export_redacted`, and
  `session_state.inspect_recovery`.
- [x] 2.3 Define shared DTOs for session refs, state key refs, typed values,
  revisions, checkpoint refs, restore plans, recovery metadata, retention policy,
  redaction summary, provider capability report, and stable descriptor hashes.
- [x] 2.4 Define result/error DTOs for success, partial page, denied, not_found,
  conflict, invalid_session, invalid_key, invalid_checkpoint, schema_mismatch,
  quota_exceeded, too_large, unsupported, unavailable, and provider_failure.
- [x] 2.5 Add schema compatibility tests and stable hash tests for command,
  result, health, snapshot, provider capability, and unavailable DTOs.

## 3. Admission, Permission, Policy, Resource, And Approval

- [x] 3.1 Implement manifest declaration validation for required/optional
  `pack.foundation.session.state.v1`, session scopes, checkpoint support, and
  retention policy.
- [x] 3.2 Validate scopes: `session_state.read`, `session_state.write`,
  `session_state.delete`, `session_state.list`, `session_state.checkpoint`,
  `session_state.restore`, `session_state.compact`, `session_state.clear`,
  `session_state.export`, and `session_state.inspect_recovery`.
- [x] 3.3 Add policy checks for session id, task id, key/prefix bounds, max state
  size, max checkpoint size, retention, restore mode, compaction bounds, export
  redaction, and provider capability.
- [x] 3.4 Add side-effect approval behavior for restore, clear, compaction, broad
  export, and cross-session restore.
- [x] 3.5 Reject raw secrets and require secret-reference interoperability for
  secret-classified state.
- [ ] 3.6 Add tests proving denied, unavailable, quota, schema_mismatch, invalid
  checkpoint, and unsupported paths do not mutate provider state.

## 4. Service Provider And Runtime Integration

- [x] 4.1 Define the session-state service trait/provider interface behind the
  service runtime.
- [x] 4.2 Implement unavailable provider behavior for absent session-state service,
  disabled checkpoint/restore/compaction/export support, missing retention
  support, and provider health failure.
- [x] 4.3 Implement deterministic mock provider for contract and replay tests.
- [x] 4.4 Implement or bind embedded durable provider with revision tracking,
  checkpoint references, retention enforcement, compaction, and restore dry-run.
- [x] 4.5 Add optional remote session store/replay provider bridge points without
  leaking provider-native APIs to SDK callers.
- [x] 4.6 Add lifecycle, health, snapshot, shutdown, retention cleanup, compaction,
  restore, redaction, and provider capability reports.

## 5. SDK, WASM ABI, And Application Framework

- [x] 5.1 Extend SDK discovery with pack metadata, command schemas, value types,
  checkpoint/restore/compaction support, permissions, policy templates, provider
  availability, health, diagnostics, and docs link.
- [x] 5.2 Add SDK command builders for every `session_state.*` command; builders
  must only produce canonical traced service calls.
- [x] 5.3 Add SDK helpers for get/put, merge patch, checkpoint creation, restore
  dry-run, checkpoint comparison, compaction, clear, recovery inspection, and
  unavailable diagnostics.
- [x] 5.4 Extend effective capability projection so applications can inspect
  callable commands, denied commands, unavailable session features, provider
  capability flags, latest checkpoint, and replay references.
- [x] 5.5 Expose WASM host imports only for declared callable session-state
  commands and route every import through the service runtime path.
- [x] 5.6 Add app-framework tests proving YAML, WASM, GenUI, and headless apps all
  use the same session-state execution path.

## 6. Trace, Audit, Replay, And Gates

- [x] 6.1 Emit sanitized events for declaration, admission, policy, resource,
  service calls, checkpoint creation, restore, compaction, clear, success,
  failure, denied, and unavailable states.
- [x] 6.2 Add audit redaction tests proving raw state values, raw secrets, prompts,
  manifests, package bytes, credentials, private keys, provider payloads, and
  unbounded output do not enter observability surfaces.
- [x] 6.3 Add restart/replay tests proving session-state commands are
  trace-addressable and can reconstruct recovery decisions without replaying raw
  state payloads.
- [x] 6.4 Add dependency-boundary tests proving kernel, SDK, shells, and
  application framework do not import concrete session-state providers.
- [x] 6.5 Add no-direct-provider-call gates proving SDK helpers and WASM host
  imports cannot bypass service runtime.
- [x] 6.6 Run `openspec validate add-pack-foundation-session-state --strict`,
  targeted cargo tests, dependency-boundary gates, file-size gates, and audit
  replay checks before marking complete.

## 7. Developer Documentation

- [x] 7.1 Create `docs/developer-packs/foundation/session-state.md`.
- [x] 7.2 Document purpose, manifest declaration, session scope, state versus
  workflow boundary, key/value model, revisions, checkpoints, restore,
  compaction, clear, retention, redaction, permissions, policy defaults, command
  DTOs, result DTOs, error DTOs, unavailable diagnostics, and provider
  replacement.
- [x] 7.3 Add minimal examples for saving transient form state, creating a
  checkpoint, restore dry-run, schema mismatch handling, compacting history,
  denied clear session, unavailable provider diagnostics, and WASM host import
  usage.
- [x] 7.4 Cross-link the guide from SDK discovery metadata and the industrial pack
  catalog index before marking this proposal complete.
