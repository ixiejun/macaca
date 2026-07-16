# Identity Tenant Pack

`pack.identity.tenant.v1` is the provider-neutral tenancy contract. It covers
tenant records, identifiers, lifecycle state, isolation policy references,
quota envelopes, usage snapshots, residency hints, config references,
relationship references, audit references, and artifact handles. It does not
own accounts, profiles, auth handoff, organization membership, billing,
payments, cloud provisioning, Kubernetes controllers, or application-specific
multitenancy workflows.

## Manifest

```toml
[service_contract]
optional_packs = ["pack.identity.tenant.v1"]
```

Use required declarations only when the app cannot safely run without tenant
metadata and quota/policy references.

## Permission Scopes

- `identity.tenant.read`
- `identity.tenant.search`
- `identity.tenant.write`
- `identity.tenant.lifecycle`
- `identity.tenant.policy.read`
- `identity.tenant.policy.write`
- `identity.tenant.quota.read`
- `identity.tenant.quota.reserve`
- `identity.tenant.usage.read`
- `identity.tenant.residency.read`
- `identity.tenant.config.read`
- `identity.tenant.config.write`
- `identity.tenant.relationship.read`
- `identity.tenant.audit.export`
- `identity.tenant.artifact.read`

Tenant creation, deletion, archive/restore, policy attachment, residency
change, external custom-domain change, quota-limit change, large usage export,
audit export, and auth-affecting config references require approval.

## Commands

- `tenant.inspect_provider`
- `tenant.discover_schema`
- `tenant.plan_create`
- `tenant.create`
- `tenant.get`
- `tenant.search`
- `tenant.plan_update`
- `tenant.update`
- `tenant.plan_lifecycle_transition`
- `tenant.request_lifecycle_transition`
- `tenant.inspect_isolation_policy`
- `tenant.plan_policy_attachment`
- `tenant.request_policy_attachment`
- `tenant.inspect_quota`
- `tenant.plan_quota_reservation`
- `tenant.request_quota_reservation`
- `tenant.snapshot_usage`
- `tenant.inspect_residency`
- `tenant.inspect_config`
- `tenant.update_config_reference`
- `tenant.inspect_relationships`
- `tenant.export_audit`
- `tenant.get_artifact`

Quota reservations are resource-policy evidence, not billing entitlement or
license grants.

## DTO Model

Primary DTOs include `TenantScope`, `TenantProviderCapability`, `TenantRecord`,
`TenantIdentifier`, `TenantLifecycleState`,
`TenantIsolationPolicyReference`, `TenantQuotaEnvelope`,
`TenantUsageSnapshot`, `TenantResidencyHint`, `TenantConfigReference`,
`TenantRelationshipReference`, `TenantAuditReference`, and
`TenantArtifactHandle`.

Config values are references only. Raw credentials, client secrets, access
tokens, refresh tokens, private keys, signatures, raw provider payloads, raw
manifests, package bytes, raw audit exports, full usage exports, unbounded
tenant lists, and unbounded output must not enter observability.

## App-Facing Examples

Generic examples cover tenant creation, tenant read/search, lifecycle
transition, policy inspection/attachment, quota inspection/reservation, usage
snapshot, residency inspection, config reference update, relationship
inspection, audit export, artifact handles, and unavailable diagnostics.
Applications use synthetic tenant, policy, quota, usage, residency, config,
relationship, audit, and artifact refs through typed SDK commands.

Diagnostic examples cover provider unavailable, missing permission, missing
entitlement, unsupported policy, unsupported quota, unsupported residency,
stale version, approval required, quota exceeded, config secret denied, audit
export denied, and artifact denied. Diagnostics must not include provider
names, credentials, raw config values, raw provider payloads, raw audit logs, or
application business workflows.

## Unavailable Behavior

The descriptor is preview-unavailable until a provider registers
`service.identity.tenant`. SDK discovery reports
`identity_tenant_provider_not_installed`.

## Provider Replacement

Provider classes are `tenant-record`, `tenant-policy`, `tenant-quota`,
`tenant-config`, `mock`, and `unavailable`. Cloud, directory, namespace, quota,
policy, local, remote, plugin, mock, and unavailable providers are composed only
through approved runtime-host or plugin roots.

## Trace And Audit

Trace evidence records tenant handles, policy refs, quota dimensions, usage
snapshot refs, residency refs, config refs, relationship refs, provider class,
descriptor hash, idempotency hash, version hash, bounded counters, and artifact
handles.

## Boundaries

Use account, profile, auth handoff, and organization packs for identity subject
behavior. Use foundation config and secrets-reference packs for config/secret
handles. Use commerce entitlement and payment packs for billing rights. Use
application code for product-specific multitenancy behavior.
