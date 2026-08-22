## 1. Research, Scope, And Governance

- [x] 1.1 Re-read architecture governance, microkernel boundaries, serviceization allowlist, design-pattern guidance, umbrella catalog proposal, and this child proposal before implementation.
- [x] 1.2 Record supplier/API findings for Okta Users/Lifecycle, Auth0 Management Users, Microsoft Graph Users, Google Admin Directory Users, SCIM 2.0 Users, WorkOS User Management, Clerk Users, and similar account providers.
- [x] 1.3 Confirm the pack scope: account records, identifiers, lifecycle state, linked identity references, status sync, recovery references, account audit export, artifact handles, freshness, attribution, and redaction.
- [x] 1.4 Explicitly exclude OAuth/OIDC/SAML handoff, token exchange, session binding, raw password or credential storage, MFA challenge execution, profile preference management, organization membership, tenant isolation policy, and application-specific account workflows.
- [x] 1.5 Inventory existing descriptors, SDK clients, service runtime hooks, policy gates, approval gates, resource gates, entitlement gates, trace/audit helpers, artifact handles, mock providers, and unavailable providers that can be reused.
- [x] 1.6 Record GitNexus CRITICAL/HIGH findings as memo-only evidence before implementation commits.

## 2. Contract, Descriptor, And DTO Schema

- [x] 2.1 Define descriptor metadata for `pack.identity.account.v1`, including family, lifecycle, stability, command schemas, scopes, policy template, resource budgets, redaction profile, SDK metadata, docs link, compatibility, health, and diagnostics.
- [x] 2.2 Define `AccountScope`, `AccountProviderCapability`, `AccountFreshness`, `AccountAttribution`, and `AccountRedactionPolicy`.
- [x] 2.3 Define `AccountRecord`, account handle, stable subject reference, identifiers, minimized attributes, lifecycle state, linked identities, organization/tenant references, recovery references, audit references, version token, freshness, and redaction class.
- [x] 2.4 Define `AccountIdentifier` for username, email, phone, user principal name, alias, external id, SCIM id, directory id, provider subject id, and verification state.
- [x] 2.5 Define `AccountAttributePatch` for bounded mutable account attributes and custom schema references without profile-preference ownership.
- [x] 2.6 Define `AccountLifecycleState`, provider state mapping, lifecycle transition request/result, transition constraints, and conflict/stale-data diagnostics.
- [x] 2.7 Define `LinkedIdentityReference`, provider class, issuer/connection reference, external subject, assurance level, link state, freshness, and replay pointer.
- [x] 2.8 Define `AccountRecoveryReference` with recovery email/phone references, reset-flow references, support case references, and redaction profile without raw tokens or secrets.
- [x] 2.9 Define `AccountAuditReference`, `AccountAuditExportPlan`, and `AccountArtifactHandle`, including event type, actor reference, timestamp, bounded reason code, checksum, expiry, retention, redaction, and replay pointer.
- [x] 2.10 Define typed `success`, `partial`, `accepted`, `denied`, `unavailable`, `unsupported`, `conflict`, `quota_exceeded`, `stale_data`, and `failure` result envelopes for every command family.
- [x] 2.11 Add descriptor hashing and compatibility tests for valid descriptors, rejected invalid descriptors, stable hashes, and schema evolution.

## 3. Command Surface And Lifecycle Semantics

- [x] 3.1 Implement command schemas for `account.inspect_provider` and `account.describe_schema`.
- [x] 3.2 Implement command schemas for `account.plan_create` and `account.create_account`, including identifier uniqueness, idempotency, and approval.
- [x] 3.3 Implement command schemas for `account.read_account` and `account.search_accounts`, including pagination, filters, freshness, and redaction.
- [x] 3.4 Implement command schemas for `account.plan_update` and `account.update_account`, including version tokens and bounded attribute patches.
- [x] 3.5 Implement command schemas for `account.plan_lifecycle_transition` and `account.lifecycle_transition_request`.
- [x] 3.6 Implement command schemas for `account.link_identity` and `account.unlink_identity`, including conflict checks and linked identity provenance.
- [x] 3.7 Implement command schemas for `account.sync_status` with provider freshness and stale-data handling.
- [x] 3.8 Implement command schemas for `account.set_recovery_reference` without raw reset tokens or secrets.
- [x] 3.9 Implement command schemas for `account.inspect_account_audit`, `account.plan_audit_export`, `account.audit_export_request`, and `account.get_artifact_handle`.
- [x] 3.10 Add validation for tenant isolation, identifier uniqueness, lifecycle transitions, version tokens, linked identity conflicts, recovery reference sensitivity, pagination, export bounds, idempotency, approval, and stale-data conditions.

## 4. Permission, Policy, Resource, Entitlement, And Approval

- [x] 4.1 Add declaration validation for `identity.account.read`, `identity.account.create`, `identity.account.update`, `identity.account.lifecycle`, `identity.account.link_identity`, and `identity.account.audit_export`.
- [x] 4.2 Require policy decisions before every command and approval before account creation, disabling, suspension, deletion, recovery, linked identity changes, recovery reference changes, and retained audit exports.
- [x] 4.3 Require entitlement checks for provider access, schema support, create support, update support, lifecycle support, linked identity support, recovery reference support, audit export support, and tenant/provider scope access.
- [x] 4.4 Reserve and meter resources for account search, status sync, linked identity changes, audit export size, provider quotas, storage, and snapshots.
- [x] 4.5 Return typed denied/unavailable/unsupported/conflict/quota/stale-data outcomes before provider calls when preconditions fail.
- [x] 4.6 Add tests proving denied, unavailable, unsupported, conflict, quota, stale-data, and redaction paths do not call concrete providers when preconditions fail.

## 5. Service Provider, Provider Strategy, And Unavailable Behavior

- [ ] 5.1 Add the account service provider interface with descriptor, lifecycle, health, snapshot, shutdown, timeout, cancellation, async export support, and command dispatch.
- [ ] 5.2 Implement provider Strategy adapters behind the service interface without provider-name routing in OS-layer command logic.
- [ ] 5.3 Implement a mock provider with synthetic accounts, identifiers, lifecycle states, linked identities, recovery references, audit references, stale-data states, and configurable capability gaps.
- [x] 5.4 Implement an unavailable provider that returns explicit unavailable diagnostics for every command without fake success.
- [ ] 5.5 Normalize provider errors into Macaca result envelopes while preserving sanitized provider class, bounded code, retriable flag, lifecycle state, freshness, version conflict, and replay pointer.
- [x] 5.6 Add provider capability discovery for create/update/search support, lifecycle transitions, linked identity support, recovery reference support, audit export support, schema extension support, pagination, versioning, freshness, limits, attribution, and entitlement.

## 6. SDK, Admission, Examples, And Developer Documentation

- [x] 6.1 Extend pack catalog and SDK discovery for `pack.identity.account.v1` with schemas, scopes, examples, availability, health, diagnostics, compatibility, provider class, and docs metadata.
- [ ] 6.2 Extend application admission so required declarations block on unavailable/denied states and optional declarations degrade explicitly with effective capability mementos.
- [x] 6.3 Add SDK command helper builders that only construct canonical traced service calls and never construct providers.
- [x] 6.4 Add generic app-facing examples for planning/creating accounts, reading/searching, updating metadata, lifecycle transitions, linking/unlinking identities, syncing status, setting recovery references, inspecting audit, exporting audit evidence, and handling conflicts.
- [x] 6.5 Create `docs/developer-packs/identity/account.md` with purpose, manifest declaration, scopes, commands, DTOs, examples, unavailable diagnostics, provider replacement, trace/audit behavior, redaction, idempotency, version tokens, freshness, lifecycle semantics, linked identity semantics, and profile/auth/organization/tenant/session boundaries.
- [x] 6.6 Cross-link the developer guide from SDK discovery metadata and the industrial pack catalog index.

## 7. Trace, Audit, Replay, And Redaction

- [x] 7.1 Emit sanitized declaration, admission, provider-inspection, policy, entitlement, approval, resource, service-call, create-planning, lifecycle-planning, linked-identity-change, unavailable, health, snapshot, and result events.
- [ ] 7.2 Add trace schemas for `account_pack_declared`, `account_pack_admission_validated`, `account_pack_policy_decision`, `account_pack_provider_inspected`, `account_pack_service_call_requested`, `account_pack_service_call_succeeded`, `account_pack_service_call_failed`, `account_pack_create_planned`, `account_pack_lifecycle_planned`, `account_pack_identity_link_changed`, `account_pack_unavailable`, and `account_pack_snapshot_recorded`.
- [x] 7.3 Add replay tests proving every command is trace-addressable through the canonical service runtime path.
- [x] 7.4 Add snapshot tests proving descriptor, provider health, command availability, schema/lifecycle/link/audit support, policy-template hash, redaction profile, resource counters, freshness, and replay pointers are retained without raw payload leakage.
- [x] 7.5 Add redaction tests proving raw credentials, password hashes, reset tokens, recovery codes, MFA secrets, access tokens, refresh tokens, raw provider payloads, identity documents, and unbounded audit exports never enter logs, traces, snapshots, or SDK diagnostics.
- [x] 7.6 Add artifact-boundary tests proving audit exports are represented as bounded handles, hashes, and redacted metadata in observability surfaces.

## 8. Boundary, Quality, And Validation Gates

- [x] 8.1 Add dependency gates proving kernel, SDK, shells, and generic application framework do not import concrete account providers.
- [x] 8.2 Add no-direct-provider-call tests proving all callable operations traverse descriptor-owned service registration and typed service commands.
- [x] 8.3 Add canonical execution-path tests covering read-only, create, update, lifecycle transition, linked identity, status sync, recovery reference, audit export, denied, unavailable, unsupported, conflict, quota, stale-data, and redaction paths.
- [ ] 8.4 Add provider replacement tests for built-in, plugin, remote, mock, and unavailable providers.
- [ ] 8.5 Add boundary tests proving account commands do not perform auth handoff, token exchange, credential storage, MFA challenge execution, profile preference management, organization membership changes, tenant policy changes, or application-specific workflows.
- [x] 8.6 Add file-size and module-ownership checks for any new implementation files.
- [ ] 8.7 Run `openspec validate add-pack-identity-account --strict`.
- [ ] 8.8 Run targeted cargo checks/tests, dependency-boundary gates, audit replay checks, redaction checks, artifact-boundary checks, and account/profile/auth/organization/tenant/session boundary checks before marking implementation tasks complete.
