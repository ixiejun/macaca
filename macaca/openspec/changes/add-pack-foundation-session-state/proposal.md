# Change: Add Foundation Session State Pack

## Why

Developers need `pack.foundation.session.state.v1` as a resumable,
session-scoped state capability. Applications need to save transient state,
record checkpoints, restore after process/server restart, inspect recovery
metadata, clear state, and replay session decisions without inventing
application-specific persistence and recovery semantics.

This pack is foundational for Macaca's 24/7 autonomous execution model. Session
state must bridge browser-like tab/session isolation, mobile process-death
recovery, app state restoration, and long-running workflow checkpoints while
remaining generic enough for YAML, WASM, GenUI, and headless applications.

## Supplier And Platform API Research

The proposal is derived from a capability-by-capability comparison of mature
state-restoration systems:

- Web `sessionStorage`: origin and top-level browsing-context partitioning, data
  kept for the duration of a page session, string key-value semantics, and
  session-scoped isolation.
- Android `SavedStateHandle` and save UI state guidance: key-value state retained
  across process death and restored into ViewModels without manual plumbing.
- Apple UIKit/SwiftUI state restoration and `NSUserActivity`: restoring app or
  scene state on subsequent launches through state restoration metadata.
- Temporal workflow event history and Continue-As-New: replayable history,
  checkpoint-like fresh executions, run ids, workflow ids, and state handoff for
  long-running executions.
- Server-side session stores such as Redis-backed sessions: TTL, session id,
  serialization, invalidation, and distributed recovery.

Macaca borrows the stable concepts, not provider APIs:

- session state is scoped by application, tenant, session, task, and trace;
- checkpoint identity and replay evidence are first-class;
- state payloads are bounded, typed, redacted, and versioned;
- long histories can be compacted through checkpoint/continue metadata;
- restore is explicit and policy-governed, never silent fake success.

## What Changes

- Define `pack.foundation.session.state.v1` as the canonical app-facing session
  state pack.
- Add an industrial command surface covering get/put/delete session state, merge
  patch, list keys, create checkpoint, list checkpoints, restore checkpoint,
  compare checkpoint, compact history, clear session, export redacted snapshot,
  and inspect recovery metadata.
- Define provider-neutral DTO requirements for session refs, state keys, typed
  values, revisions, checkpoint refs, restore plans, state schema version,
  redaction, retention, compaction, and replay metadata.
- Define permission scopes for read, write, delete, checkpoint, restore, compact,
  clear, export, and inspect recovery.
- Require a detailed developer guide under
  `docs/developer-packs/foundation/session-state.md` before this proposal can be
  marked complete.
- Keep implementation ownership in a session-state system service; kernel, SDK,
  shells, and application framework remain provider-neutral.

## Impact

- Affected specs: `pack-foundation-session-state`,
  `developer-pack-industrial-capability-catalog`, `sdk-system-facade`,
  `service-runtime`, `unified-execution-path`.
- Affected code later: provider-neutral DTOs, descriptor validators, application
  admission, SDK discovery, SDK command helpers, session-state service,
  mock/unavailable providers, trace/audit event schema, replay tests, recovery
  tests, and dependency-boundary gates.
- Non-goals: workflow planner state machines, task board semantics, raw UI state
  ownership by shell code, app-specific recovery rules, or direct database/session
  store handles in SDK.
