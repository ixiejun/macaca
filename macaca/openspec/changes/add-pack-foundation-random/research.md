# Foundation Random Pack Research

## Purpose

This note records supplier/API research for `pack.foundation.random.v1`. The
pack must provide provider-neutral cryptographic random bytes, identifiers,
nonces, tokens, bounded integers, deterministic test streams, and entropy health
without exposing provider RNG objects or permitting insecure fallback behavior.

## Source Baseline

- MDN `Crypto.getRandomValues()`:
  <https://developer.mozilla.org/en-US/docs/Web/API/Crypto/getRandomValues>
- MDN `Crypto.randomUUID()`:
  <https://developer.mozilla.org/en-US/docs/Web/API/Crypto/randomUUID>
- Node.js `crypto`:
  <https://nodejs.org/api/crypto.html>
- Apple `SecRandomCopyBytes`:
  <https://developer.apple.com/documentation/security/secrandomcopybytes%28_%3A_%3A_%3A%29>
- Apple Randomization Services:
  <https://developer.apple.com/documentation/security/randomization-services>
- Oracle Java `SecureRandom`:
  <https://docs.oracle.com/javase/8/docs/api/java/security/SecureRandom.html>
- Android `SecureRandom`:
  <https://developer.android.com/reference/java/security/SecureRandom>
- Linux `getrandom(2)`:
  <https://man7.org/linux/man-pages/man2/getrandom.2.html>
- Linux random devices:
  <https://man7.org/linux/man-pages/man4/random.4.html>
- OpenBSD `arc4random(3)`:
  <https://man.openbsd.org/arc4random>

## Web Crypto Summary

Web Crypto contributes a browser/worker model for cryptographic bytes and v4
UUIDs:

- `getRandomValues` fills caller-provided typed arrays with cryptographically
  strong random values and is available in Web Workers.
- `randomUUID` generates v4 UUID values through a cryptographically secure RNG
  and is constrained by secure-context availability.
- Browser APIs impose size and type restrictions. Macaca should convert those
  into `invalid_length`, `unsupported`, `quota_exceeded`, or `unavailable`
  results rather than leaking DOM exceptions.
- Secure-context and worker support map to provider capability and availability
  diagnostics. They must not become SDK-specific branches.

## Node.js Crypto Summary

Node.js `crypto` provides random bytes, fill, integers, and UUID generation with
sync/async variants:

- `randomBytes` and `randomFill` map to `random.bytes` and `random.fill`.
- `randomInt` contributes the requirement for bias-free bounded integer
  generation.
- `randomUUID` maps to `random.uuid_v4`.
- Sync/callback behavior must not leak into Macaca. All calls become canonical
  service commands with timeout, cancellation, trace, and structured results.
- Provider/OpenSSL details belong in provider capability and health snapshots,
  not in application-facing DTOs.

## Apple Security Summary

Apple Security exposes cryptographically secure byte generation through
`SecRandomCopyBytes` and Randomization Services:

- The stable concept is CSPRNG-backed byte generation.
- Provider failures must be structured and auditable. Macaca should return
  `entropy_unavailable`, `blocked`, `unavailable`, or `provider_failure`.
- Apple random generator references must not appear in SDK, WASM ABI, trace, or
  audit records.

## Android / Java SecureRandom Summary

Java and Android `SecureRandom` provide provider-backed cryptographically strong
randomness:

- `SecureRandom` is a CSPRNG abstraction with provider/algorithm selection.
- `getInstanceStrong` shows strong RNG selection can fail or block depending on
  provider availability, so Macaca needs strength class and health diagnostics.
- Raw provider names, algorithms, seed mutation, and `SecureRandom` instances
  must not become stable Macaca API.
- Deterministic seeded streams are useful only for tests and replay. They must
  be denied in production execution contexts by policy.

## POSIX / System RNG Summary

System RNG APIs such as `getrandom`, `/dev/urandom`, and `arc4random` establish
host entropy behavior:

- `getrandom` may block until entropy is initialized and can have request-size
  behavior that affects interruption and partial results.
- `/dev/urandom` and kernel CSPRNG devices are host-specific; Macaca should
  expose entropy health and blocking risk, not path-level device semantics.
- `arc4random` provides CSPRNG values without exposing seed management, which
  supports Macaca's no-provider-handle rule.
- Large byte requests need max-size limits, resource reservations, and rate
  limits before provider calls.

## Macaca-Owned Abstractions

`pack.foundation.random.v1` should define these provider-neutral concepts:

- `RandomStrength`: cryptographic, strong_when_available, deterministic_test.
- `RandomPurpose`: session id, nonce, idempotency key, temporary name, test data,
  provider protocol, and generic use.
- `RandomBytesRequest`: byte length, strength class, purpose, maximum blocking
  duration, rate-limit class, and trace binding.
- `RandomIntegerRequest`: minimum, maximum, inclusive/exclusive mode, and
  bias-free generation requirement.
- `RandomTokenSpec`: length, alphabet class, encoding, collision-warning policy,
  and redaction behavior.
- `RandomUuidSpec`: UUID version, canonical formatting, count, entropy cache
  policy, and provider capability requirement.
- `RandomSeedRef`: opaque deterministic seed reference for approved test/replay
  contexts; raw seeds must never enter observability.
- `RandomStreamRef`: deterministic stream id, algorithm id, current position,
  policy context, and replay binding.
- `RandomHealth`: provider class, entropy availability, blocking risk, max byte
  request, deterministic support, and unavailable reasons.
- `RandomProviderCapability`: supported commands, strength classes, byte limits,
  integer support, UUID support, token support, deterministic-test support,
  health, and unavailable diagnostics.

## Rejected API Leakage

Macaca must not expose these provider-native shapes as stable SDK/ABI contracts:

- Web Crypto `Crypto` objects, typed-array mutation semantics, DOM exception
  names, secure-context branching, or browser worker object handles.
- Node.js callback/sync variants, `Buffer` mutation APIs, OpenSSL provider
  details, entropy-cache options, or Node exception types.
- Apple `SecRandomRef`, Security framework status codes, or platform generator
  references.
- Java/Android `SecureRandom` objects, provider/algorithm selectors, seed
  mutation APIs, or Java exception types.
- POSIX file paths such as `/dev/urandom`, syscall flags, raw errno values,
  `getrandom` buffer behavior, or BSD `arc4random` function names.
- Any insecure pseudo-random generator for production commands.

All operations must enter through typed Macaca service commands with trace
context, policy checks, resource/rate limits, structured result envelopes,
sanitized audit events, unavailable provider behavior, and provider replacement
support.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
