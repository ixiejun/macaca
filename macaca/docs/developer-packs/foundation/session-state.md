# Foundation Session State Pack

`pack.foundation.session.state.v1` defines provider-neutral session-scoped
state for Macaca applications. It covers bounded values, revisions,
checkpoints, restore plans, compaction, redacted export, and recovery metadata
without making shells or applications own durable recovery semantics.

## Manifest Declaration

Declare the pack in an application service contract:

```yaml
service_contract:
  optional_packs:
    - pack.foundation.session.state.v1
```

Use `required_packs` only when the application cannot run without a registered
session-state provider. If no provider is installed, admission and SDK
discovery return `session_state_provider_not_installed`; they do not create an
implicit store or fake successful state recovery.

## State Boundary

Session state stores transient application/session data and recovery metadata.
Workflow planning, task board transitions, review retry, worker assignment, and
autonomy recovery remain owned by task and workflow services. Shells render
diagnostics and submit typed commands; they must not repair session state
directly.

All values are scoped by tenant, application, session, optional task, and trace
context through the service runtime. Raw secrets are forbidden. Store secret
references from `pack.foundation.secrets.reference.v1` when state needs to point
at secret-classified material.

## Permissions

The pack defines these provider-neutral scopes:

- `session_state.read`: read values and recovery metadata.
- `session_state.write`: put and merge state values.
- `session_state.delete`: delete individual keys.
- `session_state.list`: list keys and checkpoints.
- `session_state.checkpoint`: create checkpoint references.
- `session_state.restore`: restore or dry-run checkpoint restore plans.
- `session_state.compact`: compact bounded state history.
- `session_state.clear`: clear an entire session after policy approval.
- `session_state.export`: export redacted diagnostic snapshots.
- `session_state.inspect_recovery`: inspect provider and recovery status.

## Commands

- `session_state.get`: read one key by session and key reference.
- `session_state.put`: write a bounded value reference with an optional expected
  revision.
- `session_state.delete`: remove one key with optimistic revision checks.
- `session_state.merge_patch`: apply a bounded JSON merge-patch artifact.
- `session_state.list_keys`: list keys by prefix with paging.
- `session_state.create_checkpoint`: create a checkpoint reference under a
  retention policy.
- `session_state.list_checkpoints`: page checkpoint references for a session.
- `session_state.restore_checkpoint`: execute or dry-run a restore plan.
- `session_state.compare_checkpoint`: compare two checkpoint references without
  exposing raw state values.
- `session_state.compact_history`: compact history before a revision anchor.
- `session_state.clear_session`: dry-run or clear all state for a session.
- `session_state.export_redacted`: export redacted diagnostics.
- `session_state.inspect_recovery`: inspect latest checkpoint, provider health,
  replay references, and unavailable reasons.

## DTO Guidance

Use `SessionStateSessionRef` to bind every command to a session and optional
task. Use `SessionStateKeyRef` for normalized keys and prefixes. Use
`SessionStateValueRef` for opaque artifact or value references; never place raw
payloads in trace or audit records. Use `SessionStateRevision` and
`SessionStateCheckpointRef` for optimistic concurrency and replay evidence.

`SessionStateRestorePlan` supports dry-run restore before mutation. Cross
session restore must remain policy-gated. `SessionStateRetentionPolicy`
controls TTL, checkpoint count, and compaction thresholds. Redacted exports
summarize counts and hashes rather than values.

## Result And Error DTOs

All commands return a bounded result envelope with status, optional data,
optional error, trace id, and descriptor hash. Standard statuses are `success`,
`partial_page`, `denied`, `not_found`, `conflict`, `invalid_session`,
`invalid_key`, `invalid_checkpoint`, `schema_mismatch`, `quota_exceeded`,
`too_large`, `unsupported`, `unavailable`, and `provider_failure`.

Unavailable diagnostics are structured:

```json
{
  "status": "unavailable",
  "error": {
    "code": "unavailable",
    "message": "session-state provider is not installed",
    "retryable": false
  }
}
```

## Examples

Save transient form state:

```json
{
  "key": {
    "session": { "session_id": "session-ref", "task_id": "task-ref" },
    "key": "draft.form"
  },
  "value": {
    "value_ref": "artifact:bounded-form-state",
    "schema_id": "form.schema.v1",
    "secret_reference_required": false
  },
  "expected_revision": null
}
```

Create a checkpoint:

```json
{
  "session": { "session_id": "session-ref", "task_id": "task-ref" },
  "retention": {
    "ttl_seconds": 3600,
    "max_checkpoints": 8,
    "compact_after_revisions": 50
  }
}
```

Restore dry-run:

```json
{
  "plan": {
    "checkpoint": {
      "checkpoint_id": "checkpoint-1",
      "session": { "session_id": "session-ref", "task_id": "task-ref" },
      "revision_id": "rev-42"
    },
    "dry_run": true,
    "cross_session_allowed": false
  }
}
```

Schema mismatch handling should return `schema_mismatch` with a sanitized
message and no raw state value. Compaction should be submitted with `dry_run`
first, then retried only after policy approves the mutation. Clear-session
commands should expect `denied` unless policy grants the destructive action.

WASM applications may receive host imports only for declared callable
`session_state.*` commands. Each import must route through the same traced
service-runtime call path as YAML, GenUI, and headless applications.

## Provider Replacement

Expected provider classes include `embedded`, `remote-session-store`, `replay`,
`mock`, and `unavailable`. Providers must expose descriptor metadata, command
support, retention limits, checkpoint and restore support, health, snapshots,
redacted diagnostics, and structured unavailable behavior. SDKs, shells, kernel
code, and applications must not instantiate provider stores directly.
