# Identity Account Pack Research

## Purpose

This note records supplier/API research, Macaca provider-neutral mapping,
explicit non-goals, existing platform inventory, and GitNexus memo evidence for
`pack.identity.account.v1`. The account pack owns account records, stable
identifiers, lifecycle state, linked identity references, status sync, recovery
references, account audit export, artifact handles, freshness, attribution, and
redaction through typed service commands. It must not own auth handoff, token
exchange, session binding, raw credential storage, MFA challenge execution,
profile preference management, organization membership, tenant isolation, or
application-specific account workflows.

## Source Baseline

- Okta Users and lifecycle APIs:
  <https://developer.okta.com/docs/api/openapi/okta-management/management/tag/User/>
- Auth0 Management API users:
  <https://auth0.com/docs/api/management/v2/users>
- Microsoft Graph user resource and permissions:
  <https://learn.microsoft.com/en-us/graph/api/resources/user?view=graph-rest-1.0>
  and <https://learn.microsoft.com/en-us/graph/permissions-reference>
- Google Admin SDK Directory users:
  <https://developers.google.com/workspace/admin/directory/reference/rest/v1/users>
- SCIM 2.0 protocol and core schema:
  <https://datatracker.ietf.org/doc/html/rfc7644> and
  <https://datatracker.ietf.org/doc/html/rfc7643>
- WorkOS users and organizations:
  <https://workos.com/docs/authkit/users-organizations>
- Clerk users:
  <https://clerk.com/docs/reference/backend-api/tag/Users>

## Supplier API Notes

- Okta contributes account lifecycle transitions, status mapping, profile
  schema, enrolled factors, recovery-related operations, and user search. Macaca
  should normalize lifecycle intent and status evidence without adopting Okta
  status strings as stable OS semantics.
- Auth0 contributes user records, identities, metadata namespaces, blocking,
  MFA enrollment references, and log/audit surfaces. Macaca should treat
  metadata and linked identities as bounded references and keep auth execution
  in the auth handoff pack.
- Microsoft Graph contributes user CRUD, delta, relationships, photos,
  directory roles, and permission classes. Macaca should model permission and
  directory scope gates explicitly before any provider call.
- Google Admin Directory contributes user records, aliases, org unit
  placement, suspended state, and custom schemas. Macaca should expose only
  account-owned fields and keep organization and tenant semantics outside this
  pack.
- SCIM contributes provider-neutral User schema, external IDs, PATCH, filter,
  pagination, groups, and service-provider configuration. Macaca should use SCIM
  as an interoperability baseline, not as a provider-native payload passthrough.
- WorkOS and Clerk contribute user identities, email verification,
  organization references, metadata, and account management surfaces. Macaca
  should preserve provider attribution and bounded capability discovery.

## Macaca-Owned Abstractions

`pack.identity.account.v1` should define `AccountScope`,
`AccountProviderCapability`, `AccountRecord`, `AccountIdentifier`,
`AccountLifecycleState`, `LinkedIdentityReference`,
`AccountRecoveryReference`, `AccountAuditReference`,
`AccountAuditExportPlan`, `AccountArtifactHandle`, `AccountFreshness`,
`AccountAttribution`, and `AccountRedactionPolicy`.

The DTOs must carry stable subject references, tenant and provider scope,
identifier class, verification state, minimized account attributes, lifecycle
state, linked identity provenance, recovery-reference class, version token,
freshness timestamp, provider attribution, bounded reason codes, artifact
checksums, redaction class, and replay pointers. Raw passwords, password
hashes, reset tokens, recovery codes, MFA secrets, raw access tokens, raw
refresh tokens, raw provider payloads, and unbounded audit exports are rejected.

## Explicit Non-Goals

- Do not implement concrete Okta, Auth0, Microsoft Graph, Google, SCIM, WorkOS,
  Clerk, directory, credential, MFA, or session adapters in this research phase.
- Do not perform OAuth/OIDC/SAML handoff, token exchange, session binding,
  password or credential handling, MFA challenge execution, profile preference
  management, organization membership changes, tenant policy changes, or
  application-specific account workflows.
- Do not expose provider-native user payloads, raw credentials, raw tokens,
  provider lifecycle strings, or business-specific account rules as stable SDK
  contracts.

## Existing Macaca Platform Inventory

- `crates/foundation/macaca-proto/src/domain_pack_contract/` provides reusable
  descriptor metadata, lifecycle/availability, policy templates, diagnostics,
  SDK metadata, provider snapshots, unavailable diagnostics, and effective
  capability expansion.
- `crates/facade/macaca-sdk/src/system_facade.rs` provides the Facade pattern;
  account SDK helpers should only construct canonical traced service calls.
- `crates/runtime/macaca-host-composition/src/runtime_host.rs` and
  `crates/kernel/macaca-kernel/src/domain_pack_registration.rs` provide generic
  provider registration/composition mechanics for optional domain-pack
  providers.
- Policy, approval, resource, entitlement, trace, audit, artifact, mock-provider,
  and unavailable-provider concepts exist generically, but current evidence does
  not prove account-specific DTOs, descriptors, providers, SDK helpers, ABI
  metadata, tests, or docs.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
