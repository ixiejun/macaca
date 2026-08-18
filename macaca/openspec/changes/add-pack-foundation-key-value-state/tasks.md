## 1. Supplier API Research And Scope

- [x] 1.1 Read and summarize Redis commands relevant to get/set/delete, batch,
  increment, TTL, transactions, WATCH, scan, pub-sub/streams, and persistence.
- [x] 1.2 Read and summarize etcd/Consul-style KV APIs relevant to revisions,
  compare-and-swap, leases, watch, prefix query, compaction, and health.
- [x] 1.3 Read and summarize Apple UserDefaults and iCloud key-value storage
  concepts relevant to app scope, value types, quotas, sync, and conflicts.
- [x] 1.4 Read and summarize Android SharedPreferences and Jetpack DataStore
  concepts relevant to typed preferences, transactional updates, flows, and
  migration.
- [x] 1.5 Read and summarize Web Storage and IndexedDB concepts relevant to
  origin scope, transactions, quota, versioning, and async persistence.
- [x] 1.6 Convert the supplier comparison into Macaca-owned abstractions and
  explicitly reject provider-native API leakage.
- [x] 1.7 Record GitNexus CRITICAL/HIGH findings as memo only before
  implementation commits.

## 2. Contract, Descriptor, And DTOs

- [x] 2.1 Define `pack.foundation.key.value.state.v1` descriptor metadata:
  lifecycle, stability, service ids, command namespace, command schemas,
  permission scopes, policy template, resource template, SDK metadata, docs
  link, health, snapshot, and unavailable diagnostics.
- [x] 2.2 Define command DTOs for `kv.get`, `kv.put`, `kv.delete`, `kv.exists`,
  `kv.batch_get`, `kv.batch_put`, `kv.batch_delete`, `kv.list_keys`,
  `kv.compare_and_set`, `kv.increment`, `kv.set_ttl`, `kv.get_ttl`,
  `kv.watch_namespace`, `kv.snapshot_namespace`, `kv.restore_namespace`,
  `kv.migrate_namespace`, and `kv.compact_namespace`.
- [x] 2.3 Define shared DTOs for namespace refs, key refs, typed values,
  revisions, TTL policy, consistency level, conflict mode, watch events,
  snapshot refs, provider capability reports, and stable descriptor hashes.
- [x] 2.4 Define result/error DTOs for success, partial page, watch checkpoint,
  denied, not_found, already_exists, conflict, invalid_key, invalid_namespace,
  quota_exceeded, too_large, unsupported, expired, compacted_revision,
  unavailable, and provider_failure.
- [x] 2.5 Add schema compatibility tests and stable hash tests for command,
  result, health, snapshot, provider capability, and unavailable DTOs.

## 3. Admission, Permission, Policy, Resource, And Approval

- [x] 3.1 Implement manifest declaration validation for required/optional
  `pack.foundation.key.value.state.v1` and app-scoped namespaces.
- [x] 3.2 Validate scopes: `state.read`, `state.write`, `state.delete`,
  `state.list`, `state.watch`, `state.ttl`, `state.counter`,
  `state.snapshot`, `state.restore`, `state.migrate`, and `state.compact`.
- [x] 3.3 Add policy checks for namespace scope, key prefix bounds, max key size,
  max value size, batch size, scan page size, stream budget, TTL bounds,
  retention, consistency level, and provider capability.
- [x] 3.4 Add side-effect approval behavior for namespace-wide delete, restore,
  migration, compaction, large batch mutation, and overwrite without revision.
- [x] 3.5 Add resource reservations before side-effect provider calls and release
  resources on success, failure, timeout, cancellation, and stream termination.
- [x] 3.6 Add tests proving denied, unavailable, quota, and unsupported paths do
  not invoke a concrete provider.

## 4. Service Provider And Runtime Integration

- [x] 4.1 Define the key-value state service trait/provider interface behind the
  service runtime.
- [x] 4.2 Implement unavailable provider behavior for absent state service,
  disabled namespace, unsupported TTL/watch/snapshot/compaction capability, and
  missing entitlement.
- [x] 4.3 Implement deterministic in-memory mock provider for contract and replay
  tests.
- [x] 4.4 Implement or bind an embedded durable provider with namespace sandboxing,
  revision tracking, bounded scans, TTL cleanup, and snapshot references.
- [x] 4.5 Add optional adapter bridge points for Redis-like and etcd-like
  providers without leaking provider-native APIs to SDK callers.
- [x] 4.6 Add lifecycle, health, snapshot, shutdown, timeout, cancellation,
  bounded watch streaming, compaction handling, and provider capability reports.

## 5. SDK, WASM ABI, And Application Framework

- [x] 5.1 Extend SDK discovery with pack metadata, command schemas, namespace
  rules, value types, permissions, policy templates, provider availability,
  consistency/TTL/watch support, health, diagnostics, and docs link.
- [x] 5.2 Add SDK command builders for every `kv.*` command; builders must only
  produce canonical traced service calls.
- [x] 5.3 Add SDK helpers for CAS update loops, bounded prefix scans, watch stream
  cancellation, TTL cache entries, snapshot dry runs, and unavailable
  diagnostics.
- [x] 5.4 Extend effective capability projection so applications can inspect
  callable commands, denied commands, unavailable namespaces, provider capability
  flags, and replay references.
- [x] 5.5 Expose WASM host imports only for declared callable KV commands and
  route every import through the service runtime path.
- [x] 5.6 Add app-framework tests proving YAML, WASM, GenUI, and headless apps all
  use the same KV execution path.

## 6. Trace, Audit, Replay, And Gates

- [ ] 6.1 Emit sanitized events for declaration, admission, policy, resource,
  entitlement, service calls, watch lifecycle, snapshots, restore, migration,
  compaction, success, failure, denied, and unavailable states.
- [x] 6.2 Add audit redaction tests proving raw values, raw secrets, prompts,
  manifests, package bytes, credentials, private keys, provider payloads, and
  unbounded key listings do not enter observability surfaces.
- [x] 6.3 Add replay tests proving every KV command is trace-addressable and can
  reconstruct the decision path without replaying raw state values.
- [x] 6.4 Add dependency-boundary tests proving kernel, SDK, shells, and
  application framework do not import concrete KV providers.
- [x] 6.5 Add no-direct-provider-call gates proving SDK helpers and WASM host
  imports cannot bypass service runtime.
- [ ] 6.6 Run `openspec validate add-pack-foundation-key-value-state --strict`,
  targeted cargo tests, dependency-boundary gates, file-size gates, and audit
  replay checks before marking complete.

## 7. Developer Documentation

- [x] 7.1 Create `docs/developer-packs/foundation/key-value-state.md`.
- [x] 7.2 Document purpose, manifest declaration, namespace model, key model,
  value model, permissions, policy defaults, resource limits, approval cases,
  command DTOs, result DTOs, error DTOs, CAS, TTL, watch streams, snapshots,
  restore, migration, compaction, unavailable diagnostics, and provider
  replacement.
- [x] 7.3 Add minimal examples for preference get/put, CAS update loop, TTL cache
  entry, bounded prefix scan, watch stream cancellation, snapshot/restore dry
  run, unavailable provider diagnostics, and denied namespace-wide delete.
- [x] 7.4 Cross-link the guide from SDK discovery metadata and the industrial pack
  catalog index before marking this proposal complete.
