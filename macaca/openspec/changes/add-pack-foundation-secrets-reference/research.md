# Foundation Secrets Reference Pack Research

## Purpose

This note records supplier/API research for
`pack.foundation.secrets.reference.v1`. The pack is intentionally a
secret-reference capability: applications may declare, inspect, bind, rotate,
lease, revoke, and audit references, but raw secret values must not become
ordinary application, SDK, WASM, trace, audit, snapshot, or diagnostic data.

## Source Baseline

- AWS Secrets Manager `GetSecretValue`:
  <https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_GetSecretValue.html>
- AWS Secrets Manager CloudTrail logging:
  <https://docs.aws.amazon.com/secretsmanager/latest/userguide/monitoring-cloudtrail.html>
- AWS Secrets Manager resource policies:
  <https://docs.aws.amazon.com/secretsmanager/latest/userguide/auth-and-access_resource-policies.html>
- HashiCorp Vault KV secrets engine:
  <https://developer.hashicorp.com/vault/docs/secrets/kv>
- HashiCorp Vault policies:
  <https://developer.hashicorp.com/vault/docs/concepts/policies>
- Kubernetes Secrets:
  <https://kubernetes.io/docs/concepts/configuration/secret/>
- Apple Keychain item accessibility:
  <https://developer.apple.com/documentation/security/restricting-keychain-item-accessibility>
- Apple Keychain item sharing:
  <https://developer.apple.com/documentation/security/sharing-access-to-keychain-items-among-a-collection-of-apps>
- Azure Key Vault security:
  <https://learn.microsoft.com/en-us/azure/key-vault/general/secure-key-vault>
- Google Cloud Secret Manager audit logging:
  <https://docs.cloud.google.com/secret-manager/docs/audit-logging>
- Google Cloud Secret Manager delayed secret version destruction:
  <https://docs.cloud.google.com/secret-manager/docs/delay-destruction-of-secret-versions>

## AWS Secrets Manager Summary

AWS Secrets Manager contributes versioned retrieval, rotation, resource policy,
and audit concepts:

- Secret values have version ids and staging labels; current/previous/pending
  stages map to Macaca version-status DTOs.
- Rotation is a first-class lifecycle operation and must be auditable.
- Resource policies and confused-deputy mitigations map to allowed service ids,
  allowed app ids, purpose binding, tenant scope, and policy decisions.
- CloudTrail records Secrets Manager API calls and rotation/deletion events.
  Macaca should record access decisions and provider injection events without
  logging raw secret values or sensitive request parameters.
- `GetSecretValue` is not exposed as a normal app-facing command. Macaca maps
  it to provider-side resolution after policy and approval.

## HashiCorp Vault Summary

Vault contributes mounts, versioned KV, dynamic secrets, leases, policies, and
audit behavior:

- Secret engines are mounted providers. Macaca should model provider class,
  external locator, mount/source ref, and capability diagnostics.
- KV v2 version metadata maps to `SecretVersionStatus`.
- Dynamic secrets and leases map to `SecretLeaseRef`, renewability, ttl, expiry,
  and revocation state.
- Policies are path/operation based and deny by default. Macaca should map this
  to permission scopes, purpose binding, and allowed service ids.
- Audit data must be sanitized and bounded. Raw secret values and provider
  private locators do not enter Macaca generic observability.

## Kubernetes Secrets Summary

Kubernetes contributes object references and injection patterns:

- Pods can consume Secrets through environment variables or volume mounts; the
  stable concept is provider-side injection, not application retrieval.
- Secret references are validated before pods run, and the kubelet reports retry
  or fetch failures as Events. Macaca should perform admission validation and
  return structured unavailable/not-found diagnostics.
- Secret objects live in a scope/namespace. Macaca should scope secret refs to
  tenant/app/session/task and allowed provider/service use.
- Kubernetes object names and volume/env projection mechanics must not become
  the SDK/ABI contract.

## Apple Keychain Summary

Apple Keychain contributes item accessibility, access groups, and
user/device-authentication constraints:

- Keychain items have accessibility constraints tied to device lock state and
  user/device authentication.
- Access groups allow controlled sharing across apps. Macaca maps this to
  allowed app ids, allowed service ids, and purpose-bound policy.
- User presence or device authentication maps to approval/authentication
  requirements before provider resolution.
- Keychain query dictionaries and platform item classes must remain provider
  details.

## Cloud Key Vault / KMS Style Summary

Azure Key Vault and Google Secret Manager provide versioned secret/key/cert
storage, access policy or IAM, disabled/destroyed states, rotation, and audit:

- Key vaults protect secrets, keys, and certificates with explicit access
  controls and operational hardening.
- Google Cloud Secret Manager records audit logs for administrative and access
  activity.
- Secret versions can have disabled or destruction-related states, which map to
  Macaca `disabled`, `destroyed`, `expired`, and `rotation_required` results.
- Cloud key vault/KMS providers often own cryptographic operations. Macaca
  secret references may point at signing-key references, but signing/encryption
  is owned by separate crypto/key packs.

## Macaca-Owned Abstractions

`pack.foundation.secrets.reference.v1` should define these provider-neutral
concepts:

- `SecretReference`: opaque id, owner scope, provider class, classification,
  version selector, stage, expiry, disabled/destroyed state, and trace binding.
- `SecretExternalLocator`: provider-private location, visible to generic SDKs
  only as a sanitized alias or hash.
- `SecretPurpose`: provider_auth, api_token, database_password, certificate,
  signing_key_ref, connection_string_ref, generic_sensitive_ref, and extension
  values.
- `SecretAccessPolicy`: allowed service ids, app ids, tenant scope, approval
  requirement, ttl bounds, rotation requirement, and network/resource bounds.
- `SecretLeaseRef`: opaque lease id, reference id, ttl, renewability, provider
  class, expiry, and revocation state.
- `SecretResolutionHandle`: provider-only handle created after policy approval;
  it is not returned to application code or WASM guests as a raw value.
- `SecretVersionStatus`: current, previous, pending, disabled, destroyed,
  expired, rotation_required, and unknown.
- `SecretAuditRecord`: reference id hash, service id, purpose, decision, lease
  id hash, bounded reason code, timestamp, trace id, and replay pointer.
- `SecretProviderCapability`: supported reference types, versioning, rotation,
  leases, provider injection, audit, disabled/destroyed state detection, health,
  and unavailable reasons.

## Rejected API Leakage

Macaca must not expose these provider-native shapes as stable SDK/ABI contracts:

- AWS ARNs, staging labels as provider-native strings, CloudTrail event payloads,
  resource policy JSON, or `GetSecretValue` responses as app-facing data.
- Vault paths, mounts, secret engine request/response bodies, policy syntax,
  dynamic secret payloads, or lease protocol details.
- Kubernetes Secret object schemas, namespace/name references, env/volume
  injection YAML, kubelet event payloads, or API object watches.
- Apple Keychain query dictionaries, access-group entitlements, accessibility
  constants, or platform authentication handles.
- Azure/GCP vault resource identifiers, IAM/RBAC policy payloads, key vault
  version handles, or audit-log provider payloads.
- Raw secret values, provider-private locators, credentials, private keys,
  signatures, certificates as raw bytes, or unbounded audit exports.

All operations must enter through typed Macaca service commands with trace
context, policy checks, approval where required, structured result envelopes,
sanitized audit events, unavailable provider behavior, and provider replacement
support.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
