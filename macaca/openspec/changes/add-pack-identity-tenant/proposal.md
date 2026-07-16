# Change: Add Identity Tenant Pack

## Why

Macaca applications need `pack.identity.tenant.v1` as an industrial tenancy
capability for tenant records, tenant identifiers, isolation policy references,
quota and budget envelopes, residency/region hints, tenant-scoped configuration
references, lifecycle state, and audit export. Modern identity, cloud, and
platform APIs expose tenant-like concepts through directories, enterprise
customers, organizations, accounts, subscriptions, namespaces, resource quotas,
and policy attachments. Macaca must normalize the tenancy boundary without
becoming an identity provider, cloud account manager, billing system, workflow
engine, or application-specific multitenancy framework.

This proposal defines tenant management as a serviceized, provider-neutral pack.
It gives applications typed tenant commands while keeping concrete directory,
cloud, workspace, namespace, quota, policy, and unavailable providers behind
replaceable service providers.

## Supplier And API Baseline

The design is based on mature tenant, directory, cloud-account, and resource
isolation APIs:

- Microsoft Entra ID and Microsoft Graph expose tenants/directories through
  organization resources, tenant identifiers, verified domains, directory
  roles, subscriptions, and tenant-aware issuer/audience semantics.
- Auth0 exposes tenant settings, custom domains, enabled connections, logs, and
  organization features that often sit under a tenant-level administrative
  boundary.
- Okta exposes an org-level administrative boundary with organization settings,
  domains, authorization servers, groups, roles, policies, logs, and lifecycle
  controls.
- Google Workspace Admin SDK exposes Customers and Organizational Units, while
  Cloud Identity Groups provides directory-scoped group and membership data used
  by tenant-level administration.
- AWS Organizations exposes accounts, organizational units, service control
  policies, tag policies, delegated administration, and consolidated audit-style
  evidence for multi-account tenancy.
- Azure management groups, subscriptions, resource groups, and policy
  assignments expose cloud tenancy/resource-scope patterns that inform quota,
  policy, and hierarchy modeling without becoming cloud-management semantics.
- Kubernetes namespaces and ResourceQuota expose a compact infrastructure
  pattern for tenant-like isolation, scoped resources, quotas, and admission
  policy within one control plane.
- SCIM and OIDC provide interoperable tenant-adjacent evidence through external
  IDs, issuer/subject/audience boundaries, directory groups, and schema
  metadata.

Research references:

- Microsoft Graph organization resource:
  https://learn.microsoft.com/graph/api/resources/organization
- Microsoft identity platform tenants:
  https://learn.microsoft.com/entra/identity-platform/single-and-multi-tenant-apps
- Auth0 tenant settings:
  https://auth0.com/docs/get-started/tenant-settings
- Okta Organizations and management APIs:
  https://developer.okta.com/docs/api/openapi/okta-management/management/tag/OrgSetting/
- Google Admin SDK Customers and Organizational Units:
  https://developers.google.com/admin-sdk/directory/reference/rest/v1/customers
  and
  https://developers.google.com/admin-sdk/directory/reference/rest/v1/orgunits
- AWS Organizations API:
  https://docs.aws.amazon.com/organizations/latest/APIReference/Welcome.html
- Azure management groups and subscriptions:
  https://learn.microsoft.com/azure/governance/management-groups/overview
  and https://learn.microsoft.com/azure/azure-resource-manager/management/overview
- Kubernetes namespaces and ResourceQuota:
  https://kubernetes.io/docs/concepts/overview/working-with-objects/namespaces/
  and https://kubernetes.io/docs/concepts/policy/resource-quotas/
- SCIM 2.0 and OpenID Connect Core:
  https://www.rfc-editor.org/rfc/rfc7643,
  https://www.rfc-editor.org/rfc/rfc7644, and
  https://openid.net/specs/openid-connect-core-1_0.html

## Macaca Provider-Neutral Mapping

`pack.identity.tenant.v1` maps supplier concepts into stable Macaca contracts:

- Entra tenants/directories, Auth0 tenants, Okta org boundaries, Google
  customers/org units, AWS accounts/OUs, Azure subscriptions/resource scopes,
  Kubernetes namespaces, and internal workspace partitions become
  `TenantRecord` when they represent an isolation or administrative boundary.
- Tenant IDs, directory IDs, customer IDs, account IDs, subscription IDs,
  namespace names, verified domains, issuer IDs, external IDs, aliases, and
  slugs become `TenantIdentifier` values with uniqueness scope and verification
  metadata.
- Tenant lifecycle states such as active, suspended, disabled, archived,
  pending deletion, deleted, locked, degraded, and provider unknown become
  `TenantLifecycleState`.
- Policy assignments, service control policies, authorization server settings,
  resource policies, data-residency requirements, admission policies, and
  tenant guardrails become `TenantIsolationPolicyReference`; policy engines stay
  behind Macaca policy services.
- Subscription quotas, namespace resource quotas, API limits, storage limits,
  budget envelopes, concurrency limits, artifact retention limits, and provider
  rate limits become `TenantQuotaEnvelope` and `TenantUsageSnapshot`.
- Tenant-level configuration, feature toggles, custom domains, connection
  references, issuer metadata, and integration settings become
  `TenantConfigReference`; raw secrets belong to secrets-reference packs.
- Memberships and roles are represented only as references to organization,
  account, and policy evidence; membership management belongs to
  `pack.identity.organization.v1` and account/profile packs.
- Provider logs and tenant audit trails become `TenantAuditReference` and
  bounded export artifact handles.

## What Changes

- Add provider-neutral `pack.identity.tenant.v1` under the identity family.
- Define commands for provider inspection, schema discovery, tenant planning,
  tenant creation, read/search, update, lifecycle transition planning/request,
  isolation policy inspection, policy attachment planning/request, quota
  inspection, quota reservation planning/request, usage snapshot, residency
  inspection, config reference inspection/update, tenant relationship
  inspection, audit export, and artifact retrieval.
- Define DTOs for tenant scope, provider capability, tenant record, identifiers,
  lifecycle state, isolation policy references, quota envelopes, usage
  snapshots, residency hints, config references, relationship references,
  audit references, freshness/version metadata, redaction, and artifact handles.
- Require policy, approval for high-impact tenant changes, resource/quota
  checks, entitlement checks, tenant isolation, idempotency for mutating
  commands, sanitized trace/audit, and deterministic unavailable/unsupported
  behavior.
- Require detailed developer documentation at
  `docs/developer-packs/identity/tenant.md`.

## Impact

- Affected specs: `pack-identity-tenant`,
  `developer-pack-industrial-capability-catalog`, `sdk-system-facade`,
  `service-runtime`, `unified-execution-path`.
- Affected code later: protocol DTOs, pack descriptors, admission validators,
  SDK discovery/command builders, tenant service providers,
  mock/unavailable providers, trace/audit schemas, replay tests, redaction
  tests, quota/policy tests, and boundary gates.

## Non-Goals

- No account lifecycle, profile field management, auth handoff, token exchange,
  organization membership management, application RBAC, billing entitlement,
  payment, invoice, receipt, cloud resource provisioning, Kubernetes controller
  implementation, or application-specific multitenancy workflow.
- No provider-specific cloud governance, org-chart hierarchy, HRIS workflow,
  custom domain verification engine, billing plan logic, or product feature
  routing in Macaca OS layers.
- No raw credentials, client secrets, access tokens, refresh tokens, private
  keys, signatures, raw provider payloads, full audit logs, raw manifests,
  package bytes, or unbounded usage data in logs, traces, snapshots, or SDK
  diagnostics.
- No provider construction or provider-name routing in kernel, SDK, shells, or
  generic application framework.
