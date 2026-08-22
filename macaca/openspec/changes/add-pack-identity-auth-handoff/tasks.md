## 1. Research, Scope, And Governance

- [x] 1.1 Re-read architecture governance, microkernel boundaries, serviceization allowlist, design-pattern guidance, umbrella catalog proposal, and this child proposal before implementation.
- [x] 1.2 Record supplier/API findings for OAuth 2.0 Authorization Code, PKCE, OpenID Connect Core, SAML Web Browser SSO, WebAuthn/passkeys, Auth0 Authentication API, Okta OAuth/OIDC, WorkOS SSO, Clerk sessions/sign-in, Microsoft Entra ID, and similar providers.
- [x] 1.3 Confirm the pack scope: handoff planning, start requests, callback verification, token/assertion references, subject evidence, session binding evidence, cancellation, expiry cleanup, audit export, artifacts, freshness, attribution, and redaction.
- [x] 1.4 Explicitly exclude account lifecycle, profile updates, organization membership, tenant policy, session store implementation, credential vaulting, password verification, MFA policy engine, IdP implementation, risk scoring, and application login UI/workflows.
- [x] 1.5 Inventory existing descriptors, SDK clients, service runtime hooks, policy gates, approval gates, resource gates, entitlement gates, trace/audit helpers, artifact handles, mock providers, unavailable providers, callback adapters, and session-facade references that can be reused.
- [x] 1.6 Record GitNexus CRITICAL/HIGH findings as memo-only evidence before implementation commits.

## 2. Contract, Descriptor, And DTO Schema

- [x] 2.1 Define descriptor metadata for `pack.identity.auth.handoff.v1`, including family, lifecycle, stability, command schemas, scopes, policy template, resource budgets, redaction profile, SDK metadata, docs link, compatibility, health, and diagnostics.
- [x] 2.2 Define `AuthHandoffScope`, `AuthHandoffProviderCapability`, `AuthHandoffFreshness`, `AuthHandoffAttribution`, and `AuthHandoffRedactionPolicy`.
- [x] 2.3 Define `AuthProtocolProfile` for OAuth2 authorization code, OIDC authorization code, SAML Web SSO, WebAuthn assertion, passkey, device code, magic link, and custom provider profiles.
- [x] 2.4 Define `AuthHandoffPlan`, `AuthHandoffRecord`, redirect/callback descriptors, scopes, hints, expiry, provider class, freshness, and redaction class.
- [x] 2.5 Define `HandoffCorrelation` for state hash, nonce hash, PKCE challenge hash, RelayState hash, WebAuthn challenge hash, device/user code reference, CSRF binding, and replay pointer.
- [x] 2.6 Define `CallbackVerificationResult` with verified state, issuer, audience, signature, nonce, challenge, redirect URI, subject reference, claim references, failure reason, freshness, and replay metadata.
- [x] 2.7 Define `TokenReference` and `AssertionReference` with opaque handle, token/assertion class, expiry, scope/claim hints, storage boundary, refreshability, redaction, and access policy without raw values.
- [x] 2.8 Define `SubjectEvidence`, provider subject, account reference, profile claim references, assurance level, authentication context, organization/tenant hints, freshness, and redaction.
- [x] 2.9 Define `SessionBindingEvidence`, Macaca session reference, subject evidence reference, approval reference, expiry, binding state, and replay pointer.
- [x] 2.10 Define `AuthHandoffAuditReference`, `AuthHandoffAuditExportPlan`, and `AuthHandoffArtifactHandle`, including event type, bounded reason code, checksum, expiry, retention, redaction, and replay pointer.
- [x] 2.11 Define typed `success`, `partial`, `accepted`, `action_required`, `denied`, `unavailable`, `unsupported`, `conflict`, `quota_exceeded`, `stale_data`, `replay_rejected`, and `failure` result envelopes for every command family.
- [x] 2.12 Add descriptor hashing and compatibility tests for valid descriptors, rejected invalid descriptors, stable hashes, and schema evolution.

## 3. Command Surface And Handoff State Semantics

- [x] 3.1 Implement command schemas for `auth_handoff.inspect_provider` and `auth_handoff.describe_schema`.
- [x] 3.2 Implement command schemas for `auth_handoff.plan_handoff` and `auth_handoff.start_handoff`, including redirect allowlists, protocol profile, scopes, state/nonce/PKCE/challenge, expiry, idempotency, and approval.
- [x] 3.3 Implement command schemas for `auth_handoff.verify_callback`, including callback descriptor, state/nonce/RelayState/challenge validation, issuer/audience/signature checks, replay rejection, and freshness.
- [x] 3.4 Implement command schemas for `auth_handoff.exchange_token_reference` without exposing raw authorization codes, tokens, assertions, or provider responses.
- [x] 3.5 Implement command schemas for `auth_handoff.inspect_subject_evidence`.
- [x] 3.6 Implement command schemas for `auth_handoff.plan_session_binding` and `auth_handoff.bind_session`.
- [x] 3.7 Implement command schemas for `auth_handoff.cancel_handoff` and `auth_handoff.expire_handoff`.
- [x] 3.8 Implement command schemas for `auth_handoff.plan_audit_export`, `auth_handoff.audit_export_request`, and `auth_handoff.get_artifact_handle`.
- [x] 3.9 Add validation for redirect allowlists, callback method/binding, protocol support, state/nonce/PKCE/RelayState/challenge correlation, issuer/audience/signature checks, token-reference boundaries, subject/session binding, replay protection, expiry, idempotency, approval, export bounds, and stale-data conditions.

## 4. Permission, Policy, Resource, Entitlement, And Approval

- [x] 4.1 Add declaration validation for `identity.auth.handoff.start`, `identity.auth.handoff.callback`, `identity.auth.handoff.token_reference`, `identity.auth.handoff.subject`, `identity.auth.handoff.session_bind`, and `identity.auth.handoff.audit_export`.
- [x] 4.2 Require policy decisions before every command and approval before elevated scopes, session binding, external browser/device/passkey host usage, and retained audit exports.
- [x] 4.3 Require entitlement checks for provider access, protocol support, redirect/callback support, token-reference support, subject-evidence support, session-bind support, audit export support, and tenant/provider scope access.
- [x] 4.4 Reserve and meter resources for pending handoffs, callback attempts, token exchange, subject inspection, session binding, audit export size, provider quotas, storage, and snapshots.
- [x] 4.5 Return typed denied/unavailable/unsupported/conflict/quota/stale-data/replay-rejected outcomes before provider calls when preconditions fail.
- [x] 4.6 Add tests proving denied, unavailable, unsupported, conflict, quota, stale-data, replay-rejected, token-redaction, and callback-redaction paths do not call concrete providers when preconditions fail.

## 5. Service Provider, Provider Strategy, And Unavailable Behavior

- [x] 5.1 Add the auth handoff service provider interface with descriptor, lifecycle, health, snapshot, shutdown, timeout, cancellation, async callback/export support, and command dispatch.
- [x] 5.2 Implement provider Strategy adapters behind the service interface without provider-name routing in OS-layer command logic.
- [x] 5.3 Implement a mock provider with synthetic OAuth/OIDC/SAML/WebAuthn handoffs, callbacks, token references, subject evidence, session binding, replay rejection, expiry, and configurable capability gaps.
- [x] 5.4 Implement an unavailable provider that returns explicit unavailable diagnostics for every command without fake success.
- [x] 5.5 Normalize provider errors into Macaca result envelopes while preserving sanitized provider class, bounded code, retriable flag, handoff state, callback state, freshness, replay status, and replay pointer.
- [x] 5.6 Add provider capability discovery for protocol support, PKCE/nonce/state support, SAML support, WebAuthn/passkey support, device/magic link support, callback bindings, token-reference support, session-bind support, expiry, freshness, limits, attribution, and entitlement.

## 6. SDK, Admission, Examples, And Developer Documentation

- [x] 6.1 Extend pack catalog and SDK discovery for `pack.identity.auth.handoff.v1` with schemas, scopes, examples, availability, health, diagnostics, compatibility, provider class, and docs metadata.
- [x] 6.2 Extend application admission so required declarations block on unavailable/denied states and optional declarations degrade explicitly with effective capability mementos.
- [x] 6.3 Add SDK command helper builders that only construct canonical traced service calls and never construct providers.
- [x] 6.4 Add generic app-facing examples for planning/starting handoffs, handling callbacks, exchanging token references, inspecting subject evidence, planning/binding sessions, cancelling handoffs, exporting audit evidence, and handling replay/conflicts.
- [x] 6.5 Create `docs/developer-packs/identity/auth-handoff.md` with purpose, manifest declaration, scopes, commands, DTOs, examples, unavailable diagnostics, provider replacement, trace/audit behavior, redaction, idempotency, redirect allowlists, replay protection, expiry, protocol support, token-reference boundaries, and account/profile/organization/tenant/session/secrets/browser/device/application-login boundaries.
- [x] 6.6 Cross-link the developer guide from SDK discovery metadata and the industrial pack catalog index.

## 7. Trace, Audit, Replay, And Redaction

- [x] 7.1 Emit sanitized declaration, admission, provider-inspection, policy, entitlement, approval, resource, service-call, handoff-planning, callback-verification, token-reference-exchange, session-binding-planning, unavailable, health, snapshot, and result events.
- [x] 7.2 Add trace schemas for `auth_handoff_pack_declared`, `auth_handoff_pack_admission_validated`, `auth_handoff_pack_policy_decision`, `auth_handoff_pack_provider_inspected`, `auth_handoff_pack_service_call_requested`, `auth_handoff_pack_service_call_succeeded`, `auth_handoff_pack_service_call_failed`, `auth_handoff_pack_handoff_planned`, `auth_handoff_pack_callback_verified`, `auth_handoff_pack_token_reference_exchanged`, `auth_handoff_pack_session_binding_planned`, `auth_handoff_pack_unavailable`, and `auth_handoff_pack_snapshot_recorded`.
- [x] 7.3 Add replay tests proving every command is trace-addressable through the canonical service runtime path.
- [x] 7.4 Add snapshot tests proving descriptor, provider health, command availability, protocol/callback/token/session-bind support, policy-template hash, redaction profile, pending handoff counters, resource counters, freshness, and replay pointers are retained without raw payload leakage.
- [x] 7.5 Add redaction tests proving raw authorization codes, raw tokens, ID tokens, SAML assertions, WebAuthn assertion bodies, PKCE verifiers, client secrets, session cookies, raw callback payloads, raw provider responses, private keys, and signatures never enter logs, traces, snapshots, or SDK diagnostics.
- [x] 7.6 Add replay-protection tests proving reused state/nonce/RelayState/challenge or stale callback attempts are rejected before provider side effects.

## 8. Boundary, Quality, And Validation Gates

- [x] 8.1 Add dependency gates proving kernel, SDK, shells, and generic application framework do not import concrete auth handoff providers.
- [x] 8.2 Add no-direct-provider-call tests proving all callable operations traverse descriptor-owned service registration and typed service commands.
- [x] 8.3 Add canonical execution-path tests covering read-only, start, callback verify, token-reference exchange, subject evidence, session binding, cancel, expire, audit export, denied, unavailable, unsupported, conflict, quota, stale-data, replay-rejected, and redaction paths.
- [x] 8.4 Add provider replacement tests for built-in, plugin, remote, mock, and unavailable providers.
- [x] 8.5 Add boundary tests proving auth handoff commands do not perform account lifecycle, profile updates, organization membership changes, tenant policy changes, session store implementation, credential vaulting, MFA policy decisions, risk scoring, or application-specific login UI behavior.
- [x] 8.6 Add file-size and module-ownership checks for any new implementation files.
- [x] 8.7 Run `openspec validate add-pack-identity-auth-handoff --strict`.
- [x] 8.8 Run targeted cargo checks/tests, dependency-boundary gates, audit replay checks, redaction checks, callback replay checks, and auth/account/profile/organization/tenant/session/secrets/browser/device boundary checks before marking implementation tasks complete.
