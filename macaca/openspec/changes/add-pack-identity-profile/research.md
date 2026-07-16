# Identity Profile Pack Research

## Purpose

This note records supplier/API research, Macaca provider-neutral mapping,
explicit non-goals, existing platform inventory, and GitNexus memo evidence for
`pack.identity.profile.v1`. The profile pack owns profile records, profile
fields, schema descriptors, privacy classifications, profile-owned preferences,
avatar references, profile sync, export artifacts, freshness, attribution, and
redaction. It must not own account lifecycle, auth handoff, token exchange,
session binding, raw credential handling, MFA execution, organization
membership, tenant policy, identity document verification, marketing workflows,
or application business preferences.

## Source Baseline

- OpenID Connect Core standard claims and UserInfo:
  <https://openid.net/specs/openid-connect-core-1_0.html>
- Microsoft Graph user and profile photo resources:
  <https://learn.microsoft.com/en-us/graph/api/resources/user?view=graph-rest-1.0>
  and <https://learn.microsoft.com/en-us/graph/api/resources/profilephoto?view=graph-rest-1.0>
- Google People API:
  <https://developers.google.com/people/api/rest>
- Okta Universal Directory and user profile concepts:
  <https://developer.okta.com/docs/concepts/user-profiles/>
- Auth0 user profile and metadata concepts:
  <https://auth0.com/docs/manage-users/user-accounts/user-profiles>
- SCIM 2.0 User schema:
  <https://datatracker.ietf.org/doc/html/rfc7643>
- Clerk user metadata:
  <https://clerk.com/docs/users/user-metadata>
- WorkOS users:
  <https://workos.com/docs/authkit/users-organizations>

## Supplier API Notes

- OIDC contributes standard claims such as subject, name, email, locale, and
  picture through claims and UserInfo. Macaca should map these to profile fields
  with source, verification, privacy class, and freshness metadata.
- Microsoft Graph contributes user properties, photos, manager and relationship
  surfaces, and permission-sensitive profile data. Macaca should require field
  masks and permission gates before provider access.
- Google People API contributes person names, email addresses, photos,
  birthdays, locales, biographies, and field masks. Macaca should model profile
  minimization and avoid broad profile export by default.
- Okta Universal Directory contributes extensible profile schema and mappings.
  Macaca should use schema descriptors and compatibility hashes instead of
  leaking provider schema objects.
- Auth0, Clerk, and WorkOS contribute profile metadata and identity-linked user
  metadata. Macaca should distinguish profile-owned preferences from app
  business preferences and provider/private metadata namespaces.
- SCIM contributes interoperable user attributes, enterprise extension fields,
  schemas, mutability, returned behavior, uniqueness, and PATCH semantics.

## Macaca-Owned Abstractions

`pack.identity.profile.v1` should define `ProfileScope`,
`ProfileProviderCapability`, `ProfileRecord`, `ProfileField`,
`ProfileSchemaDescriptor`, `ProfileMetadataNamespace`, `ProfilePreference`,
`AvatarReference`, `ProfileAuditReference`, `ProfileExportPlan`,
`ProfileArtifactHandle`, `ProfileFreshness`, `ProfileAttribution`, and
`ProfileRedactionPolicy`.

The DTOs must carry subject/account references, field masks, value type,
metadata namespace, privacy class, verification state, source provenance,
retention class, mutability, localization, avatar artifact handle, version
token, freshness, attribution, bounded reason codes, export checksum, redaction
class, and replay pointers. Raw credentials, tokens, identity documents, raw
avatar bytes, raw provider payloads, unbounded profile exports, private keys,
and signatures are rejected.

## Explicit Non-Goals

- Do not implement concrete OIDC, Microsoft Graph, Google People, Okta, Auth0,
  SCIM, Clerk, WorkOS, media-processing, or credential adapters in this research
  phase.
- Do not perform account creation/lifecycle, auth handoff, token exchange,
  session binding, password or credential handling, MFA execution, organization
  membership, tenant policy, identity document verification, marketing workflow,
  or application business preference management.
- Do not expose provider-native profile payloads, raw photos, broad directory
  dumps, provider metadata internals, or application-specific preference models
  as stable SDK contracts.

## Existing Macaca Platform Inventory

- `crates/foundation/macaca-proto/src/domain_pack_contract/` provides descriptor,
  lifecycle, policy, diagnostics, SDK metadata, provider snapshot, unavailable,
  and effective capability primitives reusable by this pack.
- `crates/facade/macaca-sdk/src/system_facade.rs` provides the Facade pattern;
  profile SDK helpers should only build canonical traced service calls.
- Generic policy, approval, resource, entitlement, trace, audit, artifact,
  mock-provider, and unavailable-provider concepts are reusable, but current
  evidence does not prove profile-specific DTOs, descriptors, providers, SDK
  helpers, ABI metadata, tests, or developer docs.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
