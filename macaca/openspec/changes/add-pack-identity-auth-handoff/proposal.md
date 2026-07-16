# Change: Add Identity Auth Handoff Pack

## Why

Macaca applications need `pack.identity.auth.handoff.v1` as an industrial
capability for starting external authentication handoffs, validating callbacks,
exchanging codes for token references, binding authenticated subject evidence to
Macaca sessions, and cancelling or expiring pending handoffs. Authentication
handoff providers expose OAuth 2.0, OpenID Connect, SAML, WebAuthn/passkey,
magic-link, device-code, and hosted login flows with different state, nonce,
PKCE, assertion, token, redirect, and callback semantics. Macaca must normalize
the handoff boundary without storing raw secrets, becoming an IdP, or owning
account/profile/tenant/session business semantics.

This proposal defines auth handoff as a serviceized, provider-neutral pack. It
lets applications request and verify authentication flows through typed commands
while keeping concrete IdP adapters, browser/host entrypoints, token storage,
session binding, and unavailable behavior behind replaceable service providers.

## Supplier And API Baseline

The design is based on mature authentication and federation protocols:

- OAuth 2.0 Authorization Code with PKCE defines authorization requests,
  redirects, code verifier/challenge, state, token exchange, and client
  authentication constraints.
- OpenID Connect Core adds ID Token, nonce, UserInfo, discovery, issuer,
  audience, subject, claims, token validation, and session-related semantics.
- SAML 2.0 Web Browser SSO defines AuthnRequest, Response, Assertion,
  RelayState, issuer, audience, signature validation, and assertion conditions.
- WebAuthn/passkeys define credential creation/authentication ceremonies,
  challenges, authenticators, relying party identifiers, user verification, and
  attestation/assertion validation.
- Auth0, Okta, WorkOS, Clerk, Microsoft Entra ID, and similar identity providers
  expose hosted authorization URLs, callback handling, token endpoints,
  connection/organization hints, session/token references, and audit events.

Research references:

- OAuth 2.0: https://www.rfc-editor.org/rfc/rfc6749
- OAuth 2.0 PKCE: https://www.rfc-editor.org/rfc/rfc7636
- OpenID Connect Core:
  https://openid.net/specs/openid-connect-core-1_0.html
- SAML 2.0 Web Browser SSO Profile:
  https://docs.oasis-open.org/security/saml/v2.0/saml-profiles-2.0-os.pdf
- WebAuthn: https://www.w3.org/TR/webauthn-3/
- Auth0 Authentication API: https://auth0.com/docs/api/authentication
- Okta OAuth/OIDC API:
  https://developer.okta.com/docs/api/openapi/okta-oauth/oauth/overview/
- WorkOS SSO: https://workos.com/docs/sso
- Clerk sessions/sign-in: https://clerk.com/docs/references/backend/sessions

## Macaca Provider-Neutral Mapping

`pack.identity.auth.handoff.v1` maps protocol/provider concepts into stable
Macaca contracts:

- Authorization URLs, hosted login flows, device authorization, SAML redirect
  bindings, passkey challenges, and magic-link starts become `AuthHandoffPlan`
  and `AuthHandoffStartResult`.
- OAuth `state`, OIDC `nonce`, PKCE verifier/challenge, SAML `RelayState`, and
  WebAuthn challenge become `HandoffCorrelation` metadata with redacted hashes.
- Authorization codes, ID tokens, access tokens, refresh tokens, SAML
  assertions, WebAuthn assertions, session cookies, and provider secrets become
  `TokenReference` or `AssertionReference` handles; raw values are not exposed.
- Callback URL/query/form payloads, token endpoint responses, issuer/audience
  claims, signatures, and challenge results become `CallbackVerificationResult`.
- Authenticated subjects, account references, profile claim references,
  organization hints, tenant hints, and session bind targets become references
  for adjacent packs.
- Provider events and logs become `AuthHandoffAuditReference` and artifact
  handles with bounded metadata.

## What Changes

- Add provider-neutral `pack.identity.auth.handoff.v1` under the identity family.
- Define commands for provider inspection, schema discovery, handoff planning,
  handoff start, callback verification, token-reference exchange, subject
  evidence inspection, session binding plan, session bind request, cancellation,
  expiry cleanup, audit export, and artifact retrieval.
- Define DTOs for handoff scope, provider capability, protocol profile,
  redirect/callback descriptors, correlation state, token/assertion references,
  subject evidence, callback verification, session binding evidence, freshness,
  attribution, redaction, and artifact handles.
- Require policy, approval for high-risk session binding, state/nonce/PKCE or
  assertion validation, replay protection, redirect allowlist validation,
  idempotency, sanitized trace/audit, and deterministic unavailable/unsupported
  behavior.
- Require detailed developer documentation at
  `docs/developer-packs/identity/auth-handoff.md`.

## Impact

- Affected specs: `pack-identity-auth-handoff`,
  `developer-pack-industrial-capability-catalog`, `sdk-system-facade`,
  `service-runtime`, `unified-execution-path`.
- Affected code later: protocol DTOs, pack descriptors, admission validators,
  SDK discovery/command builders, auth handoff service providers,
  mock/unavailable providers, trace/audit schemas, replay tests, redaction
  tests, callback replay tests, and boundary gates.

## Non-Goals

- No account creation/lifecycle ownership, rich profile management,
  organization/tenant membership, session store implementation, credential
  vaulting, password verification, MFA policy engine, identity provider
  implementation, or application-specific login UI.
- No provider-specific routing, login business policy, social-login preference
  logic, risk scoring, fraud decisioning, or application authorization workflow
  in Macaca OS layers.
- No raw authorization codes, ID/access/refresh tokens, SAML assertions,
  WebAuthn assertion bodies, PKCE verifiers, client secrets, session cookies,
  private keys, signatures, raw callback payloads, or raw provider responses in
  logs, traces, snapshots, or SDK diagnostics.
- No provider construction or provider-name routing in kernel, SDK, shells, or
  generic application framework.
