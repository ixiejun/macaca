## 1. Research, Governance, And Scope

- [x] 1.1 Re-read architecture governance, microkernel boundaries, serviceization allowlist, design-pattern guidance, and the industrial catalog umbrella proposal before implementation.
- [x] 1.2 Record supplier/API findings for Auth0 Organizations, Clerk Organizations, WorkOS Organizations/Directory Sync/RBAC, Okta Groups/Roles, Microsoft Graph groups/directory roles/invitations, Google Admin/Cloud Identity Groups, SCIM Groups, and GitHub Organizations/Teams.
- [x] 1.3 Confirm boundary decisions with adjacent packs: account owns account lifecycle, profile owns profile fields, auth handoff owns login/token exchange, tenant owns tenant isolation/quota, entitlement owns licensing, workflow owns approvals/reviews, and communication owns message delivery.
- [x] 1.4 Inventory existing descriptors, SDK clients, identity services, service-runtime decorators, artifact services, mock providers, and unavailable providers that can back organization service implementation.
- [x] 1.5 Record GitNexus CRITICAL/HIGH findings as memo only before implementation commits, without letting advisory severity block this proposal track.

## 2. Contract, Descriptor, And Schema

- [x] 2.1 Define `pack.identity.organization.v1` descriptor metadata for pack id, family, lifecycle, stability, command schemas, permissions, policy template, resource budget, approval rules, data governance, SDK metadata, docs link, compatibility, health, and diagnostics.
- [x] 2.2 Define provider-neutral DTOs for `OrganizationScope`, `OrganizationRecord`, `OrganizationIdentifier`, `OrganizationLifecycleState`, `OrganizationMembership`, `OrganizationMembershipState`, `OrganizationInvitation`, `OrganizationRoleReference`, `OrganizationRoleBinding`, `DirectoryGroupReference`, `OrganizationPolicyReference`, `OrganizationAuditReference`, and `OrganizationArtifactHandle`.
- [x] 2.3 Define command DTOs for `organization.inspect_provider`, `organization.discover_schema`, `organization.plan_create`, `organization.create`, `organization.get`, `organization.search`, `organization.plan_update`, `organization.update`, `organization.archive`, `organization.restore`, `organization.list_members`, `organization.get_membership`, `organization.plan_membership_change`, `organization.request_membership_change`, `organization.create_invitation`, `organization.resend_invitation`, `organization.revoke_invitation`, `organization.inspect_invitation`, `organization.plan_role_binding`, `organization.request_role_binding`, `organization.list_role_bindings`, `organization.inspect_directory_links`, `organization.export_audit`, and `organization.get_artifact`.
- [x] 2.4 Define typed success, partial, approval-required, denied, unavailable, unsupported, conflict, stale-version, quota, rate-limited, timeout, cancelled, and failure result DTOs.
- [x] 2.5 Add descriptor hashing, schema-version compatibility, command-availability hashing, role-schema hashing, policy-template hashing, and redaction-profile hashing.
- [x] 2.6 Add unit tests for valid descriptors, rejected descriptors, missing command schemas, invalid permission scopes, unstable hashes, incompatible versions, and redaction metadata.

## 3. Permission, Policy, Resource, Entitlement, And Approval

- [x] 3.1 Implement declaration validation for scopes: `identity.organization.read`, `identity.organization.search`, `identity.organization.write`, `identity.organization.archive`, `identity.organization.membership.read`, `identity.organization.membership.write`, `identity.organization.invitation.read`, `identity.organization.invitation.write`, `identity.organization.role.read`, `identity.organization.role.write`, `identity.organization.directory.read`, `identity.organization.audit.export`, and `identity.organization.artifact.read`.
- [x] 3.2 Implement policy checks for caller subject, application id, tenant id, organization scope, requested command, requested fields, privilege class, directory-managed state, invitation recipient class, approval state, resource budget, and entitlement state before provider calls.
- [x] 3.3 Implement resource reservation for organization count, member count, role-binding count, invitation count, audit export size, pagination window, provider quota, network budget, timeout, retained artifacts, retained snapshots, and event volume.
- [x] 3.4 Implement entitlement checks and structured unavailable/denied diagnostics for missing provider, disabled pack, missing permission, missing entitlement, missing role feature, missing invitation feature, missing directory-link feature, missing audit-export feature, and disabled host capability.
- [x] 3.5 Implement approval behavior for external invitations, elevated roles, final-owner/admin removal, organization archive/restore, audit export, directory-managed membership mutation, domain identifier changes, and high-volume member exports.
- [x] 3.6 Add tests proving denied, unavailable, unsupported, quota, approval-required, conflict, stale-version, and missing-entitlement paths do not call concrete providers or emit side effects.

## 4. Service Runtime Provider Implementation

- [x] 4.1 Implement or bind organization service provider behind the service runtime; do not construct providers from SDK, shell, kernel, or application code.
- [x] 4.2 Add a deterministic unavailable provider that returns complete descriptor metadata, health state, command availability, and typed unavailable/unsupported diagnostics.
- [x] 4.3 Add mock provider support for provider inspection, schema discovery, organization lifecycle, membership lifecycle, invitation lifecycle, role binding, directory-link inspection, audit export, and artifact handle metadata.
- [x] 4.4 Add lifecycle, health, snapshot, shutdown, timeout, cancellation, pagination, async audit export, idempotency, version precondition, stale-version diagnostics, conflict diagnostics, and rate-limit diagnostics.
- [x] 4.5 Add Strategy implementations for provider adapters, schema mapping, role mapping, invitation behavior, directory-link behavior, audit-export behavior, artifact behavior, redaction, and unavailable behavior.
- [x] 4.6 Add explicit state machines for organizations, memberships, invitations, role bindings, audit exports, and provider lifecycle states.
- [x] 4.7 Add side-effect safety support for idempotency keys, provider state validation, directory-managed conflict detection, final-owner/admin protection, privilege escalation detection, and non-mutating plan commands.
- [x] 4.8 Add provider capability reporting for available, degraded, preview, unavailable, unsupported, retired, directory-limited, invitation-limited, role-limited, audit-limited, quota-limited, and rate-limited states.

## 5. SDK, Admission, ABI, And Examples

- [x] 5.1 Extend SDK discovery for `pack.identity.organization.v1` with command schemas, permission scopes, field masks, filter support, pagination support, role support, invitation support, directory-link support, audit-export support, examples, availability, diagnostics, documentation link, provider class, compatibility hash, and redaction profile.
- [x] 5.2 Extend application admission so required declarations block readiness when unavailable and optional declarations degrade explicitly with effective capability mementos.
- [x] 5.3 Add SDK command helper builders for all `organization.*` commands; helpers must only build canonical traced service calls and must never construct identity providers, hold credentials, call provider APIs directly, evaluate product RBAC, mutate account/profile/tenant state, or deliver messages.
- [x] 5.4 Extend WASM/app ABI descriptors so applications can discover organization commands, declare permissions, receive unavailable diagnostics, and submit typed service calls through the canonical execution path.
- [x] 5.5 Add generic app-facing examples for organization creation, organization read/search, membership list/change, invitation create/revoke, role binding, directory-link inspection, audit export, and unavailable diagnostics.
- [x] 5.6 Add provider-unavailable, missing-permission, missing-entitlement, directory-managed-conflict, role-unsupported, invitation-unsupported, stale-version, approval-required, quota-exceeded, audit-export-denied, and artifact-denied examples that avoid provider names, credentials, private profile data, raw invite tokens, raw provider payloads, and application business workflows.

## 6. Trace, Audit, Replay, And Boundary Gates

- [x] 6.1 Emit sanitized declaration, admission, discovery, policy, resource, entitlement, approval, service-call, organization lifecycle, membership lifecycle, invitation lifecycle, role-binding, directory-link, audit-export, artifact, health, snapshot, unavailable, conflict, and failure events.
- [x] 6.2 Ensure traces, audits, snapshots, SDK diagnostics, and examples exclude raw credentials, invite tokens, access tokens, refresh tokens, directory sync secrets, raw provider payloads, full member lists beyond requested pages, private profile fields, raw audit exports, manifests, package bytes, private keys, signatures, and unbounded output.
- [x] 6.3 Add replay tests proving every `organization.*` command is trace-addressable through the canonical service path and snapshots contain enough bounded metadata for recovery diagnostics.
- [x] 6.4 Add dependency gates proving kernel, SDK, shells, and generic application framework do not import concrete Auth0, Clerk, WorkOS, Okta, Microsoft Graph, Google, SCIM, GitHub, directory-sync, invitation-delivery, credential, or organization provider adapters.
- [x] 6.5 Add no-direct-provider-call gates proving SDK helpers, WASM ABI handlers, app admission, web, CLI, and frontend paths route through descriptor-owned service commands.
- [x] 6.6 Add boundary tests proving optional provider absence returns structured unavailable diagnostics and never crashes, hangs, silently falls back, creates organizations, mutates memberships, sends invitations, assigns roles, exports audits, contacts providers, or fakes success.
- [x] 6.7 Run `openspec validate add-pack-identity-organization --strict`, targeted cargo tests, boundary gates, file-size gates, and audit replay checks before marking implementation complete.

## 7. Developer Documentation

- [x] 7.1 Create `docs/developer-packs/identity/organization.md` with purpose, capability model, manifest declaration, required versus optional behavior, permissions, organization records, identifiers, domains, memberships, invitations, role bindings, directory links, audit exports, artifacts, unavailable diagnostics, provider replacement, and operational limits.
- [x] 7.2 Document all command DTOs and result DTOs with field-level explanations, idempotency semantics, pagination behavior, version preconditions, freshness metadata, redaction behavior, approval behavior, artifact retention behavior, and structured error codes.
- [x] 7.3 Document supplier/API mapping: Auth0 Organizations, Clerk Organizations, WorkOS Organizations/Directory Sync/RBAC, Okta Groups/Roles, Microsoft Graph groups/directory roles/invitations, Google Admin/Cloud Identity Groups, SCIM Groups, and GitHub Organizations/Teams mapped to Macaca abstractions, including what is intentionally not exposed as OS semantics.
- [x] 7.4 Add generic examples for required declaration, optional declaration, organization creation, membership change, invitation flow, role binding, directory-link inspection, audit export, artifact inspection, unavailable provider, denied permission, conflict, and stale-version handling.
- [x] 7.5 Add conformance checklist and test guidance for provider authors: descriptor completeness, organization/member/invitation/role scope validation, idempotency, version handling, directory-managed conflicts, role privilege class mapping, audit redaction, resource bounds, policy hooks, trace/audit events, unavailable behavior, snapshot/replay, and no raw payload leakage.
- [x] 7.6 Cross-link the guide from SDK discovery metadata and the industrial pack catalog index before marking `add-pack-identity-organization` complete.
