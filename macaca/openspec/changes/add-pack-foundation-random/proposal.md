# Change: Add Foundation Random Pack

## Why

Developers need `pack.foundation.random.v1` as a cryptographically safe,
provider-neutral randomness capability. Applications need random bytes, UUIDs,
nonces, tokens, bounded random integers, deterministic test streams, entropy
health diagnostics, and explicit unavailable behavior without directly calling
host RNG APIs.

Randomness is foundational for session ids, idempotency keys, nonces, temporary
resource names, tests, security workflows, and provider protocols. If every
application chooses its own RNG source, Macaca cannot enforce policy, audit
entropy failures, separate cryptographic from deterministic test randomness, or
avoid insecure pseudo-random use.

## Supplier And Platform API Research

The proposal is derived from a capability-by-capability comparison of mature
randomness APIs:

- Web Crypto: `crypto.getRandomValues` for cryptographically strong random
  values and `crypto.randomUUID` for v4 UUID generation in secure contexts.
- Node.js `crypto`: `randomBytes`, `randomFill`, `randomInt`, `randomUUID`, and
  callback/sync variants backed by platform cryptographic providers.
- Apple Security: `SecRandomCopyBytes` and Randomization Services for
  cryptographically secure random bytes.
- Android/Java `SecureRandom`: provider-backed random bytes, algorithm/provider
  selection, `getInstanceStrong`, and strong RNG diagnostics.
- POSIX/system RNG: `getrandom`, `/dev/urandom`, blocking/availability behavior,
  entropy source health, and failure modes.

Macaca borrows the stable concepts, not provider APIs:

- distinguish cryptographic random, UUID, nonce/token, bounded integer, and
  deterministic test streams;
- never expose provider RNG objects to applications;
- reject deterministic seeded streams outside test/replay policy;
- expose entropy/provider health diagnostics;
- normalize unavailable, blocked, unsupported, quota, and provider failure
  results.

## What Changes

- Define `pack.foundation.random.v1` as the canonical app-facing randomness pack.
- Add an industrial command surface covering random bytes, random fill, random
  integer, UUID v4, nonce, token, deterministic test stream creation, deterministic
  stream bytes, entropy health, and provider capability inspection.
- Define provider-neutral DTO requirements for strength class, byte length, token
  alphabet, integer bounds, UUID version, nonce purpose, deterministic seed
  reference, replay binding, and entropy diagnostics.
- Define permission scopes for cryptographic generation, identifier generation,
  token generation, deterministic test streams, and entropy health.
- Require a detailed developer guide under `docs/developer-packs/foundation/random.md`
  before this proposal can be marked complete.
- Keep implementation ownership in a random system service; kernel, SDK, shells,
  and application framework remain provider-neutral.

## Impact

- Affected specs: `pack-foundation-random`,
  `developer-pack-industrial-capability-catalog`, `sdk-system-facade`,
  `service-runtime`, `unified-execution-path`.
- Affected code later: provider-neutral DTOs, descriptor validators, application
  admission, SDK discovery, SDK command helpers, random service provider,
  deterministic mock/test provider, unavailable provider, trace/audit event
  schema, replay tests, and dependency-boundary gates.
- Non-goals: insecure pseudo-random generators for production use, raw provider
  RNG handles in SDK, provider-name routing, app-specific token formats, or
  deterministic seeded randomness outside test/replay policy.
