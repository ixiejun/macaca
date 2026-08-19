## 1. Supplier API Research And Scope

- [x] 1.1 Read and summarize AWS Secrets Manager APIs for get, batch get,
  versions/stages, rotation, resource policies, and CloudTrail audit guidance.
- [x] 1.2 Read and summarize HashiCorp Vault KV/secrets engine APIs for mounts,
  versions, leases, dynamic secrets, policies, metadata, and audit behavior.
- [x] 1.3 Read and summarize Kubernetes Secret concepts for object references,
  environment/volume injection, object validation, retry, and event diagnostics.
- [x] 1.4 Read and summarize Apple Keychain Services concepts for keychain items,
  access groups, accessibility, access control, and user/device authentication.
- [x] 1.5 Read and summarize cloud key vault/KMS style concepts for versioned
  references, disabled/destroyed versions, access policies, rotation, and audit.
- [x] 1.6 Convert the supplier comparison into Macaca-owned abstractions and
  explicitly reject raw secret values as ordinary app-facing results.
- [x] 1.7 Record GitNexus CRITICAL/HIGH findings as memo only before
  implementation commits.

## 2. Contract, Descriptor, And DTOs

- [x] 2.1 Define `pack.foundation.secrets.reference.v1` descriptor metadata:
  lifecycle, stability, service ids, command namespace, command schemas,
  permission scopes, policy template, resource template, SDK metadata, docs
  link, health, snapshot, and unavailable diagnostics.
- [x] 2.2 Define command DTOs for `secrets.create_reference`,
  `secrets.import_reference`, `secrets.inspect_reference`,
  `secrets.list_references`, `secrets.bind_purpose`,
  `secrets.resolve_for_provider`, `secrets.create_lease`,
  `secrets.renew_lease`, `secrets.revoke_lease`,
  `secrets.rotate_reference`, `secrets.version_status`, and
  `secrets.audit_access`.
- [x] 2.3 Define shared DTOs for secret reference, external locator,
  purpose, access policy, lease ref, resolution handle, version status, audit
  record, provider capability report, and stable descriptor hashes.
- [x] 2.4 Define result/error DTOs for success, denied, not_found, disabled,
  destroyed, expired, rotation_required, lease_expired, invalid_purpose,
  unsupported, unavailable, provider_failure, and raw_secret_forbidden.
- [x] 2.5 Add schema compatibility tests and stable hash tests for command,
  result, health, snapshot, provider capability, and unavailable DTOs.

## 3. Admission, Permission, Policy, Resource, And Approval

- [x] 3.1 Implement manifest declaration validation for required/optional
  `pack.foundation.secrets.reference.v1`, reference declarations, allowed
  purposes, and allowed service ids.
- [x] 3.2 Validate scopes: `secrets.reference.read`,
  `secrets.reference.create`, `secrets.reference.import`,
  `secrets.reference.list`, `secrets.reference.bind`,
  `secrets.reference.resolve`, `secrets.reference.lease`,
  `secrets.reference.rotate`, `secrets.reference.revoke`, and
  `secrets.reference.audit`.
- [x] 3.3 Add policy checks for reference id, provider class, service id,
  purpose, ttl, version status, rotation state, lease state, approval
  requirement, and provider capability.
- [x] 3.4 Reject all app-facing raw secret value results and all trace/audit/
  snapshot/diagnostic payloads containing raw secret values or provider-private
  locators.
- [x] 3.5 Add approval behavior for import, purpose binding, provider resolution,
  rotation, revoke, and audit export.
- [ ] 3.6 Add tests proving denied, unavailable, disabled, destroyed, expired,
  raw_secret_forbidden, and invalid_purpose paths do not inject secrets into
  providers.

## 4. Service Provider And Runtime Integration

- [x] 4.1 Define the secret-reference service trait/provider interface behind the
  service runtime.
- [x] 4.2 Implement unavailable provider behavior for absent secret service,
  missing reference, unsupported lease/rotation/version/audit behavior, and
  missing entitlement.
- [x] 4.3 Implement deterministic mock provider that returns references, leases,
  and diagnostics without creating real secrets.
- [x] 4.4 Implement adapter bridge points for AWS Secrets Manager, Vault,
  Kubernetes Secrets, Apple Keychain, and cloud key vault providers without
  leaking provider-native APIs to SDK callers.
- [x] 4.5 Add provider-side injection path for admitted service providers while
  preventing raw value exposure to applications and WASM guests.
- [x] 4.6 Add lifecycle, health, snapshot, shutdown, lease cleanup, rotation
  state, redaction, and provider capability reports.

## 5. SDK, WASM ABI, And Application Framework

- [x] 5.1 Extend SDK discovery with pack metadata, command schemas, reference
  classes, purpose classes, permissions, policy templates, lease/rotation
  support, provider availability, health, diagnostics, and docs link.
- [x] 5.2 Add SDK command builders for every `secrets.*` command; builders must
  only produce canonical traced service calls.
- [x] 5.3 Add SDK helpers for metadata inspection, purpose binding, provider
  resolution request building, lease renew/revoke, rotation request, audit access,
  and unavailable diagnostics.
- [ ] 5.4 Extend effective capability projection so applications can inspect
  callable commands, denied commands, unavailable providers, reference states,
  provider capability flags, and replay references.
- [ ] 5.5 Expose WASM host imports only for declared callable reference commands
  and never expose raw secret values or provider-private locators.
- [ ] 5.6 Add app-framework tests proving YAML, WASM, GenUI, and headless apps all
  use the same secret-reference execution path.

## 6. Trace, Audit, Replay, And Gates

- [ ] 6.1 Emit sanitized events for declaration, admission, policy, reference
  create/import, purpose binding, provider resolution, provider injection, lease
  create/renew/revoke, rotation, success, failure, denied, and unavailable states.
- [ ] 6.2 Add redaction tests proving raw secret values, external provider
  locators, credentials, private keys, raw signatures, raw provider payloads,
  prompts, manifests, and unbounded output do not enter observability surfaces.
- [ ] 6.3 Add replay tests proving secret-reference commands are trace-addressable
  and can reconstruct decisions without revealing raw secret values.
- [ ] 6.4 Add dependency-boundary tests proving kernel, SDK, shells, and
  application framework do not import concrete secret providers.
- [ ] 6.5 Add no-direct-provider-call gates proving SDK helpers and WASM host
  imports cannot bypass service runtime.
- [ ] 6.6 Run `openspec validate add-pack-foundation-secrets-reference --strict`,
  targeted cargo tests, dependency-boundary gates, file-size gates, and audit
  replay checks before marking complete.

## 7. Developer Documentation

- [x] 7.1 Create `docs/developer-packs/foundation/secrets-reference.md`.
- [x] 7.2 Document reference-only model, raw-secret prohibition, manifest
  declaration, purpose binding, permissions, policy defaults, command DTOs,
  result DTOs, error DTOs, provider resolution flow, leases, rotation, revocation,
  version status, audit access, unavailable diagnostics, and provider replacement.
- [x] 7.3 Add minimal examples for declaring a reference, inspecting metadata,
  binding a purpose, resolving for provider without raw exposure, rotating,
  revoking a lease, denied raw secret access, and unavailable provider
  diagnostics.
- [x] 7.4 Cross-link the guide from SDK discovery metadata and the industrial pack
  catalog index before marking this proposal complete.
