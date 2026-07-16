# Change: Add Foundation Secrets Reference Pack

## Why

Developers need `pack.foundation.secrets.reference.v1` as a safe way to declare,
inspect, pass, rotate, and revoke secret references without exposing raw secret
values to applications, SDK diagnostics, traces, audit logs, snapshots, prompts,
or WASM guests.

The pack is intentionally a reference capability, not a secret-value API.
Applications and providers often need credentials, tokens, API keys,
certificates, signing handles, or connection secrets, but generic Macaca layers
must never normalize those values into ordinary config/state/output data.

## Supplier And Platform API Research

The proposal is derived from a capability-by-capability comparison of mature
secret systems:

- AWS Secrets Manager: get secret value, batch get, version stages, rotation,
  CloudTrail audit, resource policies, and warnings not to include sensitive data
  in logged request parameters.
- HashiCorp Vault KV and secrets engines: secret engines, versioned KV, leases,
  policies, dynamic secrets, metadata, and mount-backed provider abstraction.
- Kubernetes Secrets: object references, environment/volume injection, object
  existence validation, retry/event diagnostics when a Secret cannot be fetched.
- Apple Keychain Services: keychain items, access groups, accessibility classes,
  access control, and user/device authentication constraints.
- Cloud KMS/Key Vault style APIs: key references, secret versions, access
  policies, rotation, disabled/destroyed versions, and audit evidence.

Macaca borrows the stable concepts, not provider APIs:

- secret use crosses the system only as opaque references, leases, or provider
  execution handles;
- raw secret reveal is not a normal application capability;
- policy, entitlement, approval, purpose, and provider health are checked before
  any secret-dependent side effect;
- rotation and revocation are first-class audited commands;
- traces include ids/hashes/status, never raw values.

## What Changes

- Define `pack.foundation.secrets.reference.v1` as the canonical app-facing
  secret-reference pack.
- Add an industrial command surface covering create/import reference, inspect
  reference, resolve for provider, create lease, renew lease, revoke lease,
  rotate, version status, bind purpose, list references, and audit access.
- Define provider-neutral DTO requirements for secret reference id, provider
  locator, version/stage, purpose, access policy, lease, rotation state, disabled
  state, expiry, audit metadata, and unavailable diagnostics.
- Define permission scopes for reference read, reference create/import, provider
  resolve, lease, rotate, revoke, list, and audit.
- Require a detailed developer guide under
  `docs/developer-packs/foundation/secrets-reference.md` before this proposal can
  be marked complete.
- Keep implementation ownership in a secret-reference system service; kernel,
  SDK, shells, and application framework remain provider-neutral.

## Impact

- Affected specs: `pack-foundation-secrets-reference`,
  `developer-pack-industrial-capability-catalog`, `sdk-system-facade`,
  `service-runtime`, `unified-execution-path`.
- Affected code later: provider-neutral DTOs, descriptor validators, application
  admission, SDK discovery, SDK command helpers, secret-reference service,
  mock/unavailable providers, trace/audit event schema, replay tests, redaction
  gates, and dependency-boundary gates.
- Non-goals: raw secret value retrieval for ordinary apps, secret storage in
  config/state packs, provider-specific secret APIs in SDK, or app-specific
  credential workflows in OS layers.
