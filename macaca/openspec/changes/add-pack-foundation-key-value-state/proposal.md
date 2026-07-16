# Change: Add Foundation Key-Value State Pack

## Why

Developers need `pack.foundation.key.value.state.v1` as a durable, namespaced,
policy-aware key-value state capability. Applications should not hand-roll state
files, encode private state into prompts, or call provider-specific databases
directly. They need a small but industrial state surface: get/put/delete, batch
operations, compare-and-set, TTL, watch, counters, snapshots, restore, and
explicit unavailable diagnostics.

This pack is foundational because higher-level workflow, session, UI, cache,
preference, task, and pack metadata features all need stable app-scoped state.
If this layer is shallow, every application will create its own state semantics,
breaking replay, recovery, audit, migration, and multi-provider portability.

## Supplier And Platform API Research

The proposal is derived from a capability-by-capability comparison of mature
key-value and preference APIs:

- Redis: `GET`, `SET`, `DEL`, `MGET`, `MSET`, `INCR`, `EXPIRE`, `TTL`, `WATCH`,
  transactions, key scanning, streams/pub-sub style change observation, and
  persistence diagnostics.
- etcd/Consul-style KV stores: revisioned keys, compare-and-swap, leases,
  watch streams, prefix queries, compacted history, and cluster health.
- Apple UserDefaults and iCloud key-value storage: app-scoped preference keys,
  limited value types, synchronization semantics, quotas, and conflict behavior.
- Android SharedPreferences and Jetpack DataStore Preferences: typed preference
  keys, transactional updates, coroutine/flow observation, migration from legacy
  stores, and app-local persistence.
- Web Storage and IndexedDB: origin-scoped key/value persistence, quota errors,
  transactions, object stores, versioning, and asynchronous browser storage.

Macaca borrows the stable concepts, not provider APIs:

- namespace and tenant scope every key;
- support typed values and opaque binary/blob references;
- expose revision-based compare-and-set instead of provider-native transactions;
- model TTL/lease behavior explicitly;
- support prefix/list/watch without unbounded output;
- provide snapshots and restore for replay/recovery;
- normalize provider failures into structured errors.

## What Changes

- Define `pack.foundation.key.value.state.v1` as the canonical app-facing
  key-value state pack.
- Add an industrial command surface covering get, put, delete, exists, batch
  get/put/delete, list keys, prefix scan, compare-and-set, increment, TTL,
  lease, watch, snapshot, restore, migrate namespace, and compact.
- Define provider-neutral DTO requirements for namespaces, keys, value types,
  revisions, TTLs, leases, watch streams, pagination, consistency level,
  idempotency, encryption class, and snapshot references.
- Define permission scopes for read, write, delete, list, watch, TTL, counter,
  snapshot, restore, migrate, and compact.
- Require a detailed developer guide under
  `docs/developer-packs/foundation/key-value-state.md` before this proposal can
  be marked complete.
- Keep implementation ownership in a state system service; kernel, SDK, shells,
  and application framework remain provider-neutral.

## Impact

- Affected specs: `pack-foundation-key-value-state`,
  `developer-pack-industrial-capability-catalog`, `sdk-system-facade`,
  `service-runtime`, `unified-execution-path`.
- Affected code later: provider-neutral DTOs, descriptor validators, application
  admission, SDK discovery, SDK command helpers, state service provider,
  mock/unavailable providers, trace/audit event schema, replay tests, and
  dependency-boundary gates.
- Non-goals: provider-specific Redis/etcd/IndexedDB APIs in SDK, raw database
  handles, app-specific workflow state machines, shell-owned state semantics, or
  direct provider construction outside approved composition roots.
