# Foundation Session State Pack Design

## Context

`pack.foundation.session.state.v1` provides session-scoped state and checkpoints
for Macaca applications. It exists between simple key-value state and full
workflow/task recovery: it stores bounded application/session state and recovery
metadata, while autonomy/workflow services own task planning, retries, review
loops, and recovery policy.

The pack must support YAML, WASM, GenUI, and headless applications through the
same service path. It must also support Macaca's restart/replay model without
letting shell code or application-specific logic repair state outside service
boundaries.

## Supplier API Comparison

| Source API family | Relevant concepts | Macaca abstraction |
| --- | --- | --- |
| Web `sessionStorage` | origin plus tab/session partition, page-session lifetime, key-value data | app/tenant/session partition, session lifetime, bounded session keys |
| Android `SavedStateHandle` | key-value state retained across process death, ViewModel restoration | restart-safe session state, typed values, restore into app runtime envelope |
| Apple state restoration / `NSUserActivity` | scene/app restoration metadata, subsequent launch restore | state restoration intent, activity/session metadata, redacted restore snapshots |
| Temporal event history / Continue-As-New | replayable event history, fresh run with latest state, workflow/run ids | checkpoint refs, replay evidence, compact/continue metadata, recovery chain |
| Redis/server session stores | session id, TTL, serialization, invalidation, distributed store | session refs, TTL/retention, clear/invalidate, provider health diagnostics |

Design conclusion: Macaca should expose generic session state, checkpoint, and
restore commands. It should not expose browser/mobile/workflow provider APIs or
make session state responsible for workflow semantics.

## Goals

- Provide get, put, delete, merge patch, list, checkpoint, list checkpoints,
  restore checkpoint, compare checkpoint, compact history, clear session,
  redacted export, and recovery metadata operations.
- Scope every value to tenant, app, session, optional task, and trace context.
- Support typed values, artifact references, secret references, and redacted
  snapshots.
- Support revisioned state and checkpoint ids for optimistic concurrency.
- Support retention, TTL, compaction, and restore dry-run diagnostics.
- Support mock, unavailable, embedded durable, remote session store, and
  replay-oriented providers through one contract.

## Non-Goals

- No task planner, review loop, workflow retry, or task board state machine.
- No UI shell-owned recovery repair rules.
- No unbounded app state dumps in traces/audits.
- No raw secrets in session state; use secret references only.
- No direct Redis/database/session-store provider handles in SDK.
- No application-specific keys or recovery workflows in OS code.

## Ownership And Boundaries

- Pack id: `pack.foundation.session.state.v1`.
- Family: `foundation`.
- Service owner: session-state system service.
- Provider examples: embedded durable provider, memory provider for tests,
  remote session store provider, replay provider, unavailable provider.
- SDK surface: `sdk.packs.foundation.sessionState`.
- Command namespace: `session_state.*`.
- Microkernel ownership: identity, policy facade, service-call evidence,
  session primitive ids, trace/audit primitives only.
- Application framework ownership: manifest declaration, session envelope,
  app-scoped permission declarations, effective capability projection, WASM ABI
  import exposure.
- Runtime-host ownership: provider registration, state serialization bridge,
  decorators, snapshots, and unavailable provider composition.

## Command Surface

| Command | Supplier analogs | DTO notes | Side effects |
| --- | --- | --- | --- |
| `session_state.get` | sessionStorage get, SavedStateHandle get | session ref, key, revision projection | No |
| `session_state.put` | sessionStorage set, SavedStateHandle set | key, typed value, expected revision, ttl | Yes |
| `session_state.delete` | sessionStorage remove | key, expected revision, tombstone | Yes |
| `session_state.merge_patch` | state update/patch | object patch, expected revision, schema version | Yes |
| `session_state.list_keys` | session keys | prefix, page token, metadata projection | No |
| `session_state.create_checkpoint` | state restoration snapshot, Continue-As-New handoff | label, reason, retention, redaction, schema version | Records checkpoint |
| `session_state.list_checkpoints` | checkpoint/history listing | page token, time range, state hash projection | No |
| `session_state.restore_checkpoint` | app restore / workflow continue | checkpoint id, target session, conflict mode, dry-run | Yes |
| `session_state.compare_checkpoint` | diff restore state | checkpoint id, current revision, redaction | No |
| `session_state.compact_history` | Temporal history management | retention revision/time, checkpoint anchor, dry-run | Yes |
| `session_state.clear_session` | session invalidation | session ref, reason, tombstone, approval | Yes |
| `session_state.export_redacted` | diagnostic export | checkpoint/session ref, redaction, format | No |
| `session_state.inspect_recovery` | recovery metadata | session ref, last checkpoint, provider health, replay refs | No |

## DTO Model

Core DTOs:

- `SessionStateRef`: tenant id, app id, session id, optional task id, runtime id,
  and trace binding.
- `SessionStateKeyRef`: normalized key, prefix policy, schema field, redaction
  label.
- `SessionStateValue`: typed primitive, JSON object, bounded bytes reference,
  artifact reference, or secret reference. Raw secrets are forbidden.
- `SessionStateRevision`: opaque revision id, checkpoint sequence, provider
  revision, and trace binding.
- `SessionCheckpointRef`: checkpoint id, session id, state hash, schema version,
  retention policy, creation reason, and replay refs.
- `SessionRestorePlan`: source checkpoint, target session, conflict mode,
  dry-run result, expected current revision.
- `SessionRecoveryMetadata`: last known checkpoint, provider health, compaction
  state, replay refs, unavailable reasons.
- `SessionStateError`: denied, not_found, conflict, invalid_session,
  invalid_key, invalid_checkpoint, schema_mismatch, quota_exceeded, too_large,
  unsupported, unavailable, provider_failure.

## Permission And Policy Model

Permission scopes:

- `session_state.read`
- `session_state.write`
- `session_state.delete`
- `session_state.list`
- `session_state.checkpoint`
- `session_state.restore`
- `session_state.compact`
- `session_state.clear`
- `session_state.export`
- `session_state.inspect_recovery`

Policy rules:

- Every command is scoped to tenant id, application id, session id, task id,
  runtime id, key/prefix, checkpoint id, and trace id when available.
- Session state can outlive process restarts but must obey retention and TTL.
- Restore, clear, compact, and broad export require side-effect policy and may
  require approval.
- Raw secret values are rejected; secret references require secret-reference
  interoperability and policy.
- Checkpoint snapshots must be bounded, redacted, versioned, and replayable.
- Restore must support dry-run and conflict modes rather than silently
  overwriting live state.

## SDK And Developer Documentation

SDK discovery returns command schemas, value types, checkpoint support, retention
support, provider availability, permission scopes, policy templates, health,
examples, docs link, and unavailable diagnostics.

Required developer guide:

- Path: `docs/developer-packs/foundation/session-state.md`.
- Content: session scope, state versus workflow boundary, manifest declaration,
  key/value model, revisions, checkpoints, restore, compaction, clear, retention,
  redaction, permissions, policy, unavailable diagnostics, provider replacement,
  trace/audit fields, and examples.
- Examples: save transient form state, create checkpoint, restore dry-run, handle
  schema mismatch, compact history, denied clear session, unavailable provider,
  and WASM host import usage.

## Trace, Audit, Health, Snapshot, And Replay

Required event names:

- `session_state_pack_declared`
- `session_state_pack_admission_validated`
- `session_state_pack_policy_decision`
- `session_state_pack_service_call_requested`
- `session_state_pack_service_call_succeeded`
- `session_state_pack_service_call_failed`
- `session_state_pack_checkpoint_created`
- `session_state_pack_restore_requested`
- `session_state_pack_restore_completed`
- `session_state_pack_history_compacted`
- `session_state_pack_session_cleared`
- `session_state_pack_unavailable`

Events include pack id, service id, command name, trace id, app/session/task
identifiers, key/prefix hash, checkpoint id, state hash, schema version, policy
decision, provider class, latency, bounded resource counters, and bounded error
code. Events must not include raw state values, raw secrets, raw provider
payloads, prompts, manifests, package bytes, credentials, private keys, or
unbounded output.

Health checks include provider registered state, durability mode, checkpoint
support, restore support, compaction support, max state size, max checkpoint
size, retention support, replay support, and unavailable reasons.

Snapshots include descriptor version, provider class, session id hash, key
count, checkpoint summaries, last state hash, policy template hash, retention
metadata, compaction state, and sanitized replay references.

## Implementation Slices

1. Contract slice: descriptor, command schemas, state/checkpoint/restore DTOs,
   result/error DTOs, health/snapshot DTOs, provider capability report.
2. Admission slice: session declarations, required/optional behavior, permission
   validation, retention policy, checkpoint/restore capability validation.
3. Service slice: session-state service trait/provider interface, unavailable
   provider, deterministic mock provider, embedded durable provider, replay
   provider, remote session store bridge.
4. SDK slice: discovery, typed command builders, checkpoint helper, restore
   dry-run helper, compaction helper, unavailable diagnostics, docs link.
5. WASM/app-runtime slice: expose only declared callable session-state imports
   through service runtime; no direct provider handles.
6. Observability slice: trace/audit events, redaction, replay tests, restart
   recovery tests, health snapshots.
7. Developer-docs slice: complete
   `docs/developer-packs/foundation/session-state.md` and link it from catalog
   metadata.

## Design Patterns

- **Facade**: SDK exposes session-state helpers and command builders only.
- **Command**: every operation is a typed command/result.
- **Adapter/Bridge**: web-like, mobile-like, workflow-like, embedded, remote,
  mock, and unavailable providers adapt to one contract.
- **Strategy**: provider selection, retention, conflict handling, restore,
  compaction, and unavailable behavior are replaceable.
- **Decorator**: trace, policy, resource, redaction, retention, and audit wrap
  every command.
- **Specification**: session scope, schema version, checkpoint, restore, and
  retention rules are executable validators.
- **Memento**: checkpoints and effective capability reports preserve replay
  state.

## Risks And Mitigations

- Risk: session state grows into workflow orchestration.
  Mitigation: task planning/retry/review/recovery semantics remain in workflow
  and autonomy services.
- Risk: checkpoints leak user or provider payloads.
  Mitigation: redaction policy, bounded values, artifact refs, secret refs, and
  audit leakage tests.
- Risk: restore overwrites live state unexpectedly.
  Mitigation: dry-run restore, conflict modes, expected revisions, and approval.
- Risk: long histories become unbounded.
  Mitigation: compaction command, checkpoint anchors, retention limits, and
  provider health diagnostics.
- Risk: shell code repairs session state directly.
  Mitigation: shells render diagnostics only and call SDK/service commands for
  restore, clear, and recovery inspection.
