# Foundation Key-Value State Pack Design

## Context

`pack.foundation.key.value.state.v1` provides namespaced key-value state for
Macaca applications. It must support app preferences, session-local state,
workflow checkpoints, cached provider metadata, small coordination tokens, and
pack configuration without exposing a concrete database or business workflow.

The pack is not a replacement for relational databases, document stores, vector
stores, or application-owned domain storage. It is the OS-level primitive for
small, auditable, resumable state where typed commands, policy, trace, replay,
and provider replacement matter more than provider-native API breadth.

## Supplier API Comparison

| Source API family | Relevant concepts | Macaca abstraction |
| --- | --- | --- |
| Redis | `GET`, `SET`, `DEL`, `MGET`, `MSET`, `INCR`, `EXPIRE`, `TTL`, transactions, `WATCH`, scan | Basic commands, batch commands, atomic counters, TTL/lease, CAS, prefix listing, bounded watch streams |
| etcd / Consul KV | revision, compare-and-swap, leases, watch, prefix query, compaction, cluster health | Revisioned values, compare-and-set, lease refs, namespace watch, consistency options, compact command, health diagnostics |
| Apple UserDefaults / iCloud KVS | app-scoped preference keys, limited value types, sync, quota, conflict | App namespace, typed primitive values, sync capability flags, quota diagnostics, conflict result DTOs |
| Android SharedPreferences / DataStore | typed preference keys, transactional update, observation flow, migration | typed value schema, atomic update command, watch stream, migrate namespace command |
| Web Storage / IndexedDB | origin scope, async transactions, object stores, quota, versioning | tenant/app/session namespace, transaction id/idempotency key, quota errors, schema version metadata |

Design conclusion: Macaca should provide a compact state contract with explicit
provider capability reports. It should not expose Redis commands, etcd clients,
browser storage APIs, or mobile preference APIs directly.

## Goals

- Provide namespaced get, put, delete, exists, batch, prefix list, compare-and-set,
  increment, TTL/lease, watch, snapshot, restore, migrate, and compact operations.
- Provide typed values: null, bool, integer, float, string, JSON object, bytes
  reference, secret reference, and artifact reference.
- Preserve revision metadata for optimistic concurrency and replay.
- Support bounded prefix scans and watch streams.
- Support TTL expiration and lease diagnostics when the provider supports them,
  and return `unsupported` when it does not.
- Support mock, unavailable, embedded, remote KV, browser-like, and in-memory
  providers through one contract.

## Non-Goals

- No SQL query language, document query language, graph query language, vector
  search, or full-text indexing.
- No raw Redis/etcd/IndexedDB provider handles in SDK or app ABI.
- No direct secret values in state; store secret references only.
- No application-specific state-machine transitions.
- No unbounded namespace export, list, watch, or snapshot content in diagnostics.
- No prompt, manifest, package bytes, credentials, private keys, raw provider
  payloads, or unbounded values in logs/traces.

## Ownership And Boundaries

- Pack id: `pack.foundation.key.value.state.v1`.
- Family: `foundation`.
- Service owner: key-value state system service.
- Provider examples: embedded local provider, durable workspace provider, Redis
  adapter, etcd/Consul adapter, browser-like provider, in-memory mock provider,
  unavailable provider.
- SDK surface: `sdk.packs.foundation.keyValueState`.
- Command namespace: `kv.*`.
- Microkernel ownership: identity, service-call evidence, policy facade,
  resource primitives, trace/audit primitives only.
- Application framework ownership: manifest declarations, app-scoped permission
  declarations, effective capability projection, WASM ABI import exposure.
- Runtime-host ownership: provider registration, decorators, connection
  lifecycle, unavailable provider composition, and sanitized diagnostics.

## Command Surface

| Command | Supplier analogs | DTO notes | Side effects |
| --- | --- | --- | --- |
| `kv.get` | Redis `GET`, UserDefaults read, DataStore read | namespace, key, consistency, projection | No |
| `kv.put` | Redis `SET`, DataStore update | namespace, key, value, expected revision, ttl, idempotency key | Yes |
| `kv.delete` | Redis `DEL`, preference remove | namespace, key, expected revision, tombstone flag | Yes |
| `kv.exists` | Redis `EXISTS` | namespace, key, consistency | No |
| `kv.batch_get` | Redis `MGET` | key list, max keys, projection | No |
| `kv.batch_put` | Redis `MSET`, transaction update | entries, atomic preference, idempotency key | Yes |
| `kv.batch_delete` | batch delete / transaction | keys, expected revisions, tombstone flag | Yes |
| `kv.list_keys` | Redis `SCAN`, etcd prefix | prefix, page token, max keys, metadata projection | No |
| `kv.compare_and_set` | Redis `WATCH`/transaction, etcd CAS | expected revision/value, new value, ttl, conflict result | Yes |
| `kv.increment` | Redis `INCR` | integer key, delta, bounds, expected revision | Yes |
| `kv.set_ttl` | Redis `EXPIRE`, leases | ttl, lease ref, persist flag | Yes |
| `kv.get_ttl` | Redis `TTL` | key, lease metadata | No |
| `kv.watch_namespace` | etcd watch, DataStore flow | prefix, event filter, start revision, stream budget | Starts stream |
| `kv.snapshot_namespace` | backup/export | namespace, prefix filter, retention, redaction policy | Records snapshot |
| `kv.restore_namespace` | restore/import | snapshot id, target namespace, conflict mode, dry-run flag | Yes |
| `kv.migrate_namespace` | storage migration | source namespace, target namespace, mapping, validation mode | Yes |
| `kv.compact_namespace` | etcd compaction | namespace, retention revision/time, dry-run flag | Yes |

## DTO Model

Core DTOs:

- `KvNamespaceRef`: tenant id, app id, session id, pack id, and optional logical
  namespace segment.
- `KvKeyRef`: normalized key string, prefix policy, length bound, and redaction
  label. Provider-native key syntax is rejected at the SDK boundary.
- `KvValue`: typed primitive, JSON value, bounded bytes, artifact reference, or
  secret reference. Raw secrets are forbidden.
- `KvRevision`: opaque revision id, monotonic provider revision when available,
  version vector when distributed providers need it, and trace binding.
- `KvTtlPolicy`: no ttl, ttl duration, absolute expiration, lease ref, persist.
- `KvConsistency`: best_effort, read_your_writes, strong_when_supported.
- `KvConflictMode`: fail, overwrite, compare_revision, compare_value, merge_json.
- `KvWatchEvent`: put, delete, expire, compacted, provider_unavailable,
  stream_checkpoint.
- `KvError`: denied, not_found, already_exists, conflict, invalid_key,
  invalid_namespace, quota_exceeded, too_large, unsupported, expired,
  compacted_revision, unavailable, provider_failure.

## Permission And Policy Model

Permission scopes:

- `state.read`
- `state.write`
- `state.delete`
- `state.list`
- `state.watch`
- `state.ttl`
- `state.counter`
- `state.snapshot`
- `state.restore`
- `state.migrate`
- `state.compact`

Policy rules:

- Every command is scoped to tenant id, application id, session id, task id,
  namespace, key/prefix, and trace id when available.
- Read/list operations require declared namespace access and max result bounds.
- Write/delete/batch/restore/migrate/compact operations require side-effect
  policy and resource reservation before provider calls.
- Restore, migrate, compact, namespace-wide delete, and large batch mutation
  require approval unless policy explicitly marks the namespace automation-safe.
- TTL commands must report unsupported behavior rather than emulate expiration
  silently when the provider cannot guarantee cleanup.
- Watch commands require stream budget, timeout, cancellation, and backpressure.
- Secret values are forbidden; secret references are allowed only when policy
  permits `secrets.reference` interoperability.

## SDK And Developer Documentation

SDK discovery returns command schemas, namespace rules, value types, permission
scopes, policy templates, provider availability, consistency/TTL/watch support,
health, examples, docs link, and unavailable diagnostics.

Required developer guide:

- Path: `docs/developer-packs/foundation/key-value-state.md`.
- Content: manifest declarations, namespace model, key model, value model,
  permission scopes, consistency, CAS, TTL, watch streams, snapshots, restore,
  migration, compaction, result/error DTOs, unavailable diagnostics, provider
  replacement, trace/audit fields, and security guidance.
- Examples: app preference get/put, CAS update loop, TTL cache entry, bounded
  prefix scan, watch cancellation, snapshot/restore dry run, unavailable provider
  diagnostics, and denied namespace-wide delete.

## Trace, Audit, Health, Snapshot, And Replay

Required event names:

- `kv_pack_declared`
- `kv_pack_admission_validated`
- `kv_pack_policy_decision`
- `kv_pack_service_call_requested`
- `kv_pack_service_call_succeeded`
- `kv_pack_service_call_failed`
- `kv_pack_watch_started`
- `kv_pack_watch_stopped`
- `kv_pack_snapshot_recorded`
- `kv_pack_restore_requested`
- `kv_pack_namespace_migrated`
- `kv_pack_namespace_compacted`
- `kv_pack_unavailable`

Events include pack id, service id, command name, trace id, tenant/app/session
identifiers, namespace hash, key hash or prefix hash, policy decision, provider
class, consistency level, revision, ttl presence, result counters, latency,
bounded resource counters, and bounded error code. Events must not include raw
values, raw secrets, raw provider payloads, or unbounded key listings.

Health checks include provider registered state, durability mode, max key size,
max value size, max namespace size, batch limits, TTL support, watch support,
snapshot support, restore support, consistency support, compaction state, and
unavailable reasons.

Snapshots include descriptor version, provider class, namespace metadata, key
count, retained revision range, policy template hash, resource counters, and
sanitized replay references. Snapshot content is stored as provider-managed
artifact references, not embedded in audit logs.

## Implementation Slices

1. Contract slice: descriptor, command schemas, shared DTOs, result/error DTOs,
   provider capability report, stable hashes.
2. Admission slice: namespace declarations, required/optional pack behavior,
   permissions, lifecycle, service mapping, provider capability validation.
3. Service slice: KV service trait/provider interface, unavailable provider,
   mock provider, embedded durable provider, optional external adapter bridge.
4. SDK slice: discovery, typed command builders, CAS helper, watch helper,
   snapshot/restore helper, unavailable diagnostics, docs link.
5. WASM/app-runtime slice: expose only declared callable KV imports through
   service runtime; no raw provider/database handles.
6. Observability slice: trace/audit events, redaction, replay tests, health
   snapshots, stream cancellation.
7. Developer-docs slice: complete
   `docs/developer-packs/foundation/key-value-state.md` and link it from catalog
   metadata.

## Design Patterns

- **Facade**: SDK exposes discovery and command builders only.
- **Command**: every operation is a typed command/result.
- **Adapter/Bridge**: Redis-like, etcd-like, preference-store-like, IndexedDB-like,
  embedded, remote, mock, and unavailable providers adapt to one contract.
- **Strategy**: provider selection, consistency behavior, conflict handling, TTL
  behavior, and unavailable behavior are replaceable.
- **Decorator**: policy, trace, resource, entitlement, approval, metering, and
  redaction wrap every call.
- **Specification**: namespace declarations, key rules, command schemas,
  permission scopes, and provider capability requirements are executable
  validators.
- **Observer**: watch streams, audit events, health changes, and service-call
  events are subscribable.
- **Memento**: snapshots and effective capability reports are replayable.

## Risks And Mitigations

- Risk: KV pack becomes a hidden app database.
  Mitigation: bound value size, namespace quotas, no query language, explicit
  non-goals, and provider capability diagnostics.
- Risk: secrets leak through state values.
  Mitigation: forbid raw secret values and allow only secret references with
  separate secret-reference policy.
- Risk: provider-native transaction semantics leak into SDK.
  Mitigation: expose provider-neutral CAS, batch, idempotency, and conflict DTOs.
- Risk: watch streams become unbounded or unrecoverable.
  Mitigation: stream budgets, start revision, checkpoints, cancellation, timeout,
  and compacted-revision diagnostics.
- Risk: snapshot/restore corrupts state.
  Mitigation: dry-run restore, conflict modes, revision checks, approval policy,
  and replayable snapshot references.
