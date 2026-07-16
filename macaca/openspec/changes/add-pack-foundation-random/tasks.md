## 1. Supplier API Research And Scope

- [x] 1.1 Read and summarize Web Crypto `getRandomValues` and `randomUUID`
  behavior, secure-context availability, worker support, limits, and errors.
- [x] 1.2 Read and summarize Node.js `crypto.randomBytes`, `randomFill`,
  `randomInt`, `randomUUID`, sync/async behavior, and provider limitations.
- [x] 1.3 Read and summarize Apple Security `SecRandomCopyBytes` and
  Randomization Services behavior for cryptographically secure random bytes.
- [x] 1.4 Read and summarize Android/Java `SecureRandom` and
  `getInstanceStrong` behavior, provider selection, and strong RNG diagnostics.
- [x] 1.5 Read and summarize POSIX/system RNG behavior for `getrandom`,
  `/dev/urandom`, blocking, entropy availability, and provider failures.
- [x] 1.6 Convert the supplier comparison into Macaca-owned abstractions and
  explicitly reject insecure PRNG or provider-native RNG handles.
- [x] 1.7 Record GitNexus CRITICAL/HIGH findings as memo only before
  implementation commits.

## 2. Contract, Descriptor, And DTOs

- [x] 2.1 Define `pack.foundation.random.v1` descriptor metadata: lifecycle,
  stability, service ids, command namespace, command schemas, permission scopes,
  policy template, resource template, SDK metadata, docs link, health, snapshot,
  and unavailable diagnostics.
- [x] 2.2 Define command DTOs for `random.bytes`, `random.fill`,
  `random.integer`, `random.uuid_v4`, `random.nonce`, `random.token`,
  `random.test_stream_create`, `random.test_stream_bytes`,
  `random.entropy_health`, and `random.provider_capabilities`.
- [x] 2.3 Define shared DTOs for strength class, purpose, byte request, integer
  range, token spec, UUID spec, seed reference, deterministic stream reference,
  entropy health, provider capability report, and stable descriptor hashes.
- [x] 2.4 Define result/error DTOs for success, denied, invalid_length,
  invalid_range, invalid_alphabet, unsupported, deterministic_not_allowed,
  quota_exceeded, entropy_unavailable, blocked, unavailable, and
  provider_failure.
- [x] 2.5 Add schema compatibility tests and stable hash tests for command,
  result, health, snapshot, provider capability, and unavailable DTOs.

## 3. Admission, Permission, Policy, Resource, And Approval

- [x] 3.1 Implement manifest declaration validation for required/optional
  `pack.foundation.random.v1`.
- [x] 3.2 Validate scopes: `random.generate`, `random.identifier`,
  `random.token`, `random.nonce`, `random.health`, and `random.test_seed`.
- [x] 3.3 Add policy checks for strength class, deterministic-test context,
  byte length, token length, alphabet class, integer bounds, rate limits, max
  blocking duration, and provider capability.
- [x] 3.4 Reject deterministic seeded streams outside test/replay contexts.
- [ ] 3.5 Add resource reservations/rate limits before large random byte or token
  generation requests.
- [ ] 3.6 Add tests proving denied, unavailable, quota, blocked, and unsupported
  paths do not use insecure fallback providers.

## 4. Service Provider And Runtime Integration

- [ ] 4.1 Define the random service trait/provider interface behind the service
  runtime.
- [x] 4.2 Implement unavailable provider behavior for absent random service,
  entropy unavailable, blocked provider, unsupported UUID/integer/token feature,
  and disabled deterministic test provider.
- [ ] 4.3 Implement deterministic test provider for replay and test contexts with
  opaque seed references and stream position tracking.
- [ ] 4.4 Implement or bind host CSPRNG provider with max byte limits, health
  diagnostics, bias-free integer generation, and UUID/token helpers.
- [ ] 4.5 Add lifecycle, health, snapshot, shutdown, timeout, cancellation,
  rate-limit accounting, and provider capability reports.

## 5. SDK, WASM ABI, And Application Framework

- [x] 5.1 Extend SDK discovery with pack metadata, command schemas, strength
  classes, limits, provider class, deterministic-test support, permissions,
  policy templates, health, diagnostics, and docs link.
- [x] 5.2 Add SDK command builders for every `random.*` command; builders must
  only produce canonical traced service calls.
- [ ] 5.3 Add SDK helpers for bytes, UUID v4, nonce, token, bounded integer,
  deterministic test stream, entropy health, and unavailable diagnostics.
- [ ] 5.4 Extend effective capability projection so applications can inspect
  callable commands, denied commands, unavailable entropy/provider states,
  provider capability flags, and replay references.
- [ ] 5.5 Expose WASM host imports only for declared callable random commands and
  route every import through the service runtime path.
- [ ] 5.6 Add app-framework tests proving YAML, WASM, GenUI, and headless apps all
  use the same random execution path.

## 6. Trace, Audit, Replay, And Gates

- [ ] 6.1 Emit sanitized events for declaration, admission, policy, resource,
  service calls, deterministic stream creation, entropy health, success, failure,
  denied, blocked, and unavailable states.
- [ ] 6.2 Add audit redaction tests proving generated bytes, tokens, UUIDs,
  nonces, raw seeds, credentials, private keys, provider payloads, and raw
  secrets do not enter observability surfaces.
- [ ] 6.3 Add replay tests proving random commands are trace-addressable and that
  deterministic streams can replay only in approved replay contexts.
- [ ] 6.4 Add dependency-boundary tests proving kernel, SDK, shells, and
  application framework do not import concrete RNG providers.
- [ ] 6.5 Add no-direct-provider-call gates proving SDK helpers and WASM host
  imports cannot bypass service runtime.
- [ ] 6.6 Run `openspec validate add-pack-foundation-random --strict`, targeted
  cargo tests, dependency-boundary gates, file-size gates, and audit replay
  checks before marking complete.

## 7. Developer Documentation

- [x] 7.1 Create `docs/developer-packs/foundation/random.md`.
- [x] 7.2 Document purpose, manifest declaration, cryptographic versus
  deterministic randomness, permissions, policy defaults, resource/rate limits,
  command DTOs, result DTOs, error DTOs, UUID/nonce/token guidance, integer bias
  avoidance, deterministic test streams, unavailable diagnostics, and provider
  replacement.
- [x] 7.3 Add minimal examples for generating bytes, UUID v4, nonce, token,
  bounded random integer, deterministic test stream, denied test seed in
  production, and entropy unavailable diagnostics.
- [x] 7.4 Cross-link the guide from SDK discovery metadata and the industrial pack
  catalog index before marking this proposal complete.
