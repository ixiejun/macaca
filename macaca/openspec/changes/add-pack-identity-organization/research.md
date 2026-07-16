# Identity Organization Pack Research

## Purpose

This note records supplier/API research, Macaca provider-neutral mapping,
explicit non-goals, existing platform inventory, and GitNexus memo evidence for
`pack.identity.organization.v1`. The organization pack owns organization
records, identifiers, lifecycle state, memberships, invitations, role
references, role bindings, directory group references, organization policy
references, audit references, artifact handles, freshness, attribution, and
redaction. It must not own account lifecycle, profile fields, auth handoff,
token exchange, tenant isolation/quota, entitlement licensing, workflow
approvals/reviews, message delivery, or application product RBAC.

## Source Baseline

- Auth0 Organizations:
  <https://auth0.com/docs/api/management/v2/organizations/get-organizations>
- Clerk Organizations:
  <https://clerk.com/docs/organizations/overview>
- WorkOS Organizations, Directory Sync, and RBAC:
  <https://workos.com/docs/reference/organization>,
  <https://workos.com/docs/directory-sync>, and
  <https://workos.com/docs/fga>
- Okta Groups and roles:
  <https://developer.okta.com/docs/api/openapi/okta-management/management/tag/Group/>
  and
  <https://developer.okta.com/docs/api/openapi/okta-management/management/tag/RoleAssignmentBETA/>
- Microsoft Graph groups, directory roles, and invitations:
  <https://learn.microsoft.com/en-us/graph/api/resources/group?view=graph-rest-1.0>,
  <https://learn.microsoft.com/en-us/graph/api/resources/directoryrole?view=graph-rest-1.0>,
  and <https://learn.microsoft.com/en-us/graph/api/resources/invitation?view=graph-rest-1.0>
- Google Admin SDK groups:
  <https://developers.google.com/workspace/admin/directory/reference/rest/v1/groups>
- SCIM 2.0 Group schema and protocol:
  <https://datatracker.ietf.org/doc/html/rfc7643> and
  <https://datatracker.ietf.org/doc/html/rfc7644>
- GitHub Organizations and Teams:
  <https://docs.github.com/en/rest/orgs/orgs> and
  <https://docs.github.com/en/rest/teams/teams>

## Supplier API Notes

- Auth0, Clerk, and WorkOS model organizations as customer/workspace boundaries
  with members, roles, invitations, domains, and enterprise identity
  connections. Macaca should normalize organization lifecycle and membership
  commands without adopting product-specific RBAC semantics.
- WorkOS Directory Sync and SCIM Groups contribute directory-managed group and
  membership synchronization semantics. Macaca should preserve directory-managed
  conflict evidence and prevent OS-layer mutation when the provider marks a
  resource as externally managed.
- Okta and Microsoft Graph contribute groups, administrative roles, directory
  roles, invitations, and membership surfaces. Macaca should model privilege
  class and approval requirements before role or membership mutations.
- Google Admin and GitHub contribute group/team membership, aliases, and
  role-like permissions. Macaca should map them into provider-neutral
  organization, role binding, and directory link references.

## Macaca-Owned Abstractions

`pack.identity.organization.v1` should define `OrganizationScope`,
`OrganizationRecord`, `OrganizationIdentifier`,
`OrganizationLifecycleState`, `OrganizationMembership`,
`OrganizationMembershipState`, `OrganizationInvitation`,
`OrganizationRoleReference`, `OrganizationRoleBinding`,
`DirectoryGroupReference`, `OrganizationPolicyReference`,
`OrganizationAuditReference`, `OrganizationArtifactHandle`,
`OrganizationFreshness`, `OrganizationAttribution`, and
`OrganizationRedactionPolicy`.

The DTOs must carry tenant scope, organization handle, identifier class,
domain/reference metadata, lifecycle state, member subject reference, role
privilege class, invitation recipient class, directory-managed state, version
token, freshness, attribution, bounded reason codes, artifact checksum,
redaction class, and replay pointers. Raw invite tokens, raw directory sync
secrets, raw provider payloads, private profile fields, unbounded member lists,
raw audit exports, credentials, tokens, private keys, and signatures are
rejected.

## Explicit Non-Goals

- Do not implement concrete Auth0, Clerk, WorkOS, Okta, Microsoft Graph,
  Google, SCIM, GitHub, directory-sync, invitation-delivery, credential, or
  organization-provider adapters in this research phase.
- Do not perform account lifecycle, profile field updates, auth handoff, token
  exchange, tenant isolation/quota changes, entitlement licensing, workflow
  approvals/reviews, message delivery, or application product RBAC evaluation.
- Do not expose provider-native organization payloads, raw role schemas,
  provider-specific invitation workflows, raw invite tokens, or business-specific
  authorization rules as stable SDK contracts.

## Existing Macaca Platform Inventory

- `crates/foundation/macaca-proto/src/domain_pack_contract/` provides descriptor,
  lifecycle, policy, diagnostics, SDK metadata, provider snapshot, unavailable,
  and effective capability primitives reusable by this pack.
- `crates/facade/macaca-sdk/src/system_facade.rs` provides the Facade pattern;
  organization SDK helpers should only build canonical traced service calls.
- Generic policy, approval, resource, entitlement, trace, audit, artifact,
  mock-provider, and unavailable-provider concepts are reusable, but current
  evidence does not prove organization-specific DTOs, descriptors, providers,
  SDK helpers, ABI metadata, tests, or developer docs.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
