# Change: Add Identity Organization Pack

## Why

Macaca applications need `pack.identity.organization.v1` as an industrial
organization management capability for organization records, membership,
invitations, role bindings, group-directory references, organization settings,
and audit export. Mature identity platforms expose these concepts through
organization APIs, directory groups, enterprise RBAC, SCIM groups, invitations,
and directory synchronization, but each provider has different lifecycle,
schema, role, and invitation semantics. Macaca must normalize the organization
boundary without becoming an identity provider, tenant isolation engine, billing
system, application authorization framework, or provider-specific directory
adapter.

This proposal defines organization management as a serviceized,
provider-neutral pack. It gives applications typed organization commands while
keeping concrete IdP, directory, SCIM, RBAC, and unavailable providers behind
replaceable service providers.

## Supplier And API Baseline

The design is based on mature organization, directory, and membership APIs:

- Auth0 Organizations expose organization records, enabled connections, member
  lists, member roles, invitations, branding metadata, and organization-scoped
  login hints.
- Clerk Organizations expose organization records, memberships, invitations,
  organization roles/permissions, metadata, domains, and active membership
  context for application sessions.
- WorkOS Organizations, Directory Sync, and RBAC expose organization records,
  directory-backed users and groups, organization domains, memberships, role
  assignments, invitations, and synchronized identity-provider state.
- Okta Groups and Roles APIs expose groups, group membership, user assignment,
  group rules, directory roles, and application/group relationships that often
  back enterprise organization membership.
- Microsoft Graph exposes groups, teams-backed groups, directory roles,
  group membership, owners, transitive membership, invitations, and external
  user collaboration surfaces.
- Google Admin SDK Directory and Cloud Identity Groups expose groups, members,
  aliases, memberships, nested groups, dynamic groups, and directory-backed
  organization metadata.
- SCIM 2.0 defines Groups resources with display names, member references,
  external IDs, metadata, filtering, pagination, and PATCH operations used by
  many enterprise identity providers.
- GitHub Organizations and Teams are a useful developer-platform reference for
  organization membership, team nesting, invitations, role names, and audit-log
  style evidence, but Macaca must not copy GitHub-specific semantics into OS
  contracts.

Research references:

- Auth0 Organizations API:
  https://auth0.com/docs/manage-users/organizations
- Clerk Organizations:
  https://clerk.com/docs/organizations/overview
- WorkOS Organizations, Directory Sync, and RBAC:
  https://workos.com/docs/organizations,
  https://workos.com/docs/directory-sync, and https://workos.com/docs/rbac
- Okta Groups API and Roles API:
  https://developer.okta.com/docs/api/openapi/okta-management/management/tag/Group/
  and
  https://developer.okta.com/docs/api/openapi/okta-management/management/tag/Role/
- Microsoft Graph groups, directory roles, and invitations:
  https://learn.microsoft.com/graph/api/resources/group,
  https://learn.microsoft.com/graph/api/resources/directoryrole, and
  https://learn.microsoft.com/graph/api/resources/invitation
- Google Admin SDK Directory Groups and Cloud Identity Groups:
  https://developers.google.com/admin-sdk/directory/reference/rest/v1/groups
  and https://cloud.google.com/identity/docs/reference/rest/v1/groups
- SCIM 2.0 schema and protocol:
  https://www.rfc-editor.org/rfc/rfc7643 and
  https://www.rfc-editor.org/rfc/rfc7644
- GitHub REST API organizations and teams:
  https://docs.github.com/rest/orgs/orgs and
  https://docs.github.com/rest/teams/teams

## Macaca Provider-Neutral Mapping

`pack.identity.organization.v1` maps supplier concepts into stable Macaca
contracts:

- Auth0 organizations, Clerk organizations, WorkOS organizations, Microsoft
  groups/organizations, Google groups, Okta groups, SCIM groups, and GitHub
  organizations become `OrganizationRecord` when they represent a durable
  collaboration or administrative container.
- Provider-specific organization IDs, directory group IDs, SCIM external IDs,
  verified domains, aliases, slugs, and display names become
  `OrganizationIdentifier` values with uniqueness scope and verification
  metadata.
- Members, group users, organization users, team users, directory-sync users,
  and SCIM group members become `OrganizationMembership` records that reference
  account/profile subjects without owning account lifecycle or profile fields.
- Owner, admin, member, guest, viewer, directory role, group role, team role,
  and provider permission assignments become `OrganizationRoleBinding` with
  provider-neutral role references and policy-visible privilege class metadata.
- Invitations, join requests, pending members, external collaboration invites,
  and email/domain-based invite flows become `OrganizationInvitation` records
  with expiry, acceptance, revocation, delivery-reference, and redaction state.
- Directory groups, nested groups, dynamic groups, SCIM groups, synced teams,
  and IdP group push references become `DirectoryGroupReference`; the pack
  tracks linkage evidence but does not implement directory synchronization.
- Provider audit logs and organization events become
  `OrganizationAuditReference` and export artifact handles with bounded,
  sanitized metadata.

## What Changes

- Add provider-neutral `pack.identity.organization.v1` under the identity
  family.
- Define commands for provider inspection, schema discovery, organization
  planning, organization creation, read/search, update, archive/restore,
  membership read/search, membership assignment/removal planning and request,
  invitation creation/revocation/resend/inspection, role binding planning and
  request, directory-group link inspection, audit export, and artifact
  retrieval.
- Define DTOs for organization scope, provider capability, organization record,
  identifiers, verified domains, membership records, role references, role
  bindings, invitation records, directory-group references, organization policy
  references, freshness/version metadata, audit references, redaction, and
  artifact handles.
- Require policy, approval for privileged role and invitation changes, tenant
  isolation, account/profile reference validation, membership uniqueness,
  idempotency for mutating commands, sanitized trace/audit, and deterministic
  unavailable/unsupported behavior.
- Require detailed developer documentation at
  `docs/developer-packs/identity/organization.md`.

## Impact

- Affected specs: `pack-identity-organization`,
  `developer-pack-industrial-capability-catalog`, `sdk-system-facade`,
  `service-runtime`, `unified-execution-path`.
- Affected code later: protocol DTOs, pack descriptors, admission validators,
  SDK discovery/command builders, organization service providers,
  mock/unavailable providers, trace/audit schemas, replay tests, redaction
  tests, directory-reference tests, and boundary gates.

## Non-Goals

- No account creation/lifecycle, profile field management, OAuth/OIDC/SAML
  login handoff, token exchange, password or credential handling, tenant
  isolation policy ownership, billing entitlement, payment, subscription,
  receipt, application-specific RBAC, or product workflow authorization.
- No provider-specific organization lifecycle policy, HRIS workflow,
  directory-sync engine, domain verification engine, team collaboration
  workflow, or application business role routing in Macaca OS layers.
- No raw credentials, invite tokens, access tokens, refresh tokens, directory
  sync secrets, raw provider payloads, private keys, signatures, full audit-log
  dumps, or unbounded member/profile data in logs, traces, snapshots, or SDK
  diagnostics.
- No provider construction or provider-name routing in kernel, SDK, shells, or
  generic application framework.
