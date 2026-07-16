# Foundation Session State Pack Research

## Purpose

This note records supplier/API research for
`pack.foundation.session.state.v1`. The pack must provide bounded,
session-scoped state, checkpoints, restore, compaction, and recovery metadata
without becoming the workflow/task state machine or shell-owned state repair
logic.

## Source Baseline

- MDN `sessionStorage`:
  <https://developer.mozilla.org/en-US/docs/Web/API/Window/sessionStorage>
- MDN Web Storage API:
  <https://developer.mozilla.org/en-US/docs/Web/API/Web_Storage_API>
- Android `SavedStateHandle`:
  <https://developer.android.com/topic/libraries/architecture/viewmodel/viewmodel-savedstate>
- Android save UI states:
  <https://developer.android.com/topic/libraries/architecture/saving-states>
- Apple UIKit state restoration:
  <https://developer.apple.com/documentation/uikit/restoring-your-app-s-state>
- Apple SwiftUI state restoration:
  <https://developer.apple.com/documentation/SwiftUI/restoring-your-app-s-state-with-swiftui>
- Temporal Continue-As-New:
  <https://docs.temporal.io/workflow-execution/continue-as-new>
- Temporal Workflow Id and Run Id:
  <https://docs.temporal.io/workflow-execution/workflowid-runid>
- Redis session store:
  <https://redis.io/docs/latest/develop/use-cases/session-store/nodejs/>
- Redis `EXPIRE`:
  <https://redis.io/docs/latest/commands/expire/>
- Redis `TTL`:
  <https://redis.io/docs/latest/commands/ttl/>

## Web `sessionStorage` Summary

Browser `sessionStorage` establishes a scoped transient-state model:

- Storage is partitioned by origin and top-level browsing context, which maps to
  Macaca tenant/app/session partitioning.
- Data lasts for the page session and is isolated from other tabs. Macaca should
  make session lifetime, retention, and restore scope explicit.
- Web Storage is string key-value storage. Macaca should expose typed,
  schema-aware, bounded state values instead of raw browser string storage.
- Browser Storage objects and DOM events must not leak into SDK/ABI contracts.

## Android SavedStateHandle Summary

Android's saved-state model contributes process-death restoration and ViewModel
state concepts:

- `SavedStateHandle` is a key-value map for values that need to survive process
  death and recreate UI/application state.
- Android guidance distinguishes saved UI state from durable app data. Macaca
  should keep session-state bounded and transient; durable records belong to
  state/database/application services.
- Restored state should be schema-aware and small enough for recovery. Macaca
  should enforce max state size, checkpoint size, and redaction.
- Android ViewModel and lifecycle classes are provider concepts, not Macaca
  stable SDK types.

## Apple UIKit / SwiftUI Restoration Summary

Apple state restoration and `NSUserActivity` contribute app/scene restoration
metadata:

- Restoration records enough state to re-create a scene or activity after app
  relaunch.
- State restoration is identity-driven; Macaca should store session refs,
  runtime ids, checkpoint refs, and recovery metadata.
- `NSUserActivity`-style restoration intent maps to bounded redacted restore
  metadata, not raw UI or shell state.
- UIKit/SwiftUI state restoration APIs and scene objects must remain provider
  details.

## Temporal Event History / Continue-As-New Summary

Temporal contributes replayable histories and fresh-run handoff concepts:

- Continue-As-New checkpoints current workflow state and starts a fresh run,
  reducing long history growth.
- Workflow Id and Run Id distinguish logical workflow identity from individual
  executions. Macaca should model session id, runtime id, checkpoint id, and
  recovery chain separately.
- Temporal history is a workflow concern. Macaca session-state may store
  checkpoint/replay evidence, but workflow planning, retry, review, and task
  board semantics remain in workflow/autonomy services.
- Compaction and restore must be explicit, traceable, and policy-governed.

## Redis / Server-Backed Session Store Summary

Redis-backed session stores contribute TTL, invalidation, serialization, and
distributed recovery:

- Session records can be centralized so multiple workers can recover the same
  session state.
- TTL/EXPIRE and TTL inspection map to retention policy, expiration state, and
  cleanup diagnostics.
- Invalidation/logout maps to `session_state.clear_session`.
- Serialized session payloads require schema versioning, size limits,
  redaction, and provider health diagnostics.
- Redis keys and TTL sentinel values must not become Macaca result semantics.

## Macaca-Owned Abstractions

`pack.foundation.session.state.v1` should define these provider-neutral
concepts:

- `SessionStateRef`: tenant id, app id, session id, optional task id, runtime id,
  and trace binding.
- `SessionStateKeyRef`: normalized key, prefix policy, schema field reference,
  and redaction label.
- `SessionStateValue`: typed primitive, JSON object, bounded bytes reference,
  artifact reference, or secret reference; raw secrets are forbidden.
- `SessionStateRevision`: opaque revision id, checkpoint sequence, provider
  revision, and replay binding.
- `SessionCheckpointRef`: checkpoint id, session id, state hash, schema version,
  retention policy, creation reason, and replay refs.
- `SessionRestorePlan`: source checkpoint, target session, conflict mode,
  dry-run result, expected current revision, and approval requirement.
- `SessionRecoveryMetadata`: last checkpoint, provider health, compaction state,
  replay refs, restore eligibility, and unavailable reasons.
- `SessionRetentionPolicy`: TTL, max checkpoint count, max history length,
  compaction window, and export retention.
- `SessionProviderCapability`: durability mode, checkpoint support, restore
  support, compaction support, max state/checkpoint size, replay support,
  health, and unavailable reasons.

## Rejected Boundary Leakage

Macaca must not expose these provider-native or application-specific shapes as
stable SDK/ABI contracts:

- Browser `Storage` objects, DOM event behavior, string-only storage semantics,
  origin/tab implementation details, or page-session lifecycle handles.
- Android `SavedStateHandle`, ViewModel lifecycle classes, Bundle internals, or
  activity/fragment state APIs.
- UIKit/SwiftUI restoration APIs, `NSUserActivity` objects, scene delegates, or
  UI hierarchy reconstruction rules.
- Temporal workflow command APIs, task queues, activity retries, workflow event
  history payloads, or task/review semantics.
- Redis keys, serialized session blobs, EXPIRE/TTL sentinel values, keyspace
  notifications, or session-store implementation handles.
- Workflow/task-board state repair, review retry rules, shell-owned recovery
  semantics, app-specific state keys, raw secrets, raw UI state dumps, or
  unbounded exports.

All operations must enter through typed Macaca service commands with trace
context, policy checks, resource limits, structured result envelopes, sanitized
audit events, unavailable provider behavior, replay evidence, and provider
replacement support.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
