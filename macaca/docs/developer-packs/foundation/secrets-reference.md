# Foundation Secrets Reference Pack

`pack.foundation.secrets.reference.v1` provides provider-neutral references to
secret material. It covers reference creation/import, metadata inspection,
purpose binding, provider-only resolution, leases, renew/revoke, rotation,
version status, audit access, unavailable diagnostics, and provider replacement
without exposing raw secret values to applications.

## Reference-Only Model

Applications receive secret references, version status, lease references,
resolution handles, and audit metadata. They do not receive raw secret values.
Raw value access is only allowed inside an admitted provider-to-provider
injection path after policy, purpose, entitlement, and audit gates pass.

## Manifest Declaration

Declare the pack in an application service contract:

```yaml
service_contract:
  optional_packs:
    - pack.foundation.secrets.reference.v1
```

Use `required_packs` only when the application cannot run without a registered
secret-reference provider. When no provider is installed, admission returns
`secrets_reference_provider_not_installed`.

## Permissions

The pack defines these provider-neutral scopes:

- `secrets.reference.read`: inspect reference metadata.
- `secrets.reference.create`: create managed references.
- `secrets.reference.import`: import an external redacted locator.
- `secrets.reference.list`: list bounded references.
- `secrets.reference.bind`: bind a reference to a purpose and service.
- `secrets.reference.resolve`: request provider-only resolution.
- `secrets.reference.lease`: create or renew leases.
- `secrets.reference.rotate`: rotate reference material.
- `secrets.reference.revoke`: revoke a lease.
- `secrets.reference.audit`: inspect sanitized access audit.

## Commands

- `secrets.create_reference`: create a reference with purpose and access policy.
- `secrets.import_reference`: import an external redacted locator.
- `secrets.inspect_reference`: inspect metadata for one reference.
- `secrets.list_references`: list bounded references by provider class.
- `secrets.bind_purpose`: bind reference use to a purpose and service.
- `secrets.resolve_for_provider`: create a provider-only resolution handle.
- `secrets.create_lease`: create a bounded lease.
- `secrets.renew_lease`: renew an active lease.
- `secrets.revoke_lease`: revoke an active lease with a reason.
- `secrets.rotate_reference`: request rotation or rotation dry run.
- `secrets.version_status`: inspect current version state.
- `secrets.audit_access`: inspect sanitized audit records.

## DTO Guidance

Use `SecretReference` for stable app-facing identity, `SecretExternalLocator`
only with a redacted locator hash, `SecretPurposeBinding` to bind use to a
service and purpose, and `SecretResolutionHandle` only as an opaque provider
injection handle. `SecretLeaseReference` records bounded lease lifetime without
carrying secret material.

Logs, traces, snapshots, SDK diagnostics, and examples must not include raw
secret values, provider-private locators, credentials, private keys, raw
signatures, provider payloads, prompts, manifests, or unbounded output.

## Result And Error DTOs

All commands use a bounded result envelope with status, optional data, optional
error, trace id, and descriptor hash. Standard statuses are `success`, `denied`,
`not_found`, `disabled`, `destroyed`, `expired`, `rotation_required`,
`lease_expired`, `invalid_purpose`, `unsupported`, `unavailable`,
`provider_failure`, and `raw_secret_forbidden`.

## Examples

Declaring a reference:

```json
{
  "reference": {
    "reference_id": "secret-ref",
    "provider_class": "vault",
    "version_hint": "current"
  },
  "purpose": {
    "purpose": "database-password",
    "service_id": "service.example",
    "expires_at_epoch_millis": 1800000000000
  },
  "policy": {
    "allowed_service_ids": ["service.example"],
    "requires_approval": true,
    "max_lease_ttl_seconds": 300
  }
}
```

Inspecting metadata:

```json
{
  "reference": {
    "reference_id": "secret-ref",
    "provider_class": "vault",
    "version_hint": "current"
  }
}
```

Binding a purpose:

```json
{
  "reference": {
    "reference_id": "secret-ref",
    "provider_class": "vault",
    "version_hint": "current"
  },
  "purpose": {
    "purpose": "api-token",
    "service_id": "service.api",
    "expires_at_epoch_millis": 1800000000000
  }
}
```

Resolving for provider without raw exposure:

```json
{
  "reference": {
    "reference_id": "secret-ref",
    "provider_class": "vault",
    "version_hint": "current"
  },
  "purpose": "database-password",
  "service_id": "service.example"
}
```

Rotating a reference:

```json
{
  "reference": {
    "reference_id": "secret-ref",
    "provider_class": "vault",
    "version_hint": "current"
  },
  "dry_run": true
}
```

Revoking a lease:

```json
{
  "lease": {
    "lease_id": "lease-ref",
    "reference_id": "secret-ref",
    "expires_at_epoch_millis": 1800000000000
  },
  "reason": "session-finished"
}
```

Denied raw secret access:

```json
{
  "status": "raw_secret_forbidden",
  "error": {
    "code": "raw_secret_forbidden",
    "message": "raw secret values are not app-facing results",
    "retryable": false
  }
}
```

Unavailable provider diagnostics:

```json
{
  "status": "unavailable",
  "error": {
    "code": "unavailable",
    "message": "secret-reference provider is not installed",
    "retryable": false
  }
}
```

## Provider Replacement

Providers are replaceable service implementations. Expected provider classes
include `vault`, `cloud-secrets`, `host-keychain`, `kubernetes-secret`, `mock`,
and `unavailable`. Provider adapters must expose descriptor metadata, health,
snapshots, reference-state hashes, lease-state hashes, audit-tail hashes,
unavailable states, and sanitized diagnostics through the service runtime.
SDKs, shells, kernel code, and applications must not instantiate provider
secret clients or expose raw secret values directly.
