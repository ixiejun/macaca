# Foundation Key-Value State Pack Research

## Purpose

This note records supplier/API research for `pack.foundation.key.value.state.v1`.
The pack must provide a Macaca-owned state contract for application-scoped,
session-scoped, and tenant-scoped key-value state without exposing Redis, etcd,
Consul, Apple, Android, or browser-native APIs as the stable developer surface.

## Source Baseline

- Redis command documentation:
  <https://redis.io/docs/latest/commands/>
- Redis `SET`:
  <https://redis.io/docs/latest/commands/set/>
- Redis `INCR`:
  <https://redis.io/docs/latest/commands/incr/>
- Redis `TTL`:
  <https://redis.io/docs/latest/commands/ttl/>
- etcd v3 API overview:
  <https://etcd.io/docs/v3.6/learning/api/>
- etcd API guarantees:
  <https://etcd.io/docs/v3.5/learning/api_guarantees/>
- Consul KV HTTP API:
  <https://developer.hashicorp.com/consul/api-docs/kv>
- Consul blocking queries:
  <https://developer.hashicorp.com/consul/api-docs/features/blocking>
- Apple `UserDefaults`:
  <https://developer.apple.com/documentation/foundation/userdefaults>
- Apple synchronizing app preferences with iCloud:
  <https://developer.apple.com/documentation/Foundation/synchronizing-app-preferences-with-icloud>
- Android SharedPreferences guide:
  <https://developer.android.com/training/data-storage/shared-preferences>
- Android SharedPreferences API reference:
  <https://developer.android.com/reference/android/content/SharedPreferences>
- Jetpack DataStore guide:
  <https://developer.android.com/topic/libraries/architecture/datastore>
- MDN Web Storage API:
  <https://developer.mozilla.org/en-US/docs/Web/API/Web_Storage_API>
- MDN storage quotas and eviction:
  <https://developer.mozilla.org/en-US/docs/Web/API/Storage_API/Storage_quotas_and_eviction_criteria>
- MDN IndexedDB API:
  <https://developer.mozilla.org/en-US/docs/Web/API/IndexedDB_API>
- W3C Indexed Database API 3.0:
  <https://www.w3.org/TR/IndexedDB/>

## Redis Summary

Redis contributes the common operational vocabulary for fast key-value state:
single-key get/set/delete, counters, TTL, batch/pipeline-style use, optimistic
transactions, scans, publish/subscribe, streams, and persistence. Macaca should
borrow these concepts but not Redis command shapes:

- `GET`, `SET`, and deletion map to `kv.get`, `kv.put`, and `kv.delete`.
- `INCR`-style counters require typed numeric values, overflow behavior,
  idempotency metadata, and `state.counter` permission.
- TTL commands require explicit `KvTtlPolicy`, expiry diagnostics, and
  `expired` result states instead of Redis-native sentinel return values.
- `WATCH`/transaction concepts map to compare-and-set and revision-checked
  batch operations. They must expose conflict results, not Redis transaction
  arrays or wire-level command replies.
- Scan and stream/pub-sub concepts map to bounded key pagination and watch
  leases. The service must enforce page limits, stream budgets, backpressure,
  and cancellation cleanup.
- Persistence should be provider capability metadata. Macaca does not promise
  Redis RDB/AOF semantics in its stable API.

## etcd / Consul Summary

etcd and Consul establish the distributed-state vocabulary: revisions, compare
conditions, leases, watches, prefix queries, compaction, blocking queries, and
health. Macaca should borrow the following:

- Revisions are first-class conflict-control evidence. Every mutable command
  should be able to carry an expected revision or conflict mode.
- Compare-and-set is a generic command family rather than a provider-specific
  transaction DSL.
- Leases and TTLs are lifecycle policies tied to keys or namespaces. Expiry,
  renewal, and lease loss need structured diagnostics.
- Watches are resumable leases with checkpoints. Compacted revisions become
  `compacted_revision` results with a replay pointer and recovery guidance.
- Prefix queries are bounded list operations with cursor, page size, consistency
  level, and redaction policy.
- Compaction is a namespace operation that needs approval, retention policy,
  and resource accounting.
- Health is a service snapshot with provider class, revision watermark,
  compaction watermark, lease/watch support, persistence support, and degraded
  or unavailable reasons.

## Apple UserDefaults / iCloud KVS Summary

Apple preferences APIs provide a local app-scoped preference model and an
iCloud-synchronized KVS model for small app preference data:

- App scope is central. Macaca namespaces must be app/tenant/session/task scoped
  and must not expose cross-application ambient defaults.
- Values are small typed preference-like data, not general documents or secrets.
- iCloud-style synchronization introduces quota, propagation delay, conflict,
  and remote-change diagnostics. Macaca should expose sync state as freshness,
  attribution, and conflict metadata rather than pretending every provider is
  strongly consistent.
- The API is not a secret vault. Sensitive values should use the secrets
  reference pack, not KV state.

## Android SharedPreferences / Jetpack DataStore Summary

Android provides both legacy simple preferences and modern coroutine/Flow-based
DataStore:

- SharedPreferences is suitable for small key-value pairs but has legacy
  consistency and threading limitations. Macaca should model it only as a
  simple provider class, not as the contract baseline.
- DataStore contributes typed value schemas, asynchronous consistency,
  transactional updates, observable flows, and migration support.
- Migration from older stores is a first-class lifecycle action. Macaca should
  expose `kv.migrate_namespace` with planning, approval, dry-run diagnostics,
  resource budget, and rollback/replay evidence.
- Observable changes map to watch namespace leases, not Android-specific Flow
  objects in the SDK ABI.

## Web Storage / IndexedDB Summary

Browser storage APIs show two ends of the local persistence spectrum:
synchronous small string key-value storage and asynchronous transactional
indexed storage.

- Web Storage is origin-scoped, string-based, quota-limited, and synchronous.
  Macaca should borrow origin/app scope and quota diagnostics, but not expose
  blocking browser storage behavior.
- IndexedDB provides async transactions, schema versions, object stores,
  indexes, and persistent records. Macaca should borrow transaction/versioning
  concepts for provider capability and migration metadata, but keep the stable
  pack as key-value state rather than a browser database API.
- Quota and eviction need explicit result envelopes such as `quota_exceeded`,
  `unavailable`, `too_large`, and `provider_failure`.
- Async persistence maps to canonical service commands and bounded result
  pages, never direct browser callbacks or DOM request objects.

## Macaca-Owned Abstractions

`pack.foundation.key.value.state.v1` should define these provider-neutral
concepts:

- `KvNamespaceRef`: app/tenant/session/task scoped namespace id, retention
  class, consistency class, provider capability class, and redaction profile.
- `KvKeyRef`: normalized key, namespace ref, prefix class, redaction class,
  max-size evidence, and validation result.
- `KvTypedValue`: typed scalar, bytes reference, JSON-like bounded value,
  encrypted/reference-only marker, size class, schema reference, and redaction
  class.
- `KvRevision`: monotonic or provider-derived revision watermark with provider
  attribution and comparison semantics.
- `KvTtlPolicy`: expiry timestamp/duration, renewal policy, lease reference,
  stale/expired behavior, and provider support diagnostics.
- `KvConsistencyLevel`: best-effort local, read-your-writes, strong,
  revision-bound, provider-default, and unsupported.
- `KvConflictMode`: require absent, require present, require revision, overwrite,
  merge-provider-supported, and reject.
- `KvWatchLease`: namespace/prefix scope, checkpoint, max duration, event budget,
  compaction recovery policy, backpressure, and cleanup state.
- `KvSnapshotRef`: bounded namespace snapshot handle, checksum/hash, retention,
  restore policy, and replay pointer.
- `KvProviderCapability`: supported commands, namespace model, max key/value
  sizes, TTL support, CAS support, watch support, snapshot/restore support,
  migration support, compaction support, persistence class, health, and
  unavailable reasons.

## Rejected API Leakage

Macaca must not expose these provider-native shapes as stable SDK/ABI contracts:

- Redis command names, RESP replies, Lua scripts, pub/sub channels, stream IDs,
  RDB/AOF persistence options, or sentinel return values.
- etcd gRPC request/response types, raw revision implementation details,
  transaction DSLs, watch protocol frames, or compaction internals.
- Consul HTTP query parameters, `X-Consul-Index` headers, raw session locks,
  blocking-query mechanics, or datacenter-specific API shapes.
- Apple `UserDefaults`/`NSUbiquitousKeyValueStore` object APIs, bundle-default
  behavior, iCloud entitlement specifics, or provider conflict notifications.
- Android `SharedPreferences`, `Editor`, DataStore `Flow`, protobuf-generated
  classes, or coroutine-specific ABI shapes.
- Browser `Storage`, `IDBDatabase`, `IDBTransaction`, DOM events, request
  objects, object-store/index APIs, or synchronous localStorage behavior.

All operations must enter through typed Macaca service commands with trace
context, policy checks, resource reservations, structured result envelopes,
sanitized audit events, unavailable provider behavior, and provider replacement
support.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
