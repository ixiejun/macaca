# Foundation Random Pack

`pack.foundation.random.v1` provides provider-neutral randomness primitives for
Macaca applications. It is designed for cryptographic random bytes, UUID v4
identifiers, nonces, tokens, bounded integers, deterministic test streams, and
entropy diagnostics without exposing host RNG handles.

## Manifest Declaration

Declare the pack in an application service contract:

```yaml
service_contract:
  optional_packs:
    - pack.foundation.random.v1
```

Use `required_packs` only when the application cannot run without a registered
random provider. When no provider is installed, admission returns an explicit
`random_provider_not_installed` diagnostic instead of faking success.

## Permissions

The pack defines these provider-neutral scopes:

- `random.generate`: random bytes and fill operations.
- `random.identifier`: UUID v4 generation.
- `random.token`: token generation.
- `random.nonce`: nonce generation.
- `random.health`: entropy and provider capability inspection.
- `random.test_seed`: deterministic test stream creation.

## Commands

- `random.bytes`: generate byte-like output with a length, strength class,
  purpose, encoding, and optional max blocking duration.
- `random.fill`: fill an opaque artifact range through the service runtime.
- `random.integer`: generate a bias-free bounded integer.
- `random.uuid_v4`: generate canonical v4 UUID values.
- `random.nonce`: generate nonces with purpose and encoding metadata.
- `random.token`: generate bounded tokens from a declared alphabet class.
- `random.test_stream_create`: create an approved deterministic test stream from
  an opaque seed reference.
- `random.test_stream_bytes`: read deterministic bytes from an approved stream.
- `random.entropy_health`: inspect entropy availability and blocking risk.
- `random.provider_capabilities`: inspect supported commands, limits, strength
  classes, deterministic stream support, and availability.

## DTO Guidance

Use `RandomStrengthClass::Cryptographic` for production secrets, identifiers,
nonces, and tokens. Use `StrongWhenAvailable` only when policy allows degraded
host entropy. Use `DeterministicTest` only in test or replay contexts.

Generated values, raw seeds, provider payloads, credentials, private keys, and
secret material must never enter logs, traces, snapshots, SDK diagnostics, or
examples. Observability records include only bounded metadata such as length,
purpose, strength class, provider class, result code, trace id, descriptor hash,
and sanitized unavailable reason.

## Result And Error DTOs

All commands use a bounded result envelope with status, optional data, optional
error, trace id, and descriptor hash. Standard statuses are `success`, `denied`,
`invalid_length`, `invalid_range`, `invalid_alphabet`, `unsupported`,
`deterministic_not_allowed`, `quota_exceeded`, `entropy_unavailable`, `blocked`,
`unavailable`, and `provider_failure`.

## Examples

Random bytes:

```json
{
  "length": 32,
  "strength": "cryptographic",
  "purpose": "session_id",
  "encoding": "base64_url",
  "max_blocking_ms": 50
}
```

UUID v4:

```json
{
  "count": 1,
  "lowercase": true
}
```

Nonce:

```json
{
  "byte_length": 16,
  "purpose": "nonce",
  "encoding": "hex",
  "uniqueness_window": "session"
}
```

Token:

```json
{
  "char_length": 32,
  "alphabet": "url_safe",
  "purpose": "idempotency_key",
  "collision_warning_policy": "warn_only"
}
```

Bounded integer:

```json
{
  "min_inclusive": 10,
  "max_exclusive": 20,
  "purpose": "generic",
  "require_bias_free": true
}
```

Deterministic test stream:

```json
{
  "seed": {
    "seed_ref": "opaque-seed-reference",
    "replay_binding": "trace-or-test-run-id"
  },
  "algorithm_id": "deterministic-test-v1",
  "replay_policy": "test_only"
}
```

Production policy must deny deterministic stream creation unless the execution
context is explicitly a test or replay context.

Entropy unavailable diagnostic:

```json
{
  "status": "unavailable",
  "error": {
    "code": "unavailable",
    "message": "random provider is not installed",
    "retryable": false
  }
}
```

## Provider Replacement

Providers are replaceable service implementations. Expected provider classes
include `host-csprng`, `deterministic-test`, `mock`, and `unavailable`.
Provider adapters must expose descriptor metadata, health, snapshots, command
support, unavailable behavior, and sanitized diagnostics through the service
runtime. SDKs, shells, kernel code, and applications must not instantiate
provider RNG objects directly.
