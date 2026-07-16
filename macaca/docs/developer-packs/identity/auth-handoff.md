# Identity Auth Handoff Pack

`pack.identity.auth.handoff.v1` is the provider-neutral authentication handoff
contract. It covers handoff planning, hosted-login start handles, callback
verification, token/assertion references, subject evidence, session-binding
evidence, cancellation, expiry cleanup, audit references, and bounded artifact
handles. It does not own account lifecycle, profile writes, tenant policy,
organization membership, session storage internals, credential vaulting, risk
scoring, MFA policy, or application login UI.

## Manifest

```toml
[service_contract]
optional_packs = ["pack.identity.auth.handoff.v1"]
```

Applications should declare this pack when they need a generic auth handoff
surface. Required declarations must fail if redirect/callback support,
token-reference support, session-bind support, policy, entitlement, or provider
availability is missing.

## Permission Scopes

- `identity.auth.handoff.start`
- `identity.auth.handoff.callback`
- `identity.auth.handoff.token_reference`
- `identity.auth.handoff.subject`
- `identity.auth.handoff.session_bind`
- `identity.auth.handoff.audit_export`

Session binding, elevated scopes, external browser/device/passkey host usage,
and retained audit exports require approval.

## Commands

- `auth_handoff.inspect_provider`
- `auth_handoff.describe_schema`
- `auth_handoff.plan_handoff`
- `auth_handoff.start_handoff`
- `auth_handoff.verify_callback`
- `auth_handoff.exchange_token_reference`
- `auth_handoff.inspect_subject_evidence`
- `auth_handoff.plan_session_binding`
- `auth_handoff.bind_session`
- `auth_handoff.cancel_handoff`
- `auth_handoff.expire_handoff`
- `auth_handoff.plan_audit_export`
- `auth_handoff.audit_export_request`
- `auth_handoff.get_artifact_handle`

Callbacks must validate state, nonce, PKCE, RelayState, WebAuthn challenge,
issuer, audience, signature evidence, redirect allowlists, freshness, replay
status, and idempotency before side effects.

## DTO Model

Primary DTOs include `AuthHandoffScope`, `AuthHandoffProviderCapability`,
`AuthProtocolProfile`, `RedirectCallbackDescriptor`, `AuthHandoffPlan`,
`AuthHandoffRecord`, `HandoffCorrelation`, `CallbackVerificationResult`,
`TokenReference`, `AssertionReference`, `SubjectEvidence`,
`SessionBindingEvidence`, `AuthHandoffAuditReference`, and
`AuthHandoffArtifactHandle`.

Raw authorization codes, ID/access/refresh tokens, SAML assertions, WebAuthn
assertion bodies, PKCE verifiers, client secrets, session cookies, raw callback
payloads, raw provider responses, private keys, and signatures are represented
only as references or hashes.

## Unavailable Behavior

The descriptor is preview-unavailable until a provider registers
`service.identity.auth_handoff`. SDK discovery reports
`identity_auth_handoff_provider_not_installed`.

## App-Facing Examples

- Plan and start handoffs with redirect allowlist, state, nonce, PKCE, expiry,
  and idempotency evidence.
- Verify callbacks, exchange token references, and inspect subject evidence
  without exposing raw codes, tokens, assertions, or callback payloads.
- Plan session binding with explicit session refs and approval evidence.
- Cancel or expire stale handoffs and export audit evidence through bounded
  artifact handles.
- Handle replay attempts, stale callbacks, state/nonce conflicts, unsupported
  protocols, unavailable providers, denied scopes, quota, and artifact-denied
  diagnostics as typed results.

## Provider Replacement

Provider classes are `auth-protocol`, `callback-verifier`, `token-reference`,
`mock`, and `unavailable`. OAuth/OIDC, SAML, WebAuthn, hosted-login, device-code,
magic-link, and custom providers adapt behind Strategy providers in runtime-host
or plugins.

## Trace And Audit

Trace evidence records handoff/callback handles, protocol profile, correlation
hashes, subject/session references, provider class, descriptor hash, policy
decision, idempotency hash, replay status, bounded result code, and artifact
refs. Raw callback and token payloads are never emitted.

## Boundaries

Accounts, profiles, organizations, tenants, sessions, secrets, browser
automation, device host capabilities, and application login UI remain separate
capabilities. This pack returns references and evidence, not product login
flows or provider-specific routing.
