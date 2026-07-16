## 1. Research, Scope, And Governance

- [x] 1.1 Re-read architecture governance, microkernel boundaries, serviceization allowlist, design-pattern guidance, umbrella catalog proposal, and this child proposal before implementation.
- [x] 1.2 Record supplier/API findings for OpenID Connect Standard Claims, Microsoft Graph user/profile photo, Google People API, Okta Universal Directory profile schema, Auth0 user profiles/metadata, SCIM User schema, Clerk metadata, WorkOS user management, and similar profile providers.
- [x] 1.3 Confirm the pack scope: profile records, fields, schema descriptors, privacy classifications, profile-owned preferences, avatar references, profile sync, export artifacts, freshness, attribution, and redaction.
- [x] 1.4 Explicitly exclude account creation/lifecycle, OAuth/OIDC/SAML handoff, token exchange, session binding, password or credential handling, MFA execution, organization membership, tenant policy, identity document verification, marketing workflow, and application business preferences.
- [x] 1.5 Inventory existing descriptors, SDK clients, service runtime hooks, policy gates, approval gates, resource gates, entitlement gates, trace/audit helpers, artifact handles, mock providers, and unavailable providers that can be reused.
- [x] 1.6 Record GitNexus CRITICAL/HIGH findings as memo-only evidence before implementation commits.

## 2. Contract, Descriptor, And DTO Schema

- [x] 2.1 Define descriptor metadata for `pack.identity.profile.v1`, including family, lifecycle, stability, command schemas, scopes, policy template, resource budgets, redaction profile, SDK metadata, docs link, compatibility, health, and diagnostics.
- [x] 2.2 Define `ProfileScope`, `ProfileProviderCapability`, `ProfileFreshness`, `ProfileAttribution`, and `ProfileRedactionPolicy`.
- [x] 2.3 Define `ProfileRecord`, account/subject references, fields, metadata namespaces, preferences, avatar reference, privacy map, version token, freshness, and redaction class.
- [x] 2.4 Define `ProfileField`, normalized key, value type, value reference, source, verification state, visibility, privacy class, retention, mutability, localization, and redaction metadata.
- [x] 2.5 Define `ProfileSchemaDescriptor`, field definitions, provider/custom schema extensions, validation constraints, field masks, mutability, privacy defaults, and compatibility hash.
- [x] 2.6 Define `ProfileMetadataNamespace` for public, private, app-scoped, provider-scoped, directory-scoped, unsafe, and custom namespaces with access policy.
- [x] 2.7 Define `ProfilePreference` for profile-owned preferences with scope, source, retention, privacy class, conflict metadata, and application-business-preference boundary markers.
- [x] 2.8 Define `AvatarReference`, hosted URL, media artifact handle, checksum, dimensions, content type, expiry, retention, source, and redaction profile.
- [x] 2.9 Define `ProfileAuditReference`, `ProfileExportPlan`, and `ProfileArtifactHandle`, including event type, actor reference, field mask, checksum, expiry, retention, redaction, and replay pointer.
- [x] 2.10 Define typed `success`, `partial`, `accepted`, `denied`, `unavailable`, `unsupported`, `conflict`, `quota_exceeded`, `stale_data`, and `failure` result envelopes for every command family.
- [x] 2.11 Add descriptor hashing and compatibility tests for valid descriptors, rejected invalid descriptors, stable hashes, and schema evolution.

## 3. Command Surface And Profile Semantics

- [x] 3.1 Implement command schemas for `profile.inspect_provider` and `profile.describe_schema`.
- [x] 3.2 Implement command schemas for `profile.read_profile` and `profile.search_profiles`, including field masks, pagination, freshness, and redaction.
- [x] 3.3 Implement command schemas for `profile.plan_patch` and `profile.update_profile`, including schema validation, privacy classes, version tokens, and idempotency.
- [x] 3.4 Implement command schemas for `profile.inspect_privacy_fields`.
- [x] 3.5 Implement command schemas for `profile.list_preferences` and `profile.set_preference`, including namespace access and application-business-preference rejection.
- [x] 3.6 Implement command schemas for `profile.plan_avatar_update`, `profile.set_avatar_reference`, and `profile.clear_avatar_reference`, including artifact bounds and media references.
- [x] 3.7 Implement command schemas for `profile.sync_profile` with provider freshness and stale-data handling.
- [x] 3.8 Implement command schemas for `profile.plan_export`, `profile.export_profile`, and `profile.get_artifact_handle`.
- [x] 3.9 Add validation for field minimization, schema constraints, privacy class, metadata namespace access, version tokens, avatar bounds, artifact retention, export bounds, idempotency, approval, and stale-data conditions.

## 4. Permission, Policy, Resource, Entitlement, And Approval

- [x] 4.1 Add declaration validation for `identity.profile.read`, `identity.profile.write`, `identity.profile.preferences`, `identity.profile.avatar`, `identity.profile.privacy`, and `identity.profile.export`.
- [ ] 4.2 Require policy decisions before every command and approval before sensitive field updates, privacy-class changes, retained avatar artifacts, namespace writes that leave the profile boundary, and retained profile exports.
- [ ] 4.3 Require entitlement checks for provider access, schema support, field support, preference support, avatar support, privacy inspection support, export support, and tenant/provider scope access.
- [ ] 4.4 Reserve and meter resources for profile search, sync, avatar artifact retrieval, export size, provider quotas, storage, and snapshots.
- [ ] 4.5 Return typed denied/unavailable/unsupported/conflict/quota/stale-data outcomes before provider calls when preconditions fail.
- [ ] 4.6 Add tests proving denied, unavailable, unsupported, conflict, quota, stale-data, avatar-artifact, and redaction paths do not call concrete providers when preconditions fail.

## 5. Service Provider, Provider Strategy, And Unavailable Behavior

- [x] 5.1 Add the profile service provider interface with descriptor, lifecycle, health, snapshot, shutdown, timeout, cancellation, async export support, and command dispatch.
- [ ] 5.2 Implement provider Strategy adapters behind the service interface without provider-name routing in OS-layer command logic.
- [ ] 5.3 Implement a mock provider with synthetic profiles, schemas, privacy classes, metadata namespaces, preferences, avatar references, export artifacts, stale-data states, and configurable capability gaps.
- [x] 5.4 Implement an unavailable provider that returns explicit unavailable diagnostics for every command without fake success.
- [ ] 5.5 Normalize provider errors into Macaca result envelopes while preserving sanitized provider class, bounded code, retriable flag, field mask, freshness, version conflict, and replay pointer.
- [ ] 5.6 Add provider capability discovery for schema support, field support, metadata namespace support, preference support, avatar support, export support, versioning, field masks, freshness, limits, attribution, and entitlement.

## 6. SDK, Admission, Examples, And Developer Documentation

- [x] 6.1 Extend pack catalog and SDK discovery for `pack.identity.profile.v1` with schemas, scopes, examples, availability, health, diagnostics, compatibility, provider class, and docs metadata.
- [ ] 6.2 Extend application admission so required declarations block on unavailable/denied states and optional declarations degrade explicitly with effective capability mementos.
- [x] 6.3 Add SDK command helper builders that only construct canonical traced service calls and never construct providers.
- [x] 6.4 Add generic app-facing examples for reading/searching profiles, planning/updating fields, inspecting privacy, reading/writing profile-owned preferences, updating avatar references, syncing profile state, exporting profile evidence, and handling conflicts.
- [x] 6.5 Create `docs/developer-packs/identity/profile.md` with purpose, manifest declaration, scopes, commands, DTOs, examples, unavailable diagnostics, provider replacement, trace/audit behavior, redaction, idempotency, version tokens, field minimization, privacy classes, avatar artifact boundaries, and account/auth/organization/tenant/session/secret/media/application-preference boundaries.
- [x] 6.6 Cross-link the developer guide from SDK discovery metadata and the industrial pack catalog index.

## 7. Trace, Audit, Replay, And Redaction

- [ ] 7.1 Emit sanitized declaration, admission, provider-inspection, policy, entitlement, approval, resource, service-call, patch-planning, privacy-inspection, avatar-reference-change, export-planning, unavailable, health, snapshot, and result events.
- [ ] 7.2 Add trace schemas for `profile_pack_declared`, `profile_pack_admission_validated`, `profile_pack_policy_decision`, `profile_pack_provider_inspected`, `profile_pack_service_call_requested`, `profile_pack_service_call_succeeded`, `profile_pack_service_call_failed`, `profile_pack_patch_planned`, `profile_pack_privacy_inspected`, `profile_pack_avatar_reference_changed`, `profile_pack_export_planned`, `profile_pack_unavailable`, and `profile_pack_snapshot_recorded`.
- [x] 7.3 Add replay tests proving every command is trace-addressable through the canonical service runtime path.
- [ ] 7.4 Add snapshot tests proving descriptor, provider health, command availability, schema/field/metadata/preference/avatar/export support, policy-template hash, redaction profile, resource counters, freshness, and replay pointers are retained without raw payload leakage.
- [ ] 7.5 Add redaction tests proving raw credentials, tokens, identity documents, raw provider payloads, unbounded profile exports, raw avatar/photo bytes, private keys, and signatures never enter logs, traces, snapshots, or SDK diagnostics.
- [ ] 7.6 Add artifact-boundary tests proving avatar/photo resources and profile exports are represented as bounded handles, hashes, and redacted metadata in observability surfaces.

## 8. Boundary, Quality, And Validation Gates

- [x] 8.1 Add dependency gates proving kernel, SDK, shells, and generic application framework do not import concrete profile providers.
- [x] 8.2 Add no-direct-provider-call tests proving all callable operations traverse descriptor-owned service registration and typed service commands.
- [ ] 8.3 Add canonical execution-path tests covering read-only, search, patch, privacy inspection, preference read/write, avatar reference update, sync, export, denied, unavailable, unsupported, conflict, quota, stale-data, and redaction paths.
- [ ] 8.4 Add provider replacement tests for built-in, plugin, remote, mock, and unavailable providers.
- [ ] 8.5 Add boundary tests proving profile commands do not perform account lifecycle, auth handoff, token exchange, credential storage, MFA execution, organization membership changes, tenant policy changes, media processing, or application-specific preference workflows.
- [x] 8.6 Add file-size and module-ownership checks for any new implementation files.
- [x] 8.7 Run `openspec validate add-pack-identity-profile --strict`.
- [ ] 8.8 Run targeted cargo checks/tests, dependency-boundary gates, audit replay checks, redaction checks, artifact-boundary checks, and profile/account/auth/organization/tenant/session/media/application-preference boundary checks before marking implementation tasks complete.
