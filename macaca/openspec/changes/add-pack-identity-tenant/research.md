# Identity Tenant Pack Research

## Purpose

This note records supplier/API research, Macaca provider-neutral mapping,
explicit non-goals, existing platform inventory, and GitNexus memo evidence for
`pack.identity.tenant.v1`. The tenant pack owns tenant records, identifiers,
lifecycle state, isolation policy references, quota envelopes, usage snapshots,
residency hints, config references, relationship references, audit references,
artifact handles, freshness, attribution, and redaction. It must not own account
lifecycle, profile fields, auth handoff, organization membership/invitations,
licensing, commerce billing, workflow approvals/reviews, raw secrets/config
values, or cloud resource provisioning.

## Source Baseline

- Microsoft Graph organization resource and tenant permissions:
  <https://learn.microsoft.com/en-us/graph/api/resources/organization?view=graph-rest-1.0>
  and <https://learn.microsoft.com/en-us/graph/permissions-reference>
- Auth0 tenant settings and Management API:
  <https://auth0.com/docs/get-started/tenant-settings> and
  <https://auth0.com/docs/api/management/v2>
- Okta org settings and Core Okta API:
  <https://developer.okta.com/docs/reference/core-okta-api/>
- Google Workspace customers and organizational units:
  <https://developers.google.com/workspace/admin/directory/reference/rest/v1/customers>
  and
  <https://developers.google.com/workspace/admin/directory/v1/guides/manage-org-units>
- AWS Organizations:
  <https://docs.aws.amazon.com/organizations/latest/APIReference/API_Organization.html>
  and
  <https://docs.aws.amazon.com/organizations/latest/APIReference/API_OrganizationalUnit.html>
- Azure management groups and subscriptions:
  <https://learn.microsoft.com/en-us/azure/governance/management-groups/overview>
- Kubernetes namespaces and resource quotas:
  <https://kubernetes.io/docs/concepts/overview/working-with-objects/namespaces/>
  and <https://kubernetes.io/docs/concepts/policy/resource-quotas/>
- SCIM 2.0 and OIDC issuer semantics:
  <https://datatracker.ietf.org/doc/html/rfc7644> and
  <https://openid.net/specs/openid-connect-core-1_0.html>

## Supplier API Notes

- Microsoft Graph organization represents the authenticated Entra tenant and
  exposes tenant-level details, permissions, and update constraints. Macaca
  should map this into tenant records with explicit permission gates.
- Auth0 and Okta tenant/org settings contribute issuer, branding, connection,
  policy, and security-setting references. Macaca should represent these as
  bounded config references and policy references, never raw config values or
  secrets.
- Google Workspace customers and org units contribute customer identity,
  hierarchy, and administrative partitioning. Macaca should model hierarchy and
  residency hints without merging tenant and organization membership semantics.
- AWS Organizations, Azure management groups/subscriptions, and Kubernetes
  namespaces/resource quotas contribute hierarchy, quota, policy attachment, and
  isolation patterns. Macaca should use them as infrastructure baselines without
  provisioning cloud resources from this pack.
- SCIM and OIDC contribute tenant-adjacent issuer, service provider, and
  identity-domain semantics. Macaca should keep issuer references and tenant
  records provider-neutral.

## Macaca-Owned Abstractions

`pack.identity.tenant.v1` should define `TenantScope`, `TenantRecord`,
`TenantIdentifier`, `TenantLifecycleState`,
`TenantIsolationPolicyReference`, `TenantQuotaEnvelope`,
`TenantUsageSnapshot`, `TenantResidencyHint`, `TenantConfigReference`,
`TenantRelationshipReference`, `TenantAuditReference`,
`TenantArtifactHandle`, `TenantFreshness`, `TenantAttribution`, and
`TenantRedactionPolicy`.

The DTOs must carry tenant scope, issuer/reference identifiers, lifecycle
state, isolation policy handle, quota dimension, reserved capacity, usage
window, residency hint, config reference class, relationship type, version
token, freshness, attribution, bounded reason code, artifact checksum,
redaction class, and replay pointers. Raw credentials, client secrets, access
tokens, refresh tokens, private keys, signatures, raw config values, raw
provider payloads, raw manifests, raw package bytes, unbounded tenant lists, and
raw audit exports are rejected.

## Explicit Non-Goals

- Do not implement concrete Microsoft, Auth0, Okta, Google, AWS, Azure,
  Kubernetes, SCIM, OIDC, quota, policy, credential, or tenant-provider adapters
  in this research phase.
- Do not perform account lifecycle, profile field updates, auth handoff,
  organization membership/invitation changes, entitlement licensing, commerce
  billing, workflow approvals/reviews, raw secret/config management, or cloud
  resource provisioning.
- Do not expose provider-native tenant payloads, cloud account objects, raw
  configuration, raw secrets, billing state, or product-specific tenant policy
  as stable SDK contracts.

## Existing Macaca Platform Inventory

- `crates/foundation/macaca-proto/src/domain_pack_contract/` provides descriptor,
  lifecycle, policy, diagnostics, SDK metadata, provider snapshot, unavailable,
  and effective capability primitives reusable by this pack.
- `crates/facade/macaca-sdk/src/system_facade.rs` provides the Facade pattern;
  tenant SDK helpers should only build canonical traced service calls.
- Generic policy, approval, resource, entitlement, trace, audit, artifact,
  mock-provider, unavailable-provider, config-reference, and secrets-reference
  concepts are reusable, but current evidence does not prove tenant-specific
  DTOs, descriptors, providers, SDK helpers, ABI metadata, tests, or developer
  docs.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
