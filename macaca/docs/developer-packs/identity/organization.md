# Identity Organization Pack

`pack.identity.organization.v1` is the provider-neutral organization-management
contract. It covers organization records, identifiers, memberships,
invitations, role references, role bindings, directory-group references,
policy references, audit references, freshness/version metadata, and artifact
handles. It does not own account lifecycle, profile fields, auth handoff,
tenant isolation policy, billing, payment, application RBAC, HR workflows, or
message delivery.

## Manifest

```toml
[service_contract]
optional_packs = ["pack.identity.organization.v1"]
```

Required declarations are appropriate only when organization management is a
hard runtime dependency and an organization provider is installed.

## Permission Scopes

- `identity.organization.read`
- `identity.organization.search`
- `identity.organization.write`
- `identity.organization.archive`
- `identity.organization.membership.read`
- `identity.organization.membership.write`
- `identity.organization.invitation.read`
- `identity.organization.invitation.write`
- `identity.organization.role.read`
- `identity.organization.role.write`
- `identity.organization.directory.read`
- `identity.organization.audit.export`
- `identity.organization.artifact.read`

External invitations, elevated role bindings, final-owner removal, archive or
restore, audit export, and directory-managed membership mutation require
approval.

## Commands

- `organization.inspect_provider`
- `organization.discover_schema`
- `organization.plan_create`
- `organization.create`
- `organization.get`
- `organization.search`
- `organization.plan_update`
- `organization.update`
- `organization.archive`
- `organization.restore`
- `organization.list_members`
- `organization.get_membership`
- `organization.plan_membership_change`
- `organization.request_membership_change`
- `organization.create_invitation`
- `organization.resend_invitation`
- `organization.revoke_invitation`
- `organization.inspect_invitation`
- `organization.plan_role_binding`
- `organization.request_role_binding`
- `organization.list_role_bindings`
- `organization.inspect_directory_links`
- `organization.export_audit`
- `organization.get_artifact`

Planning commands validate policy, entitlement, directory-managed conflicts,
role privilege class, version preconditions, quotas, and approval requirements
before side effects.

## DTO Model

Primary DTOs include `OrganizationScope`, `OrganizationProviderCapability`,
`OrganizationRecord`, `OrganizationIdentifier`, `OrganizationLifecycleState`,
`OrganizationMembership`, `OrganizationMembershipState`,
`OrganizationInvitation`, `OrganizationRoleReference`,
`OrganizationRoleBinding`, `DirectoryGroupReference`,
`OrganizationPolicyReference`, `OrganizationAuditReference`, and
`OrganizationArtifactHandle`.

Raw credentials, invite tokens, access tokens, refresh tokens, directory sync
secrets, raw provider payloads, full member lists beyond requested pages,
private profile fields, raw audit exports, private keys, signatures, and
unbounded output must not enter observability.

## App-Facing Examples

Generic examples cover organization creation, organization read/search,
membership list/change, invitation create/revoke, role binding,
directory-link inspection, audit export, artifact handles, and unavailable
diagnostics. Applications use synthetic organization, membership, invitation,
role, directory, audit, and artifact refs through typed SDK commands.

Diagnostic examples cover provider unavailable, missing permission, missing
entitlement, directory-managed conflict, unsupported role, unsupported
invitation, stale version, approval required, quota exceeded, audit export
denied, and artifact denied. Diagnostics must not include provider names,
credentials, private profile data, raw invite tokens, raw provider payloads,
raw audit exports, or application business workflows.

## Unavailable Behavior

The descriptor is preview-unavailable until a provider registers
`service.identity.organization`. SDK discovery reports
`identity_organization_provider_not_installed`.

## Provider Replacement

Provider classes are `organization-record`, `organization-membership`,
`organization-invitation-role`, `mock`, and `unavailable`. Provider-specific
groups, teams, roles, invitations, and directory links are adapted through
Strategy providers and descriptor metadata.

## Trace And Audit

Trace evidence records organization, membership, invitation, role, directory,
policy, and artifact handles with bounded codes and hashes. Shells may render
events but must not own membership, role, or invitation semantics.

## Boundaries

Use account/profile packs for subject data, auth handoff for login, tenant pack
for isolation/quota, workflow packs for approvals/reviews, communication packs
for delivery, and commerce packs for billing or entitlements.
