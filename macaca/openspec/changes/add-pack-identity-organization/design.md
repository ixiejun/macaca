# Identity Organization Pack Design

## Context

`pack.identity.organization.v1` is a child proposal of the developer-pack
industrial capability catalog. It provides a serviceized organization
management surface for application developers while preserving Macaca's
microkernel model. The pack owns provider-neutral organization, membership,
invitation, role-binding, directory-group-reference, and organization audit
contracts. It does not own account lifecycle, profile attributes, auth handoff,
tenant isolation policy, billing entitlement, product authorization, or
application-specific organization workflows.

Enterprise identity APIs expose overlapping but inconsistent concepts:
organizations, groups, workspaces, tenants, teams, verified domains, directory
groups, roles, invitations, and collaboration memberships. Macaca needs a
bounded operating-system capability that lets applications request these
operations through typed service commands, with policy, trace, resource,
entitlement, approval, health, snapshot, replay, and unavailable behavior
wrapped around every call.

## Supplier Capability Matrix

| Supplier or protocol | Relevant capability | Macaca interpretation |
| --- | --- | --- |
| Auth0 Organizations | Organizations, enabled connections, members, member roles, invitations, branding metadata | Organization records, connection references, membership, role binding, invite lifecycle; branding and login behavior stay provider/application-side |
| Clerk Organizations | Organization records, memberships, invitations, roles, permissions, metadata, domains | Organization record, membership, role/permission references, invitation, domain identifier; active session context remains auth/session-side |
| WorkOS Organizations, Directory Sync, RBAC | Organizations, domains, directory users/groups, group membership, RBAC roles, invitations | Organization record, verified-domain reference, directory group reference, role binding, invitation; sync engine and IdP adapters remain service-provider internals |
| Okta Groups and Roles | Groups, group membership, group rules, roles, user assignments | Organization/group-backed membership and role binding; group-rule automation remains provider-side evidence |
| Microsoft Graph | Groups, owners, members, transitive membership, directory roles, invitations, external users | Organization/group references, owner/member bindings, invitation references, external-collaboration evidence; Entra tenant policy remains tenant pack |
| Google Admin SDK and Cloud Identity Groups | Groups, aliases, members, nested groups, dynamic groups | Organization or directory-group references, membership, nested/dynamic group metadata; group evaluation stays provider-side |
| SCIM 2.0 Groups | Groups, displayName, members, externalId, metadata, filtering, PATCH | Interoperable group schema, member references, version/freshness, paginated search, patch preconditions |
| GitHub Organizations and Teams | Organization membership, teams, team roles, invitations, audit-style events | Developer-platform reference for membership and role evidence; GitHub-specific collaboration semantics are not OS semantics |

## Goals

- Provide a stable pack id `pack.identity.organization.v1` and command
  namespace `organization.*`.
- Normalize organization records, identifiers, memberships, invitations,
  role bindings, directory-group references, organization policy references,
  and audit export handles.
- Support provider inspection, schema discovery, planning commands, mutating
  commands, search/list commands, and artifact retrieval through typed
  command/result DTOs.
- Preserve a single canonical execution path through SDK/facade clients,
  service runtime decorators, and replaceable organization service providers.
- Return structured `success`, `partial`, `denied`, `approval_required`,
  `unavailable`, `unsupported`, `conflict`, `stale_version`, `quota_exceeded`,
  `rate_limited`, and `failure` results.
- Emit sanitized trace, audit, health, snapshot, and replay evidence for every
  declaration, admission, policy decision, service call, provider decision, and
  unavailable state.
- Require detailed developer documentation at
  `docs/developer-packs/identity/organization.md`.

## Non-Goals

- No account lifecycle ownership. Account records and subject status belong to
  `pack.identity.account.v1`.
- No rich profile or preference ownership. Profile fields belong to
  `pack.identity.profile.v1`.
- No authentication, token exchange, callback handling, or login UI ownership.
  Auth handoff belongs to `pack.identity.auth.handoff.v1`.
- No tenant isolation policy, quota ownership, residency policy, or tenant
  billing boundary. Tenant isolation belongs to `pack.identity.tenant.v1`.
- No payment, subscription, receipt, license, or billing entitlement behavior.
- No application-specific RBAC, feature gating, workflow approval, HRIS
  lifecycle, or product authorization policy.
- No provider-name routing, concrete provider construction, or provider-specific
  branches in kernel, SDK, shells, or the generic application framework.

## Ownership And Boundaries

- Pack id: `pack.identity.organization.v1`.
- Family: `identity`.
- Backing service owner: replaceable organization service provider.
- SDK surface: `sdk.packs.identity.organization`.
- Command namespace: `organization.*`.
- Microkernel ownership: identity handles, service-call evidence, policy
  facade, resource facade, trace/audit primitives, and scheduling primitives
  only.
- Application framework ownership: manifest declaration, app-scoped permission
  declarations, lifecycle projection, and effective capability mementos.
- Runtime-host ownership: provider registration, service runtime decorators,
  transport adapters, health/snapshot bridge, and unavailable/mock provider
  composition through approved composition roots.

## Command Surface

All commands carry trace context, application/session/task/tenant identifiers
when available, policy context, idempotency key for side effects, redaction
profile, resource budget, and replay metadata.

| Command | Purpose | Notes |
| --- | --- | --- |
| `organization.inspect_provider` | Return provider capability metadata | Reports commands, lifecycle states, schema support, role support, invitation support, directory-link support, rate limits, and unavailable reasons |
| `organization.discover_schema` | Return organization/member/invitation/role schema | Exposes field descriptors, version support, filter support, pagination, and redaction policy |
| `organization.plan_create` | Validate a create request without side effects | Checks identifiers, domains, display names, parent references, policy, quota, and provider capability |
| `organization.create` | Create an organization record | Requires idempotency, approval when policy marks high impact, conflict handling, and audit evidence |
| `organization.get` | Read one organization record | Returns minimized organization fields and version/freshness metadata |
| `organization.search` | Search/list organizations | Requires bounded pagination, field masks, tenant/app scoping, and redaction |
| `organization.plan_update` | Validate updates without side effects | Checks version preconditions, immutable fields, domain changes, metadata limits, and policy |
| `organization.update` | Patch organization metadata/settings | Uses version preconditions and idempotency; excludes tenant policy and billing settings |
| `organization.archive` | Archive, disable, or soft-delete an organization | Requires privileged approval and provider lifecycle support |
| `organization.restore` | Restore an archived organization | Requires version/lifecycle validation and privileged approval |
| `organization.list_members` | List memberships for an organization | Supports filters, pagination, role filters, member state, and redacted account/profile references |
| `organization.get_membership` | Inspect one membership | Returns account/profile references, role bindings, source, state, freshness, and audit references |
| `organization.plan_membership_change` | Validate add/remove/update membership | Checks account references, uniqueness, role constraints, directory-managed state, approval, and policy |
| `organization.request_membership_change` | Add, remove, suspend, reactivate, or update membership | Requires idempotency, side-effect audit, and directory-managed conflict handling |
| `organization.create_invitation` | Create an invitation | Requires recipient minimization, expiry, delivery reference, role constraints, approval, and no raw invite token exposure |
| `organization.resend_invitation` | Resend an invitation through provider/application delivery reference | Does not own email/messaging delivery content; records delivery reference only |
| `organization.revoke_invitation` | Revoke a pending invitation | Requires idempotency and conflict handling for accepted/expired invites |
| `organization.inspect_invitation` | Inspect invitation state | Returns pending/accepted/expired/revoked state without raw token data |
| `organization.plan_role_binding` | Validate role assignment/removal | Checks role catalog, privilege class, separation-of-duty policy, directory-managed state, and approval |
| `organization.request_role_binding` | Assign or remove role bindings | Requires privileged approval for elevated roles and audit evidence |
| `organization.list_role_bindings` | List role bindings | Supports member, role, source, and privilege-class filters |
| `organization.inspect_directory_links` | Inspect linked groups/directories | Returns directory-group references, sync freshness, provider class, and conflict/unavailable diagnostics |
| `organization.export_audit` | Request bounded organization audit export | Returns artifact handle with retention/redaction metadata |
| `organization.get_artifact` | Retrieve audit/export artifact handle metadata | Does not expose raw provider payloads or unbounded logs |

## Provider-Neutral DTO Model

- `OrganizationScope`: application id, tenant id, organization id, provider
  reference, directory reference, caller subject, and trace context.
- `OrganizationRecord`: stable organization handle, display name, identifiers,
  verified-domain references, lifecycle state, metadata namespaces, policy
  references, version, freshness, and audit references.
- `OrganizationIdentifier`: provider id, external id, slug, alias, domain,
  display name, SCIM external id, or directory group id with uniqueness scope
  and verification metadata.
- `OrganizationLifecycleState`: planned, active, suspended, archived,
  restoring, deleted, unavailable, provider_unknown.
- `OrganizationMembership`: membership handle, organization handle, account
  reference, profile reference, membership state, role bindings, source,
  invitation reference, directory group reference, version, and freshness.
- `OrganizationMembershipState`: pending, active, suspended, removed,
  directory_managed, expired, conflict, provider_unknown.
- `OrganizationInvitation`: invitation handle, organization handle, recipient
  reference or redacted contact hint, requested role references, expiry,
  delivery reference, state, acceptance reference, revocation reference, and
  audit references.
- `OrganizationRoleReference`: provider-neutral role handle, display label,
  privilege class, source, inherited flag, directory-managed flag, and provider
  capability reference.
- `OrganizationRoleBinding`: binding handle, role reference, subject reference,
  organization reference, source, effective state, inherited state, version,
  freshness, and audit reference.
- `DirectoryGroupReference`: directory provider class, group handle, nested
  group hint, dynamic group hint, SCIM schema version, sync freshness, and
  conflict metadata.
- `OrganizationPolicyReference`: opaque reference to tenant/application policy,
  separation-of-duty checks, invite policy, role policy, or domain policy. The
  organization pack consumes decisions but does not own policy engines.
- `OrganizationAuditReference`: bounded event reference, provider event cursor,
  export artifact handle, redaction profile, and retention metadata.
- `OrganizationArtifactHandle`: artifact id, content class, redaction state,
  retention deadline, size class, checksum/hash, and retrieval permissions.

## Permission, Policy, Resource, Entitlement, And Approval Model

Initial permission scopes:

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

Policy checks run before side effects and before provider calls that could
reveal sensitive data. Policy inputs include caller subject, application id,
tenant id, organization scope, requested command, requested fields, privilege
class, directory-managed state, invitation recipient class, resource budget,
approval state, and entitlement state.

Approval is required for high-impact operations such as creating external
invitations, assigning elevated roles, removing the final owner/admin,
archiving or restoring organizations, exporting audit data, modifying
directory-managed memberships, or changing verified-domain identifiers.

Resource checks cover organization count, member count, role-binding count,
invitation count, audit export size, pagination window, provider quota, network
budget, timeout, retained artifacts, retained snapshots, and event volume.

Entitlement checks determine whether the calling application/tenant may use the
pack, requested commands, directory-link features, audit export features, and
privileged role operations. Missing entitlement returns structured
`unavailable` or `denied` diagnostics rather than provider fallback.

## Service Runtime And Provider Strategy

The organization service provider is a Strategy behind the service runtime. The
runtime composes provider adapters, unavailable providers, mock providers,
policy decorators, resource decorators, entitlement decorators, metering,
redaction, trace, audit, and health/snapshot behavior.

Provider adapters may target Auth0, Clerk, WorkOS, Okta, Microsoft Graph,
Google/Cloud Identity, SCIM-compatible directories, GitHub-style developer
organizations, built-in local providers, remote providers, plugin providers, or
mock providers. Provider-specific capabilities are descriptor data, not OS
routing branches.

The unavailable provider is first-class. It exposes descriptor metadata, health
state, unsupported command diagnostics, and stable error DTOs without crashing,
hanging, silently falling back, or faking success.

## State, Consistency, And Idempotency

Organization records, memberships, invitations, role bindings, directory links,
and audit exports have explicit lifecycle states. Mutating commands require
idempotency keys and version preconditions when provider support exists. When a
provider has eventual consistency, the result must include freshness,
provider_state, replay cursor, and partial/async status rather than pretending
the state is immediately final.

Directory-managed records are protected. If a membership, role binding, or
group link is controlled by an external directory-sync source, direct mutation
returns `conflict`, `unsupported`, or `approval_required` according to policy and
provider capability.

## SDK Discovery And Developer Documentation

SDK discovery must return pack metadata, command schemas, permission scopes,
field masks, filter support, pagination support, role/permission support,
invitation support, directory-link support, audit-export support, examples,
availability, diagnostics, provider class, compatibility hash, redaction
profile, and documentation link.

SDK helper builders only build canonical traced service calls. They must never
construct identity providers, hold credentials, call provider APIs directly,
evaluate product RBAC, mutate account/profile/tenant state, or deliver
invitation messages outside declared communication packs.

Developer documentation at `docs/developer-packs/identity/organization.md` must
cover:

- Capability purpose and non-goals.
- Manifest declaration examples for required and optional usage.
- Permission scopes and approval behavior.
- Command DTOs and result DTOs with field-level explanations.
- Organization, membership, invitation, role-binding, directory-link, audit,
  artifact, version, and freshness models.
- Supplier/API mapping and provider replacement guidance.
- Unavailable/denied/conflict/stale-version diagnostics.
- Trace/audit events, redaction rules, snapshot/replay behavior, and
  conformance checklist for provider authors.

## Trace, Audit, Health, Snapshot, And Replay

Required event families:

- `organization_pack_declared`
- `organization_pack_admission_validated`
- `organization_pack_discovery_requested`
- `organization_pack_policy_decision`
- `organization_pack_resource_reserved`
- `organization_pack_approval_required`
- `organization_pack_service_call_requested`
- `organization_pack_service_call_succeeded`
- `organization_pack_service_call_failed`
- `organization_pack_unavailable`
- `organization_pack_conflict_detected`
- `organization_pack_snapshot_recorded`
- `organization_pack_audit_export_requested`

Events include pack id, descriptor version, command name, trace id,
application/session/task/tenant identifiers when available, organization handle
hash, subject handle hash, policy decision, approval state, provider class,
latency, bounded resource counters, capability hash, and bounded error code.

Events, snapshots, SDK diagnostics, and examples must exclude raw credentials,
invite tokens, access tokens, refresh tokens, directory sync secrets, raw
provider payloads, full member lists beyond requested pages, private profile
fields, raw audit exports, manifests, package bytes, private keys, signatures,
and unbounded output.

Snapshots include descriptor version, provider capability hash, command
availability, provider health, role schema hash, policy template hash, resource
counters, bounded organization/member/invitation/role summary counts, artifact
summaries, event cursors, and sanitized replay pointers.

## Design Patterns

- **Facade**: `SystemFacade` and focused SDK clients expose discovery and typed
  command builders while hiding service runtime and provider composition.
- **Command**: every operation is represented as a typed command/result DTO
  with explicit success, partial, denied, unavailable, unsupported, conflict,
  stale-version, quota, approval-required, and failure variants.
- **Adapter/Bridge**: Auth0, Clerk, WorkOS, Okta, Microsoft Graph, Google,
  SCIM, GitHub-style, built-in, plugin, remote, mock, and unavailable providers
  adapt into the same provider-neutral contract.
- **Strategy**: provider selection, schema compatibility, role mapping,
  invitation capability, directory-link behavior, audit-export behavior, and
  unavailable behavior are replaceable.
- **Decorator**: trace, audit, policy, resource, entitlement, approval,
  metering, timeout, cancellation, and redaction wrap every service call.
- **State**: organization, membership, invitation, role binding, audit export,
  and provider lifecycle states are explicit and replayable.
- **Observer**: trace, audit, health, and service events are subscribable by
  shells without giving shells semantic ownership.
- **Memento**: effective capability reports, snapshots, provider capability
  hashes, and audit cursors preserve bounded recovery state.
- **Specification**: admission validates pack id, command availability,
  permission scopes, provider health, entitlement, resource budgets, and policy
  templates.
- **Abstract Factory**: concrete provider adapters are constructed only in
  approved composition roots.

## Risks And Mitigations

- Risk: organization becomes a hidden tenant policy engine. Mitigation:
  tenant isolation, residency, quotas, and tenant billing remain referenced
  decisions owned by `pack.identity.tenant.v1`.
- Risk: organization roles become product authorization logic. Mitigation:
  role bindings are provider-neutral identity evidence; application feature
  gating remains application or entitlement policy.
- Risk: directory sync semantics leak into OS code. Mitigation: directory sync
  is provider-side; Macaca stores only references, freshness, and conflict
  diagnostics.
- Risk: SDK helpers become a second execution path. Mitigation: helpers only
  build canonical service commands and are covered by no-direct-provider-call
  gates.
- Risk: invite tokens or member/profile data leak through observability.
  Mitigation: event schemas allow only handles, hashes, counters, bounded codes,
  redacted contact hints, and artifact references.
- Risk: provider-specific role names become OS semantics. Mitigation:
  provider roles map to `OrganizationRoleReference` and privilege classes; raw
  role names remain provider data or display metadata.
