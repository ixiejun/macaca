# Identity Auth Handoff Pack Research

## Purpose

This note records supplier/API research, Macaca provider-neutral mapping,
explicit non-goals, existing platform inventory, and GitNexus memo evidence for
`pack.identity.auth.handoff.v1`. The auth handoff pack owns handoff planning,
start requests, callback verification, token/assertion references, subject
evidence, session binding evidence, cancellation, expiry cleanup, audit export,
artifacts, freshness, attribution, and redaction. It must not own account
lifecycle, profile updates, organization membership, tenant policy, session
store implementation, credential vaulting, password verification, MFA policy
engines, IdP implementation, risk scoring, or application login UI workflows.

## Source Baseline

- OAuth 2.0 Authorization Framework and PKCE:
  <https://datatracker.ietf.org/doc/html/rfc6749> and
  <https://datatracker.ietf.org/doc/html/rfc7636>
- OpenID Connect Core:
  <https://openid.net/specs/openid-connect-core-1_0.html>
- SAML 2.0 Web Browser SSO profile:
  <https://docs.oasis-open.org/security/saml/v2.0/saml-profiles-2.0-os.pdf>
- WebAuthn Level 3:
  <https://www.w3.org/TR/webauthn-3/>
- Auth0 Authentication API:
  <https://auth0.com/docs/api/authentication>
- Okta OAuth 2.0 and OIDC API:
  <https://developer.okta.com/docs/api/openapi/okta-oauth/oauth/overview/>
- WorkOS SSO:
  <https://workos.com/docs/sso>
- Clerk sessions and sign-in:
  <https://clerk.com/docs/references/backend/session>
- Microsoft identity platform OAuth/OIDC:
  <https://learn.microsoft.com/en-us/entra/identity-platform/v2-protocols>

## Supplier API Notes

- OAuth 2.0 and PKCE contribute authorization code flow, state, redirect URI,
  code verifier/challenge, token exchange, scope, expiry, and error semantics.
  Macaca should store only opaque token references and correlation hashes.
- OIDC contributes nonce, ID token claims, UserInfo, issuer, audience,
  signature, authentication context, and subject evidence. Macaca should verify
  evidence through typed provider commands and never expose raw tokens.
- SAML Web Browser SSO contributes RelayState, assertions, bindings,
  signatures, audience restrictions, and subject confirmation. Macaca should
  convert assertions into bounded assertion references and subject evidence.
- WebAuthn/passkeys contribute challenge, origin, credential ID, authenticator
  data, client data, and assertion verification. Macaca should keep passkey
  verification evidence separate from account/profile mutation semantics.
- Auth0, Okta, WorkOS, Clerk, and Microsoft Entra contribute provider-specific
  redirect/callback, token, session, and SSO APIs. Macaca should normalize
  protocol capability discovery and callback verification without provider-name
  routing in OS-layer logic.

## Macaca-Owned Abstractions

`pack.identity.auth.handoff.v1` should define `AuthHandoffScope`,
`AuthHandoffProviderCapability`, `AuthProtocolProfile`, `AuthHandoffPlan`,
`AuthHandoffRecord`, `HandoffCorrelation`, `CallbackVerificationResult`,
`TokenReference`, `AssertionReference`, `SubjectEvidence`,
`SessionBindingEvidence`, `AuthHandoffAuditReference`,
`AuthHandoffAuditExportPlan`, `AuthHandoffArtifactHandle`,
`AuthHandoffFreshness`, `AuthHandoffAttribution`, and
`AuthHandoffRedactionPolicy`.

The DTOs must carry redirect/callback descriptors, protocol profile, scope
hints, state/nonce/PKCE/RelayState/WebAuthn challenge hashes, issuer, audience,
signature verification evidence, subject reference, token/assertion class,
expiry, storage boundary, session reference, replay pointer, redaction class,
and bounded failure reason. Raw authorization codes, access tokens, refresh
tokens, ID tokens, SAML assertions, WebAuthn assertion bodies, PKCE verifiers,
client secrets, session cookies, raw callback payloads, and raw provider
responses are rejected.

## Explicit Non-Goals

- Do not implement concrete Auth0, Okta, WorkOS, Clerk, Microsoft Entra, SAML,
  WebAuthn, browser, device, credential-vault, or session-store adapters in this
  research phase.
- Do not perform account lifecycle, profile updates, organization membership
  changes, tenant policy changes, session-store implementation, credential
  vaulting, password verification, MFA policy decisions, risk scoring, or
  application-specific login UI behavior.
- Do not expose raw protocol payloads, tokens, assertions, cookies, provider
  responses, or IdP-specific workflow state as stable SDK contracts.

## Existing Macaca Platform Inventory

- `crates/foundation/macaca-proto/src/domain_pack_contract/` provides descriptor,
  lifecycle, policy, diagnostics, SDK metadata, provider snapshot, unavailable,
  and effective capability primitives reusable by this pack.
- `crates/facade/macaca-sdk/src/system_facade.rs` provides the Facade pattern;
  auth handoff SDK helpers should only build canonical traced service calls.
- Generic policy, approval, resource, entitlement, trace, audit, artifact,
  mock-provider, unavailable-provider, session reference, and callback adapter
  concepts are reusable, but current evidence does not prove auth-handoff
  specific DTOs, descriptors, providers, SDK helpers, ABI metadata, tests, or
  developer docs.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
