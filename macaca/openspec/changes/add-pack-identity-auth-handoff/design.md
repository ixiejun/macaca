# Identity Auth Handoff Pack Design

## Context

`pack.identity.auth.handoff.v1` is Macaca's provider-neutral authentication
handoff capability. It owns handoff planning, start requests, callback
verification, token/assertion references, subject evidence inspection, session
binding evidence, cancellation, expiry cleanup, and audit export. It does not
own account lifecycle, profile data, organization membership, tenant policy,
credential secrets, session storage internals, or application-specific login UI.

Authentication handoff is a state machine with strong replay and redaction
requirements. Providers differ by protocol and hosted implementation, but the
same boundary applies: Macaca stores correlation and evidence references, not
raw tokens or provider payloads.

## Supplier Capability Matrix

| Supplier family | Relevant capabilities | Constraints Macaca must model |
| --- | --- | --- |
| OAuth 2.0 + PKCE | Authorization request, redirect, state, PKCE challenge/verifier, authorization code, token exchange | Redirect allowlists, state replay protection, raw code/verifier redaction, token references only |
| OpenID Connect | ID Token, nonce, issuer/audience validation, discovery, UserInfo, claims | Nonce and signature validation, claim minimization, token payload redaction |
| SAML Web SSO | AuthnRequest, Response, Assertion, RelayState, signature, audience, conditions | XML/assertion validation, RelayState replay protection, assertion references only |
| WebAuthn/passkeys | Challenge, credential assertion, authenticator data, RP ID, user verification | Challenge binding, authenticator evidence, raw assertion redaction, device/host boundary |
| Auth0/Okta/Entra/WorkOS/Clerk | Hosted auth URLs, callbacks, token endpoints, sessions, organizations, connections, logs | Provider hints are descriptors, not routing branches; session/account/profile are adjacent references |

## Goals

- Provide provider inspection, schema discovery, handoff planning, handoff start,
  callback verification, token-reference exchange, subject evidence inspection,
  session binding plan/request, cancellation, expiry cleanup, audit export, and
  artifact retrieval.
- Preserve state/nonce/RelayState/challenge correlation, PKCE semantics,
  redirect allowlist validation, issuer/audience/signature checks, token
  reference boundaries, callback replay protection, freshness, and audit
  evidence.
- Keep accounts, profiles, organizations, tenants, sessions, secrets, MFA policy,
  risk scoring, and application login UI as separate capability boundaries.
- Route every command through canonical service runtime with trace, policy,
  entitlement, resource, approval when required, health, snapshot, and
  structured errors.

## Non-Goals

- Account creation, account lifecycle, profile update, organization membership,
  tenant policy, session store implementation, credential vaulting, password
  verification, MFA policy engine, IdP implementation, risk scoring, or
  application-specific login UI/workflow.
- Provider-specific social login preference, connection routing, fraud
  decisioning, or user authorization policy in OS layers.
- Raw authorization codes, access tokens, refresh tokens, ID tokens, SAML
  assertions, WebAuthn assertion bodies, PKCE verifiers, client secrets, session
  cookies, raw callback payloads, or raw provider responses in observability.

## Ownership And Boundaries

- Pack id: `pack.identity.auth.handoff.v1`.
- Family: `identity`.
- Backing service owner: auth handoff service provider family.
- SDK surface: `sdk.packs.identity.auth.handoff`.
- Command namespace: `auth_handoff.*`.
- Kernel ownership: identity, service-call evidence, policy facade, trace/audit
  primitives, and resource primitives only.
- Application framework ownership: manifest declaration, app-scoped permission
  declarations, lifecycle projection, and effective-capability mementos.
- Runtime-host ownership: provider registration, decorators, mock/unavailable
  providers, callback entry adapters, and adapter composition through approved
  composition roots.
- Service ownership: protocol capability discovery, handoff state machine,
  correlation validation, provider Strategy dispatch, token-reference handling,
  redaction, and sanitized audit.

## Command Surface

| Command | Purpose | Side-effect class |
| --- | --- | --- |
| `auth_handoff.inspect_provider` | Return protocol, callback, token, subject, session-bind, expiry, freshness, and attribution support | Read-only |
| `auth_handoff.describe_schema` | Return handoff, protocol, correlation, callback, token reference, subject evidence, session bind, and artifact schema | Read-only |
| `auth_handoff.plan_handoff` | Validate protocol, redirect/callback, scopes, hints, state, PKCE/nonce/challenge, and provider constraints | Planning |
| `auth_handoff.start_handoff` | Create pending handoff and return redirect/device/passkey/magic-link start handle | Mutating |
| `auth_handoff.verify_callback` | Verify callback payload, state/nonce/RelayState/challenge, signature, issuer, audience, and freshness | Mutating verification |
| `auth_handoff.exchange_token_reference` | Exchange provider code/assertion for bounded token/assertion reference handles | Mutating secret-adjacent |
| `auth_handoff.inspect_subject_evidence` | Return minimized authenticated subject and claim references | Read-only |
| `auth_handoff.plan_session_binding` | Validate session binding target, subject evidence, approval, and freshness | Planning |
| `auth_handoff.bind_session` | Bind verified subject evidence to Macaca session through approved session boundary | Mutating reference |
| `auth_handoff.cancel_handoff` | Cancel a pending handoff without provider success | Mutating |
| `auth_handoff.expire_handoff` | Expire pending handoff and release bounded resources | Mutating cleanup |
| `auth_handoff.plan_audit_export` | Plan auth handoff audit export scope, format, redaction, and retention | Planning |
| `auth_handoff.audit_export_request` | Produce auth handoff audit artifact handle | Mutating/export |
| `auth_handoff.get_artifact_handle` | Retrieve artifact metadata without raw callback/token leakage | Read-only |

Every command must define typed command DTOs, success DTOs, partial/async shapes,
denied/unavailable/unsupported/conflict/quota/stale-data/failure results,
idempotency for side effects, redaction policy, and replay metadata.

## Provider-Neutral DTO Model

- `AuthHandoffScope`: application, tenant, session, task, provider scope,
  handoff handle, callback handle, subject reference, and permission scope.
- `AuthHandoffProviderCapability`: protocol support, PKCE/nonce/state support,
  SAML support, WebAuthn/passkey support, device/magic link support, callback
  bindings, token-reference support, session-bind support, expiry, freshness,
  limits, attribution, and entitlement.
- `AuthProtocolProfile`: oauth2_authorization_code, oidc_authorization_code,
  saml_web_sso, webauthn_assertion, passkey, device_code, magic_link, and custom
  provider profiles with support metadata.
- `AuthHandoffPlan` and `AuthHandoffRecord`: protocol, redirect/callback
  descriptor, scopes, hints, correlation hashes, expiry, state, provider class,
  freshness, and redaction class.
- `HandoffCorrelation`: state hash, nonce hash, PKCE challenge hash, RelayState
  hash, WebAuthn challenge hash, device/user code reference, CSRF binding, and
  replay pointer.
- `CallbackVerificationResult`: verified state, issuer, audience, signature,
  nonce, challenge, redirect URI, subject reference, claim references, failure
  reason, freshness, and replay metadata.
- `TokenReference` and `AssertionReference`: opaque handle, token/assertion
  class, expiry, scope/claim hints, storage boundary, refreshability, redaction,
  and access policy without raw token values.
- `SubjectEvidence`: provider subject, account reference, profile claim
  references, assurance level, authentication context, organization/tenant
  hints, freshness, and redaction.
- `SessionBindingEvidence`: Macaca session reference, subject evidence reference,
  approval reference, expiry, binding state, and replay pointer.
- `AuthHandoffAuditReference` and `AuthHandoffArtifactHandle`: event type,
  bounded reason code, checksum, expiry, retention, redaction, and replay
  pointer.

## Permission, Policy, Resource, Entitlement, And Approval

Initial scopes:

- `identity.auth.handoff.start`
- `identity.auth.handoff.callback`
- `identity.auth.handoff.token_reference`
- `identity.auth.handoff.subject`
- `identity.auth.handoff.session_bind`
- `identity.auth.handoff.audit_export`

Policy defaults:

- Scope every call to application id, tenant id, session id, task id, trace id,
  provider scope, handoff handle, callback handle, redirect/callback descriptor,
  and subject reference.
- Require approval for session binding, elevated scopes, retained audit exports,
  device/passkey host capabilities, and any flow crossing sensitive identity or
  external browser boundaries.
- Require idempotency or replay keys for start, callback verification,
  token-reference exchange, session binding, cancellation, and export requests.
- Validate redirect allowlists, state/nonce/PKCE/RelayState/challenge
  correlation, callback freshness, issuer/audience/signature conditions,
  subject/session binding, token reference boundaries, and expiry before
  provider calls when detectable.
- Return typed `denied`, `unavailable`, `unsupported`, `conflict`,
  `quota_exceeded`, `stale_data`, or `failure` before provider calls when
  preconditions fail.
- Enforce resource budgets for pending handoffs, callback attempts, token
  exchange, subject inspection, audit export size, provider quotas, storage, and
  snapshots.

## Trace, Audit, Health, Snapshot, And Replay

Required event families:

- `auth_handoff_pack_declared`
- `auth_handoff_pack_admission_validated`
- `auth_handoff_pack_policy_decision`
- `auth_handoff_pack_provider_inspected`
- `auth_handoff_pack_service_call_requested`
- `auth_handoff_pack_service_call_succeeded`
- `auth_handoff_pack_service_call_failed`
- `auth_handoff_pack_handoff_planned`
- `auth_handoff_pack_callback_verified`
- `auth_handoff_pack_token_reference_exchanged`
- `auth_handoff_pack_session_binding_planned`
- `auth_handoff_pack_unavailable`
- `auth_handoff_pack_snapshot_recorded`

Events include pack id, command name, trace id, application/session/task/tenant
identifiers, handoff/callback handles, protocol profile, correlation hash,
subject reference, session reference, policy decision, provider class,
descriptor hash, latency, freshness, idempotency hash, bounded resource
counters, result code, and sanitized artifact references. Events must exclude
raw authorization codes, raw tokens, ID tokens, SAML assertions, WebAuthn
assertion bodies, PKCE verifiers, client secrets, session cookies, raw callback
payloads, raw provider responses, private keys, and signatures.

Snapshots include descriptor version, provider health, command availability,
protocol/callback/token/session-bind support, policy-template hash, redaction
profile, pending handoff counters, freshness, resource counters, and replay
pointers.

## SDK And Developer Documentation

SDK discovery must return pack metadata, lifecycle, service mapping, command
schemas, permission scopes, policy templates, examples, availability, health,
provider class, compatibility, diagnostics, and documentation links.

The required developer guide at
`docs/developer-packs/identity/auth-handoff.md` must cover:

- Manifest declaration and permission scopes.
- Provider/protocol/schema discovery and unavailable diagnostics.
- DTO reference for scopes, provider capability, protocol profiles, handoff
  plans, correlation metadata, callback verification, token/assertion
  references, subject evidence, session binding evidence, audit references,
  freshness, redaction, and artifacts.
- Examples for planning/starting handoffs, handling callbacks, exchanging token
  references, inspecting subject evidence, planning/binding sessions, cancelling
  pending handoffs, exporting audit evidence, and handling conflicts/replay.
- Provider replacement, mock/unavailable provider behavior, trace/audit
  interpretation, redaction guarantees, idempotency, redirect allowlists, replay
  protection, expiry, and boundaries with account, profile, organization,
  tenant, session, secrets, browser automation, device, and application login UI.

## Design Patterns

- **Facade**: SDK exposes discovery and command builders while hiding providers.
- **Command**: every auth handoff operation is a typed command/result DTO.
- **Strategy**: OAuth/OIDC-like, SAML-like, WebAuthn-like, Auth0-like,
  Okta-like, WorkOS-like, Clerk-like, Entra-like, and other providers adapt
  behind one contract.
- **Decorator**: trace, policy, entitlement, approval, resource, idempotency,
  replay protection, expiry, redirect allowlist, and redaction wrap every call.
- **State**: handoff pending, callback verified, token reference exchanged,
  subject inspected, session binding planned/bound, cancelled, expired, and
  provider-health states are explicit.
- **Specification**: admission validates declarations, scopes, protocol support,
  redirect/callback descriptors, correlation, expiry, session binding, and
  resource limits.
- **Observer**: trace, audit, provider, callback, token-reference, session-bind,
  and snapshot events are subscribable.
- **Memento**: effective capability reports, handoff records, callback evidence,
  token references, session binding evidence, audit references, and artifact
  handles are replayable bounded records.
- **Abstract Factory**: providers register only through approved runtime-host or
  plugin composition roots.

## Risks And Mitigations

- Risk: raw tokens or callback payloads leak. Mitigation: DTOs carry opaque
  handles and hashes only; redaction tests cover logs, traces, snapshots, and
  SDK diagnostics.
- Risk: handoff becomes account/session implementation. Mitigation: account,
  profile, tenant, organization, session, and secrets surfaces are adjacent
  capabilities; this pack returns evidence and references only.
- Risk: replay or redirect attacks. Mitigation: state/nonce/PKCE/RelayState or
  challenge correlation, redirect allowlists, expiry, idempotency, and replay
  tests are mandatory.
- Risk: provider-specific login UI/routing enters OS layers. Mitigation:
  provider hints live in descriptors and command DTOs, not provider-name
  branches.
- Risk: SDK helpers become a second execution path. Mitigation: helpers only
  build canonical service-call commands and no-direct-provider-call gates cover
  every command.
