# Identity Profile Pack Design

## Context

`pack.identity.profile.v1` is Macaca's provider-neutral profile data capability.
It owns profile records, profile fields, schema descriptors, privacy
classifications, preference namespaces, avatar references, profile export, and
profile audit evidence. It does not own account lifecycle, auth handoff,
credentials, organization membership, tenant policy, sessions, or
application-specific business preferences.

Profile APIs frequently mix identity, account, contacts, people data, metadata,
photos, and privacy controls. Macaca normalizes only the profile slice and keeps
provider-specific schemas behind service provider Strategy adapters.

## Supplier Capability Matrix

| Supplier family | Relevant capabilities | Constraints Macaca must model |
| --- | --- | --- |
| OpenID Connect | Standard claims for profile, email, phone, address, locale, zoneinfo, picture, updated_at | Claims are usually read-only from auth context; token payloads are sensitive and belong to auth handoff |
| Microsoft Graph | User profile fields, profile photo, extensions, settings-adjacent data, directory relationships | Directory attributes and relations must be references; photo bytes must be artifact-bounded |
| Google People API | Names, emails, phones, photos, birthdays, bios, locales, organizations, field masks | Field masks and source metadata determine freshness and minimization |
| Okta Universal Directory | Profile schemas, custom attributes, mappings, enrollment constraints | Schema-driven validation, custom attribute redaction, account lifecycle separation |
| Auth0 Profiles | Normalized profile, user metadata, app metadata, identities, picture, verification fields | Metadata namespaces have different visibility and trust; identities are references |
| SCIM 2.0 | User profile attributes, photos, addresses, locale, timezone, preferredLanguage, extensions | Patch semantics, schema extension, active/account fields are out of scope references |
| Clerk/WorkOS-like platforms | Public/private/unsafe metadata, avatars, external identity profile attributes | Metadata visibility classes and provider freshness must be explicit |

## Goals

- Provide provider inspection, schema discovery, profile read/search, patch
  planning, profile update, privacy-field inspection, preference namespace
  read/write, avatar reference management, export planning, export request, and
  artifact retrieval.
- Preserve field minimization, privacy classes, visibility scopes, schema
  validation, version tokens, freshness, avatar artifact boundaries, and audit
  evidence.
- Keep account lifecycle, auth handoff, token handling, sessions,
  organizations, tenants, secrets, identity documents, and application business
  preferences as separate capability boundaries.
- Route every command through canonical service runtime with trace, policy,
  entitlement, resource, approval when required, health, snapshot, and
  structured errors.

## Non-Goals

- Account creation/lifecycle, OAuth/OIDC/SAML handoff, token exchange, session
  binding, password handling, MFA execution, organization membership, tenant
  policy, identity document verification, HRIS provisioning, or app-specific
  onboarding/offboarding workflow.
- Provider-specific profile mapping policy, marketing preference workflow,
  application feature flags, or business preference logic in OS layers.
- Raw credentials, access/refresh tokens, identity documents, raw provider
  payloads, unbounded profile exports, or unbounded avatar/photo bytes in
  observability.

## Ownership And Boundaries

- Pack id: `pack.identity.profile.v1`.
- Family: `identity`.
- Backing service owner: profile service provider family.
- SDK surface: `sdk.packs.identity.profile`.
- Command namespace: `profile.*`.
- Kernel ownership: identity, service-call evidence, policy facade, trace/audit
  primitives, and resource primitives only.
- Application framework ownership: manifest declaration, app-scoped permission
  declarations, lifecycle projection, and effective-capability mementos.
- Runtime-host ownership: provider registration, decorators, mock/unavailable
  providers, and adapter composition through approved composition roots.
- Service ownership: capability discovery, schema validation, profile field
  normalization, provider Strategy dispatch, artifact boundary enforcement,
  redaction, and sanitized audit.

## Command Surface

| Command | Purpose | Side-effect class |
| --- | --- | --- |
| `profile.inspect_provider` | Return schema, field, metadata, preference, avatar, export, freshness, and attribution support | Read-only |
| `profile.describe_schema` | Return profile field, privacy, metadata namespace, preference, avatar, export, and artifact schema | Read-only |
| `profile.read_profile` | Read one normalized profile record with field minimization | Read-only |
| `profile.search_profiles` | Search profiles by authorized filters, cursor, field mask, or account reference | Read-only |
| `profile.plan_patch` | Validate profile field patch, privacy class, schema constraints, version token, and approval | Planning |
| `profile.update_profile` | Apply bounded profile field patch through approved path | Mutating |
| `profile.inspect_privacy_fields` | Return privacy classifications, visibility, retention, and redaction metadata | Read-only |
| `profile.list_preferences` | Read scoped preference namespace values with minimization | Read-only |
| `profile.set_preference` | Set profile-owned preference value, not application business logic | Mutating |
| `profile.plan_avatar_update` | Validate avatar reference, artifact bounds, image metadata, and provider support | Planning |
| `profile.set_avatar_reference` | Store or update avatar artifact/URL reference without leaking raw bytes | Mutating |
| `profile.clear_avatar_reference` | Remove or detach avatar reference when provider supports it | Mutating |
| `profile.sync_profile` | Refresh fields, schema version, freshness, and provider attribution | Read-only or provider sync |
| `profile.plan_export` | Plan profile export scope, format, redaction, retention, and artifact bounds | Planning |
| `profile.export_profile` | Produce profile export artifact handle | Mutating/export |
| `profile.get_artifact_handle` | Retrieve avatar or export artifact metadata without raw payload leakage | Read-only |

Every command must define typed command DTOs, success DTOs, partial/async shapes,
denied/unavailable/unsupported/conflict/quota/stale-data/failure results,
idempotency for side effects, redaction policy, and replay metadata.

## Provider-Neutral DTO Model

- `ProfileScope`: application, tenant, session, task, provider scope, account
  handle, subject reference, profile handle, and permission scope.
- `ProfileProviderCapability`: schema support, field support, metadata namespace
  support, preference support, avatar support, export support, versioning,
  field masks, freshness, limits, attribution, and entitlement.
- `ProfileRecord`: profile handle, account/subject references, fields, metadata
  namespaces, preferences, avatar reference, privacy map, version token,
  freshness, attribution, and redaction class.
- `ProfileField`: normalized field key, value type, value reference, source,
  verification state, visibility, privacy class, retention, mutability,
  localization, and redaction metadata.
- `ProfileSchemaDescriptor`: field definitions, provider/custom schema
  extensions, validation constraints, field masks, mutability, privacy defaults,
  and compatibility hash.
- `ProfileMetadataNamespace`: public, private, app-scoped, provider-scoped,
  directory-scoped, unsafe, and custom namespaces with access policy.
- `ProfilePreference`: profile-owned preference key/value, scope, source,
  retention, privacy class, and conflict metadata; app business preferences are
  out of scope.
- `AvatarReference`: hosted URL, media artifact handle, checksum, dimensions,
  content type, expiry, retention, source, and redaction profile.
- `ProfileAuditReference` and `ProfileArtifactHandle`: event type, actor
  reference, field mask, checksum, expiry, retention, redaction, and replay
  pointer.

## Permission, Policy, Resource, Entitlement, And Approval

Initial scopes:

- `identity.profile.read`
- `identity.profile.write`
- `identity.profile.preferences`
- `identity.profile.avatar`
- `identity.profile.privacy`
- `identity.profile.export`

Policy defaults:

- Scope every call to application id, tenant id, session id, task id, trace id,
  provider scope, account/subject reference, profile handle, field mask, and
  metadata namespace.
- Require approval for sensitive field updates, privacy-class changes, avatar
  updates with retained artifacts, preference namespace writes that leave the
  profile boundary, and retained profile exports.
- Require idempotency keys for mutating commands and export requests.
- Validate field minimization, schema constraints, privacy class, metadata
  namespace access, version tokens, avatar bounds, retention, and freshness
  before provider calls when detectable.
- Return typed `denied`, `unavailable`, `unsupported`, `conflict`,
  `quota_exceeded`, `stale_data`, or `failure` before provider calls when
  preconditions fail.
- Enforce resource budgets for profile search, sync, avatar artifact retrieval,
  export size, provider quotas, storage, and snapshots.

## Trace, Audit, Health, Snapshot, And Replay

Required event families:

- `profile_pack_declared`
- `profile_pack_admission_validated`
- `profile_pack_policy_decision`
- `profile_pack_provider_inspected`
- `profile_pack_service_call_requested`
- `profile_pack_service_call_succeeded`
- `profile_pack_service_call_failed`
- `profile_pack_patch_planned`
- `profile_pack_privacy_inspected`
- `profile_pack_avatar_reference_changed`
- `profile_pack_export_planned`
- `profile_pack_unavailable`
- `profile_pack_snapshot_recorded`

Events include pack id, command name, trace id, application/session/task/tenant
identifiers, profile/account/subject handles, field mask, metadata namespace,
privacy class, policy decision, provider class, descriptor hash, latency,
freshness, version token hash, idempotency hash, bounded resource counters,
result code, and sanitized artifact references. Events must exclude raw
credentials, tokens, identity documents, raw provider payloads, unbounded
profile exports, raw avatar/photo bytes, private keys, and signatures.

Snapshots include descriptor version, provider health, command availability,
schema/field/metadata/preference/avatar/export support, policy-template hash,
redaction profile, freshness, resource counters, and replay pointers.

## SDK And Developer Documentation

SDK discovery must return pack metadata, lifecycle, service mapping, command
schemas, permission scopes, policy templates, examples, availability, health,
provider class, compatibility, diagnostics, and documentation links.

The required developer guide at `docs/developer-packs/identity/profile.md` must
cover:

- Manifest declaration and permission scopes.
- Provider/schema discovery and unavailable diagnostics.
- DTO reference for scopes, provider capability, profile records, fields,
  schemas, metadata namespaces, preferences, privacy classes, avatar references,
  freshness, redaction, and artifacts.
- Examples for reading/searching profiles, planning/updating profile fields,
  inspecting privacy fields, reading/writing profile preferences, updating
  avatar references, syncing profile state, exporting profile evidence, and
  handling conflicts.
- Provider replacement, mock/unavailable provider behavior, trace/audit
  interpretation, redaction guarantees, idempotency, version tokens, field
  minimization, avatar artifact boundaries, and boundaries with account, auth
  handoff, organization, tenant, sessions, secrets, media, and application
  preferences.

## Design Patterns

- **Facade**: SDK exposes discovery and command builders while hiding providers.
- **Command**: every profile operation is a typed command/result DTO.
- **Strategy**: OIDC-like, Graph-like, Google People-like, Okta-like,
  Auth0-like, SCIM-like, Clerk-like, WorkOS-like, and other providers adapt
  behind one service contract.
- **Decorator**: trace, policy, entitlement, approval, resource, idempotency,
  metering, artifact bounds, field minimization, and redaction wrap every call.
- **State**: profile freshness, schema version, field patch, avatar reference,
  preference namespace, export, and provider health are explicit states.
- **Specification**: admission validates declarations, scopes, field masks,
  privacy classes, metadata namespaces, schema constraints, avatar bounds, and
  resource limits.
- **Observer**: trace, audit, provider, profile patch, privacy, avatar, export,
  and snapshot events are subscribable.
- **Memento**: effective capability reports, field patch evidence, privacy
  evidence, avatar references, audit references, and artifact handles are
  replayable bounded records.
- **Abstract Factory**: providers register only through approved runtime-host or
  plugin composition roots.

## Risks And Mitigations

- Risk: profile pack becomes account or auth logic. Mitigation: account
  lifecycle, auth handoff, sessions, tokens, credentials, and MFA are references
  or separate packs.
- Risk: profile pack becomes application preference or marketing preference
  storage. Mitigation: only profile-owned preference namespaces are in scope;
  app business preferences remain app-owned.
- Risk: personal data leaks through traces or snapshots. Mitigation: field
  masks, privacy classes, redaction profiles, artifact handles, and redaction
  tests are mandatory.
- Risk: provider custom schemas become hardcoded OS behavior. Mitigation:
  schemas are descriptors and Strategy provider mappings, not branches in OS
  layers.
- Risk: SDK helpers become a second execution path. Mitigation: helpers only
  build canonical service-call commands and no-direct-provider-call gates cover
  every command.
