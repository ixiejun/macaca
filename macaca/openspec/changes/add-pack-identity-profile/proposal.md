# Change: Add Identity Profile Pack

## Why

Macaca applications need `pack.identity.profile.v1` as an industrial profile
capability for reading, updating, minimizing, exporting, and auditing user or
agent profile attributes. Mature identity providers expose profile data through
OpenID Connect standard claims, directory profile schemas, people/contact APIs,
avatar/photo APIs, locale/timezone fields, custom attributes, privacy controls,
and user metadata. Macaca must normalize those surfaces without turning profile
management into account lifecycle, auth handoff, organization membership, tenant
policy, application-specific preferences, or raw identity document storage.

This proposal defines profile management as a serviceized, provider-neutral
pack. It gives applications typed profile commands while keeping concrete IdP,
directory, people API, and avatar providers behind replaceable service
providers.

## Supplier And API Baseline

The design is based on mature profile and identity-data APIs:

- OpenID Connect Core defines standard profile claims such as name, given name,
  family name, preferred username, profile URL, picture, website, gender,
  birthdate, zoneinfo, locale, updated_at, email, email_verified, phone number,
  phone_number_verified, and address.
- Microsoft Graph exposes user profile fields, photos, mailbox/settings-related
  user data, manager references, extensions, and directory-backed profile
  metadata.
- Google People API exposes names, email addresses, phone numbers, photos,
  birthdays, biographies, locales, organizations, relations, user-defined data,
  and field masks.
- Okta Universal Directory and user profile schemas expose base profile
  attributes, custom attributes, profile mappings, schema validation, and
  profile enrollment constraints.
- Auth0 user profiles expose normalized profile data, user metadata,
  app metadata, identities, picture, email verification state, and custom
  profile fields.
- SCIM 2.0 User schema exposes userName, name, displayName, emails, phone
  numbers, photos, addresses, locale, timezone, preferredLanguage, and extension
  schemas.
- Clerk, WorkOS, and similar identity platforms expose user profile fields,
  public metadata, private metadata, unsafe metadata, avatars, and external
  identity-derived profile attributes.

Research references:

- OpenID Connect Standard Claims:
  https://openid.net/specs/openid-connect-core-1_0.html#StandardClaims
- Microsoft Graph user and profile photo:
  https://learn.microsoft.com/graph/api/resources/user and
  https://learn.microsoft.com/graph/api/resources/profilephoto
- Google People API:
  https://developers.google.com/people/api/rest/v1/people
- Okta user profile schema:
  https://developer.okta.com/docs/reference/api/schemas/
- Auth0 normalized user profiles:
  https://auth0.com/docs/manage-users/user-accounts/user-profiles
- SCIM User schema: https://www.rfc-editor.org/rfc/rfc7643
- Clerk user metadata:
  https://clerk.com/docs/users/user-metadata
- WorkOS user management:
  https://workos.com/docs/user-management

## Macaca Provider-Neutral Mapping

`pack.identity.profile.v1` maps supplier concepts into stable Macaca contracts:

- OIDC claims, directory user profile fields, People API fields, and user
  metadata become `ProfileRecord` and `ProfileField`.
- Provider profile schemas, SCIM extension schemas, custom attributes, and
  metadata namespaces become `ProfileSchemaDescriptor`.
- Provider avatar/photo URLs or binary resources become `AvatarReference` and
  `ProfileArtifactHandle`; raw image bytes are handled only through bounded
  artifact paths.
- Locale, timezone, preferred language, display name, contact hints, pronouns,
  website, bio, and visibility fields become normalized profile fields with
  redaction and privacy classification.
- Provider user/app/private metadata become scoped `ProfileMetadataNamespace`
  values with policy and visibility controls.
- Provider field masks, `updated_at`, ETags, and version tokens become
  `ProfileFreshness` and `ProfileVersion`.
- Account identifiers, lifecycle states, auth identities, organization
  membership, tenant policy, sessions, and secrets become references only.

## What Changes

- Add provider-neutral `pack.identity.profile.v1` under the identity family.
- Define commands for provider inspection, schema discovery, profile read/search,
  patch planning, profile update, privacy-field inspection, preference namespace
  read/write, avatar reference management, export planning, export request, and
  artifact retrieval.
- Define DTOs for profile scope, provider capability, profile records, fields,
  schema descriptors, metadata namespaces, privacy classes, preferences, avatar
  references, freshness/version metadata, audit references, redaction, and
  artifact handles.
- Require policy, consent/approval for sensitive profile changes and retained
  exports, field minimization, visibility classification, version checks,
  idempotency for mutating commands, sanitized trace/audit, and deterministic
  unavailable/unsupported behavior.
- Require detailed developer documentation at
  `docs/developer-packs/identity/profile.md`.

## Impact

- Affected specs: `pack-identity-profile`,
  `developer-pack-industrial-capability-catalog`, `sdk-system-facade`,
  `service-runtime`, `unified-execution-path`.
- Affected code later: protocol DTOs, pack descriptors, admission validators,
  SDK discovery/command builders, profile service providers,
  mock/unavailable providers, trace/audit schemas, replay tests, redaction
  tests, artifact tests, and boundary gates.

## Non-Goals

- No account creation/lifecycle, OAuth/OIDC/SAML handoff, token exchange,
  session binding, password or credential handling, organization membership,
  tenant isolation policy, identity-document verification, or application-owned
  business preference logic.
- No provider-specific profile mapping policy, HRIS workflow, marketing
  preference workflow, or application-specific onboarding/offboarding behavior
  in Macaca OS layers.
- No raw credentials, tokens, identity documents, raw provider payloads, private
  keys, signatures, unbounded profile exports, or unbounded avatar/photo bytes in
  logs, traces, snapshots, or SDK diagnostics.
- No provider construction or provider-name routing in kernel, SDK, shells, or
  generic application framework.
