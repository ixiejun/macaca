# Identity Profile Pack

`pack.identity.profile.v1` is the provider-neutral profile-data contract for
Macaca applications. It covers profile records, profile fields, schema
descriptors, privacy classifications, profile-owned preferences, avatar
references, freshness/version metadata, profile audit references, and bounded
artifact handles. It does not own account lifecycle, auth handoff, sessions,
tenant policy, organization membership, media processing, or application
business preferences.

## Manifest

```toml
[service_contract]
optional_packs = ["pack.identity.profile.v1"]
```

Use optional declarations for progressive profile enrichment. Required
declarations block readiness until a profile provider is available and policy
admits the requested scopes.

## Permission Scopes

- `identity.profile.read`
- `identity.profile.write`
- `identity.profile.preferences`
- `identity.profile.avatar`
- `identity.profile.privacy`
- `identity.profile.export`

Sensitive field changes, privacy-class changes, avatar updates with retained
artifacts, cross-boundary preference writes, and retained exports require
approval.

## Commands

- `profile.inspect_provider`
- `profile.describe_schema`
- `profile.read_profile`
- `profile.search_profiles`
- `profile.plan_patch`
- `profile.update_profile`
- `profile.inspect_privacy_fields`
- `profile.list_preferences`
- `profile.set_preference`
- `profile.plan_avatar_update`
- `profile.set_avatar_reference`
- `profile.clear_avatar_reference`
- `profile.sync_profile`
- `profile.plan_export`
- `profile.export_profile`
- `profile.get_artifact_handle`

Commands use field masks, bounded pagination, schema descriptors, version
tokens, privacy classes, redaction profiles, and artifact handles.

## DTO Model

Primary DTOs include `ProfileScope`, `ProfileProviderCapability`,
`ProfileRecord`, `ProfileField`, `ProfileSchemaDescriptor`,
`ProfileMetadataNamespace`, `ProfilePreference`, `AvatarReference`,
`ProfileAuditReference`, `ProfileExportPlan`, and `ProfileArtifactHandle`.

Profile field values should be references or minimized values according to field
mask and privacy class. Raw credentials, tokens, identity documents, raw
provider payloads, unbounded profile exports, raw avatar/photo bytes, private
keys, and signatures must not enter observability.

## Unavailable Behavior

The reference descriptor is preview-unavailable until a serviceized provider
registers command schemas for `service.identity.profile`. SDK discovery reports
`identity_profile_provider_not_installed`.

## App-Facing Examples

- Read or search profiles with field masks, privacy classes, and bounded pages.
- Plan field updates before mutating profile state and preserve version-token
  conflict diagnostics.
- Inspect privacy fields, list or set profile-owned preferences, and keep
  application business preferences outside this pack.
- Plan avatar reference updates through bounded artifact handles without raw
  photo bytes.
- Sync profile state, plan exports, and handle conflicts, unavailable
  providers, denied fields, schema mismatch, quota, stale data, and
  artifact-denied diagnostics as typed results.

## Provider Replacement

Provider classes are `profile-schema`, `profile-privacy`, `profile-avatar`,
`mock`, and `unavailable`. Provider custom schemas are descriptor data and
Strategy adapter behavior, not OS-layer branches.

## Trace And Audit

Profile trace evidence records pack id, command, profile/account/subject
handles, field masks, metadata namespace, privacy class, provider class,
descriptor hash, idempotency hash, version hash, bounded resource counters, and
sanitized artifact refs.

## Boundaries

Account identifiers and lifecycle belong to `pack.identity.account.v1`. Login,
tokens, and callbacks belong to `pack.identity.auth.handoff.v1`. Organization
membership and roles belong to `pack.identity.organization.v1`. Tenant policy
belongs to `pack.identity.tenant.v1`. Image processing belongs to media packs;
this pack stores only avatar references.
