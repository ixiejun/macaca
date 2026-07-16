# Change: Add Identity Account Pack

## Why

Macaca applications need `pack.identity.account.v1` as an industrial account
management capability for account lookup, creation, lifecycle state, linked
identities, status synchronization, recovery references, and audit export.
Identity providers expose these operations with different user objects,
lifecycle transitions, directory schemas, federation links, and risk controls.
Macaca must normalize the account boundary without owning OAuth/OIDC handoff,
profile preferences, organization membership, tenant isolation policy, or raw
credential management.

This proposal defines account management as a serviceized, provider-neutral
pack. It gives applications typed account commands while keeping concrete IdP
adapters, lifecycle semantics, directory schemas, and unavailable behavior behind
replaceable service providers.

## Supplier And API Baseline

The design is based on mature identity management APIs:

- Okta Users API exposes user objects, profile/provider data, credentials,
  lifecycle operations such as activate, deactivate, suspend, unsuspend, unlock,
  expire password, and linked identity provider metadata.
- Auth0 Management API exposes users, identities, connection membership, block
  status, email verification state, metadata, account linking, logs, and
  deletion behavior.
- Microsoft Graph Users API exposes Entra ID user resources, create/update/list,
  account enabled state, identities, mail/user principal names, license-related
  references, manager links, and directory audit integration.
- Google Admin SDK Directory API exposes users, aliases, suspension, recovery
  data, organizations, custom schemas, tokens, and undelete behavior.
- SCIM 2.0 defines interoperable Users resources with external IDs, active
  status, groups, emails, names, metadata, filtering, pagination, PATCH, and
  schema extension mechanics.
- WorkOS, Clerk, and similar developer identity platforms expose user records,
  external identities, invitations, sessions, organization references, and audit
  events that need provider-neutral mapping.

Research references:

- Okta Users API: https://developer.okta.com/docs/api/openapi/okta-management/management/tag/User/
- Auth0 Management Users API: https://auth0.com/docs/api/management/v2/users
- Microsoft Graph users: https://learn.microsoft.com/graph/api/resources/user
- Google Admin SDK Directory Users:
  https://developers.google.com/admin-sdk/directory/reference/rest/v1/users
- SCIM Users schema: https://www.rfc-editor.org/rfc/rfc7643 and
  https://www.rfc-editor.org/rfc/rfc7644
- WorkOS User Management: https://workos.com/docs/user-management
- Clerk Backend Users API: https://clerk.com/docs/reference/backend-api/tag/Users

## Macaca Provider-Neutral Mapping

`pack.identity.account.v1` maps supplier concepts into stable Macaca contracts:

- Provider users, directory users, SCIM users, account records, and external
  identity users become `AccountRecord`.
- Provider subject IDs, user principal names, usernames, emails, aliases,
  phone-number identifiers, and external IDs become `AccountIdentifier`.
- Active, staged, provisioned, locked, suspended, disabled, deprovisioned,
  deleted, archived, password-expired, and unknown provider states become
  `AccountLifecycleState`.
- Provider identities, social/enterprise connections, federation links, SCIM
  external IDs, and IdP references become `LinkedIdentityReference`.
- Provider profile fields are represented only as minimized account attributes;
  rich profile and preferences belong to `pack.identity.profile.v1`.
- Organization, group, role, and tenant relationships become references only;
  membership and isolation policy belong to organization and tenant packs.
- Provider logs and directory audit events become `AccountAuditReference` and
  audit export artifact handles.

## What Changes

- Add provider-neutral `pack.identity.account.v1` under the identity family.
- Define commands for provider inspection, schema discovery, account planning,
  account creation, read/search, update, lifecycle transition planning,
  lifecycle transition request, linked identity management, status sync, recovery
  reference management, audit export, and artifact retrieval.
- Define DTOs for account scope, provider capability, account record,
  identifiers, minimized attributes, lifecycle state, linked identity
  references, recovery references, audit references, freshness, attribution,
  redaction, and artifact handles.
- Require policy, consent/approval for sensitive lifecycle changes, tenant
  isolation, account identifier uniqueness, idempotency for mutating commands,
  raw credential rejection, sanitized trace/audit, and deterministic
  unavailable/unsupported behavior.
- Require detailed developer documentation at
  `docs/developer-packs/identity/account.md`.

## Impact

- Affected specs: `pack-identity-account`,
  `developer-pack-industrial-capability-catalog`, `sdk-system-facade`,
  `service-runtime`, `unified-execution-path`.
- Affected code later: protocol DTOs, pack descriptors, admission validators,
  SDK discovery/command builders, account service providers,
  mock/unavailable providers, trace/audit schemas, replay tests, redaction
  tests, and boundary gates.

## Non-Goals

- No OAuth/OIDC/SAML auth handoff, token exchange, session binding, raw password
  or credential storage, MFA challenge execution, profile preference management,
  organization membership management, tenant isolation policy ownership, or
  application-specific account workflow.
- No provider-specific user lifecycle policy, HRIS business workflow,
  provisioning business rule, or directory routing in Macaca OS layers.
- No raw credentials, password hashes, recovery codes, MFA secrets, access
  tokens, refresh tokens, raw provider payloads, private keys, signatures,
  identity documents, or unbounded audit exports in logs, traces, snapshots, or
  SDK diagnostics.
- No provider construction or provider-name routing in kernel, SDK, shells, or
  generic application framework.
