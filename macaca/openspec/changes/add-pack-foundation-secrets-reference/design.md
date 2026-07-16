# Foundation Secrets Reference Pack Design

## Context

`pack.foundation.secrets.reference.v1` lets applications declare and use secret
references through policy-governed service calls. It is the bridge between
generic application code and provider-owned secret systems without converting
secrets into ordinary app data.

This pack deliberately does not provide `get_secret_value` to applications.
Instead, a provider that needs a secret receives a scoped resolution handle or
provider-local credential injection through the service runtime after policy and
approval checks. That preserves traceability without leaking raw secrets.

## Supplier API Comparison

| Source API family | Relevant concepts | Macaca abstraction |
| --- | --- | --- |
| AWS Secrets Manager | `GetSecretValue`, `BatchGetSecretValue`, versions/stages, rotation, resource policy, CloudTrail audit | opaque secret refs, version/stage metadata, rotation commands, audit events, no sensitive request params |
| HashiCorp Vault | KV v1/v2, secret engines, leases, policies, dynamic secrets, metadata | provider locator, lease refs, policy-bound resolve, version metadata, dynamic secret diagnostics |
| Kubernetes Secrets | Secret object refs, volume/env injection, object validation, kubelet retry/events | declared secret refs, provider-side injection, readiness blocking, structured fetch diagnostics |
| Apple Keychain | keychain item attributes, access groups, accessibility, access control, user presence | accessibility policy, purpose binding, access group metadata, approval/authentication requirement |
| Cloud Key Vault/KMS style systems | versioned secrets/keys, disabled/destroyed versions, access policies, rotation | version state DTOs, disabled/destroyed diagnostics, rotation state, provider-neutral policy |

Design conclusion: Macaca exposes secret references, leases, and provider
resolution contracts. It never treats raw secret bytes/strings as ordinary
command results.

## Goals

- Provide secret reference create/import, inspect, list, bind purpose, resolve
  for provider, create/renew/revoke lease, rotate, inspect version status, and
  audit access operations.
- Support static secrets, dynamic secrets, certificates, private-key references,
  API token references, connection-secret references, and provider-owned opaque
  handles.
- Keep raw secret values out of SDKs, WASM guests, traces, audits, snapshots,
  prompts, config, state, and diagnostics.
- Support expiration, lease renewal, rotation, disabled/destroyed states, and
  provider unavailable diagnostics.
- Support mock and unavailable providers for deterministic tests without
  generating real credentials.

## Non-Goals

- No ordinary app-facing raw secret reveal command.
- No password manager UI or human credential vault UI.
- No encryption/signing API; key and crypto packs may use secret references but
  own cryptographic operations separately.
- No storage of raw secrets in config or key-value state packs.
- No provider-specific secret names, paths, ARNs, Vault mounts, Kubernetes object
  names, or keychain query dictionaries in generic SDK APIs.
- No raw secret values in logs, traces, snapshots, audit records, examples, or
  unavailable diagnostics.

## Ownership And Boundaries

- Pack id: `pack.foundation.secrets.reference.v1`.
- Family: `foundation`.
- Service owner: secret-reference system service.
- Provider examples: AWS Secrets Manager adapter, Vault adapter, Kubernetes
  Secret adapter, Keychain adapter, cloud key vault adapter, mock provider,
  unavailable provider.
- SDK surface: `sdk.packs.foundation.secretsReference`.
- Command namespace: `secrets.*`.
- Microkernel ownership: identity, policy facade, service-call evidence,
  trace/audit primitives only.
- Application framework ownership: manifest declaration, app-scoped permission
  declarations, effective capability projection, WASM ABI import exposure.
- Runtime-host ownership: provider registration, provider-side secret injection,
  redaction decorators, lease cleanup, unavailable provider composition.

## Command Surface

| Command | Supplier analogs | DTO notes | Raw secret exposure |
| --- | --- | --- | --- |
| `secrets.create_reference` | create secret / keychain add / Secret object ref | provider locator, purpose, classification, owner scope | Never |
| `secrets.import_reference` | import existing secret id/path/ARN | external locator, version/stage, policy, metadata | Never |
| `secrets.inspect_reference` | metadata read | reference id, version status, expiry, rotation state | Never |
| `secrets.list_references` | list secrets metadata | prefix/filter, page token, redaction | Never |
| `secrets.bind_purpose` | resource policy / access group | reference id, purpose, allowed service ids, ttl | Never |
| `secrets.resolve_for_provider` | GetSecretValue/Vault read/K8s mount/Keychain access | reference id, service id, purpose, lease request | Provider-only injection |
| `secrets.create_lease` | Vault lease/dynamic secret | reference id, ttl, renewability, purpose | Never |
| `secrets.renew_lease` | lease renew | lease id, ttl extension | Never |
| `secrets.revoke_lease` | revoke/delete lease | lease id, reason | Never |
| `secrets.rotate_reference` | AWS RotateSecret / versioned KV | reference id, rotation policy, idempotency key | Never |
| `secrets.version_status` | version/stage metadata | reference id, version selector | Never |
| `secrets.audit_access` | CloudTrail/Vault audit/keychain access evidence | reference id, time range, filters | Never |

## DTO Model

Core DTOs:

- `SecretReference`: opaque id, provider class, owner scope, classification,
  version selector, stage, expiry, disabled/destroyed state, and trace binding.
- `SecretExternalLocator`: provider-private locator stored only inside provider
  boundary; SDK receives a sanitized hash/alias.
- `SecretPurpose`: provider_auth, api_token, database_password, certificate,
  signing_key_ref, connection_string_ref, generic_sensitive_ref.
- `SecretAccessPolicy`: allowed service ids, allowed app ids, tenant scope,
  approval requirement, ttl bounds, rotation requirement, network scope.
- `SecretLeaseRef`: opaque lease id, reference id, ttl, renewable flag,
  provider class, expiry, and revocation state.
- `SecretResolutionHandle`: provider-only handle passed to an admitted provider,
  not to application code.
- `SecretVersionStatus`: current, previous, pending, disabled, destroyed,
  expired, unknown.
- `SecretAuditRecord`: reference id hash, service id, purpose, decision, lease
  id hash, bounded reason code, timestamp, trace id.
- `SecretError`: denied, not_found, disabled, destroyed, expired,
  rotation_required, lease_expired, invalid_purpose, unsupported, unavailable,
  provider_failure, raw_secret_forbidden.

## Permission And Policy Model

Permission scopes:

- `secrets.reference.read`
- `secrets.reference.create`
- `secrets.reference.import`
- `secrets.reference.list`
- `secrets.reference.bind`
- `secrets.reference.resolve`
- `secrets.reference.lease`
- `secrets.reference.rotate`
- `secrets.reference.revoke`
- `secrets.reference.audit`

Policy rules:

- Every command is scoped to tenant id, app id, session id, task id, reference
  id, service id, purpose, and trace id when available.
- `resolve_for_provider` requires declared reference, allowed service id,
  matching purpose, provider health, entitlement, resource budget, and optional
  approval before provider-side injection.
- Raw secret values are forbidden as app-facing command results and forbidden in
  trace/audit/snapshot/diagnostic payloads.
- Rotation, import, purpose binding, and revoke operations require elevated
  permission and may require approval.
- Lease renewal must check max ttl, renewability, provider status, and policy.
- Disabled, destroyed, expired, or rotation-required references return
  structured diagnostics and do not resolve.

## SDK And Developer Documentation

SDK discovery returns command schemas, reference classes, purpose classes,
permission scopes, policy templates, provider availability, lease/rotation
support, health, examples, docs link, and unavailable diagnostics.

Required developer guide:

- Path: `docs/developer-packs/foundation/secrets-reference.md`.
- Content: reference-only model, manifest declarations, purpose binding,
  permission scopes, policy defaults, provider resolution flow, leases, rotation,
  revocation, version status, audit access, raw-secret prohibitions, unavailable
  diagnostics, provider replacement, trace/audit fields, and examples.
- Examples: declare secret reference, inspect metadata, bind purpose to an LLM or
  gateway service, resolve for provider without exposing raw value, rotate,
  revoke lease, denied raw secret access, and unavailable provider diagnostics.

## Trace, Audit, Health, Snapshot, And Replay

Required event names:

- `secrets_pack_declared`
- `secrets_pack_admission_validated`
- `secrets_pack_policy_decision`
- `secrets_pack_reference_created`
- `secrets_pack_reference_imported`
- `secrets_pack_purpose_bound`
- `secrets_pack_resolve_requested`
- `secrets_pack_provider_injection_succeeded`
- `secrets_pack_provider_injection_failed`
- `secrets_pack_lease_created`
- `secrets_pack_lease_renewed`
- `secrets_pack_lease_revoked`
- `secrets_pack_reference_rotated`
- `secrets_pack_unavailable`

Events include pack id, service id, command name, trace id, app/session/task
identifiers, reference id hash, purpose, provider class, version status, lease id
hash, policy decision, latency, bounded resource counters, and bounded error
code. Events must not include raw secret values, external provider locators,
credentials, private keys, raw signatures, raw provider payloads, prompts, or
unbounded output.

Health checks include provider registered state, reference metadata availability,
lease support, rotation support, version support, provider-side injection
support, max ttl, audit support, and unavailable reasons.

Snapshots include descriptor version, provider class, reference count, capability
flags, policy template hash, lease metadata hashes, rotation status summaries,
and sanitized replay references.

## Implementation Slices

1. Contract slice: descriptor, command schemas, reference/lease/version/policy
   DTOs, result/error DTOs, health/snapshot DTOs, provider capability report.
2. Admission slice: reference declarations, required/optional behavior,
   permission validation, purpose binding, service mapping, provider capability
   validation.
3. Service slice: secret-reference service trait/provider interface,
   unavailable provider, mock provider, adapter bridges for AWS/Vault/Kubernetes/
   Keychain/cloud key vault providers.
4. SDK slice: discovery, typed command builders, metadata inspection helpers,
   provider resolution request builders, lease/rotation helpers, unavailable
   diagnostics, docs link.
5. WASM/app-runtime slice: expose only declared callable reference commands and
   never expose raw secret values or provider locators to WASM guests.
6. Observability slice: redaction decorators, trace/audit events, raw-secret
   leakage tests, health snapshots, replay tests.
7. Developer-docs slice: complete
   `docs/developer-packs/foundation/secrets-reference.md` and link it from
   catalog metadata.

## Design Patterns

- **Facade**: SDK exposes reference helpers and command builders only.
- **Command**: every operation is a typed command/result.
- **Adapter/Bridge**: AWS, Vault, Kubernetes, Keychain, cloud key vault, mock,
  and unavailable providers adapt to one contract.
- **Strategy**: provider selection, lease behavior, rotation behavior, purpose
  matching, and unavailable behavior are replaceable.
- **Decorator**: trace, policy, entitlement, approval, lease, and redaction wrap
  every command.
- **Specification**: purpose, permission, provider capability, version, lease,
  and raw-secret prohibition rules are executable validators.
- **Memento**: snapshots capture reference/lease metadata without exposing
  secret values.

## Risks And Mitigations

- Risk: application code receives raw secret values.
  Mitigation: no app-facing raw reveal command; provider-side injection handles
  are opaque and policy-bound.
- Risk: logs leak secret locators or values.
  Mitigation: event schemas use hashes/aliases and redaction gates reject raw
  values and provider-private locators.
- Risk: provider-specific paths/ARNs leak into generic SDK.
  Mitigation: external locators remain provider-private DTOs; SDK sees sanitized
  references only.
- Risk: rotation/revocation breaks long-running tasks.
  Mitigation: lease metadata, rotation-required diagnostics, revocation events,
  and recovery-facing replay evidence.
- Risk: secret-reference pack becomes a secret store implementation.
  Mitigation: service owns references and adapters; storage/crypto is provider
  responsibility behind the service boundary.
