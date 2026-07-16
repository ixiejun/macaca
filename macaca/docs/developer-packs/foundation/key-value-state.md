# Foundation Key-Value State Pack

`pack.foundation.key.value.state.v1` defines namespace-scoped key-value state
for Macaca applications. It covers typed values by reference, revisions,
compare-and-set, TTL, bounded prefix scans, watches, namespace snapshots,
restore dry-runs, migration, compaction, and unavailable diagnostics without
exposing provider-native database APIs.

## Manifest Declaration

Declare the pack in an application service contract:

```yaml
service_contract:
  optional_packs:
    - pack.foundation.key.value.state.v1
```

Use `required_packs` only when the application cannot run without a registered
state provider. If no provider is installed, discovery returns
`key_value_state_provider_not_installed`; it does not create an implicit store
or fake successful mutations.

## Namespace And Key Model

`KeyValueNamespaceRef` scopes keys to application, tenant, session, or another
admitted namespace policy. `KeyValueKeyRef` identifies one normalized key inside
that namespace. Raw secrets are forbidden; values that point at secret-classified
material must use `pack.foundation.secrets.reference.v1`.

Values use `KeyValueTypedValueRef` so raw data stays out of traces, audits, and
diagnostics. Revisions use `KeyValueRevision` for optimistic concurrency and
replay evidence. TTL uses `KeyValueTtlPolicy` with either relative expiry or an
absolute expiry timestamp.

## Permissions

- `state.read`: get and exists operations.
- `state.write`: put and compare-and-set operations.
- `state.delete`: delete and batch delete operations.
- `state.list`: bounded key listing.
- `state.watch`: namespace watch streams.
- `state.ttl`: set and read TTL metadata.
- `state.counter`: numeric increment operations.
- `state.snapshot`: create namespace snapshots.
- `state.restore`: restore snapshots, normally after dry-run approval.
- `state.migrate`: namespace migration.
- `state.compact`: compaction before a revision anchor.

## Commands

- `kv.get`
- `kv.put`
- `kv.delete`
- `kv.exists`
- `kv.batch_get`
- `kv.batch_put`
- `kv.batch_delete`
- `kv.list_keys`
- `kv.compare_and_set`
- `kv.increment`
- `kv.set_ttl`
- `kv.get_ttl`
- `kv.watch_namespace`
- `kv.snapshot_namespace`
- `kv.restore_namespace`
- `kv.migrate_namespace`
- `kv.compact_namespace`

Mutating and namespace-wide commands require policy and resource checks before
provider calls. Restore, migration, compaction, namespace-wide delete, and large
batch mutations may require approval.

## Result And Error DTOs

Commands return a bounded result envelope with status, optional data, optional
error, trace id, and descriptor hash. Standard statuses are `success`,
`partial_page`, `watch_checkpoint`, `denied`, `not_found`, `already_exists`,
`conflict`, `invalid_key`, `invalid_namespace`, `quota_exceeded`, `too_large`,
`unsupported`, `expired`, `compacted_revision`, `unavailable`, and
`provider_failure`.

Unavailable diagnostic example:

```json
{
  "status": "unavailable",
  "error": {
    "code": "unavailable",
    "message": "key-value state provider is not installed",
    "retryable": false
  }
}
```

## Examples

Preference get:

```json
{
  "key": {
    "namespace": { "namespace": "preferences", "tenant_ref": "tenant-ref" },
    "key": "ui.theme"
  },
  "consistency": "session"
}
```

Preference put with TTL:

```json
{
  "key": {
    "namespace": { "namespace": "preferences", "tenant_ref": "tenant-ref" },
    "key": "ui.theme"
  },
  "value": {
    "value_ref": "artifact:bounded-value",
    "value_kind": "json",
    "schema_id": "preference.schema.v1",
    "secret_reference_required": false
  },
  "ttl": { "ttl_seconds": 3600, "expire_at_epoch_millis": null },
  "conflict_mode": "compare_revision"
}
```

Bounded prefix scan:

```json
{
  "namespace": { "namespace": "preferences", "tenant_ref": "tenant-ref" },
  "prefix": "ui.",
  "page_size": 100,
  "cursor": null
}
```

Watch stream cancellation must be handled by the caller and service runtime.
Snapshot/restore should run in dry-run mode before mutation. Namespace-wide
delete, restore, migration, and compaction should expect `denied` unless policy
approves the side effect.

## Provider Replacement

Expected provider classes include `embedded-durable`, `remote-kv`,
`lease-consensus`, `mock`, and `unavailable`. Providers must expose descriptor
metadata, command support, TTL support, watch support, snapshot support,
compaction support, health, snapshots, unavailable diagnostics, and sanitized
audit data. SDKs, shells, kernel code, and applications must not instantiate
provider stores directly.
