## ADDED Requirements

### Requirement: Macaca SHALL provide Identity Auth Handoff as a serviceized pack

Macaca SHALL provide `pack.identity.auth.handoff.v1` as a provider-neutral,
serviceized identity pack for authentication handoff planning, start requests,
callback verification, token/assertion references, subject evidence inspection,
session binding evidence, cancellation, expiry cleanup, audit export, and
artifact handles.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.identity.auth.handoff.v1` as required and the auth handoff service provider is registered, healthy, entitled, permissioned, and policy-admissible
- **THEN** admission SHALL expose the pack in the effective capability set with descriptor hash, command schemas, permission scopes, policy template, provider capability, protocol support, callback support, token-reference support, session-bind support, health, freshness, and replay metadata
- **AND** SDK discovery SHALL mark supported commands as callable without exposing raw authorization codes, raw tokens, ID tokens, SAML assertions, WebAuthn assertion bodies, PKCE verifiers, client secrets, session cookies, raw callback payloads, raw provider responses, private keys, or signatures

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.identity.auth.handoff.v1` as required but provider, permission, entitlement, policy, resource, host support, protocol support, callback binding, token-reference support, session-bind support, redirect allowlist, or tenant/provider scope access is absent
- **THEN** admission SHALL block readiness with structured unavailable or denied diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back, call another provider, or fake success

#### Scenario: Optional declaration is unavailable
- **WHEN** an application declares `pack.identity.auth.handoff.v1` as optional and the pack is unavailable
- **THEN** admission SHALL produce an explicit degraded effective capability report
- **AND** SDK helpers SHALL refuse to build callable service calls for unavailable commands

### Requirement: Identity Auth Handoff SHALL expose provider and protocol discovery

`pack.identity.auth.handoff.v1` SHALL expose provider-neutral discovery for
protocol profiles, PKCE/nonce/state support, SAML support, WebAuthn/passkey
support, device/magic-link support, callback bindings, token-reference support,
subject evidence support, session-bind support, expiry, freshness, limits,
attribution, entitlement, and unavailable limitations.

#### Scenario: Provider schema is inspected
- **WHEN** an application invokes `auth_handoff.inspect_provider` or `auth_handoff.describe_schema`
- **THEN** Macaca SHALL return `AuthHandoffProviderCapability` and schema metadata with command support, protocol support, redirect/callback descriptors, correlation requirements, token-reference support, subject-evidence support, session-bind support, expiry, freshness, attribution, and limits
- **AND** the response SHALL use provider-neutral metadata rather than raw token, assertion, callback, session, credential, or provider payloads

#### Scenario: Protocol is unsupported
- **WHEN** a provider supports OIDC but not SAML or WebAuthn/passkey handoff
- **THEN** SDK discovery SHALL mark unsupported protocol profiles as non-callable for the effective capability
- **AND** invoking an unsupported protocol SHALL return typed `unsupported` before provider side effects

### Requirement: Identity Auth Handoff commands SHALL use typed canonical service calls

Every Identity Auth Handoff operation SHALL be represented as a typed command
and result DTO, and every invocation SHALL traverse the canonical service
runtime path with trace, policy, resource, entitlement, approval when required,
health, snapshot, replay protection, redaction, and structured error behavior.

#### Scenario: Handoff start succeeds through service runtime
- **WHEN** a declared and policy-allowed command such as `auth_handoff.start_handoff` is invoked with valid protocol profile, redirect descriptor, scopes, correlation data, expiry, and idempotency key
- **THEN** Macaca SHALL route the command through SDK or facade helpers into the service runtime and auth handoff service provider
- **AND** it SHALL emit sanitized admission, policy, provider-inspection, handoff-planning, service-call, result, audit, and replay events with stable trace identifiers

#### Scenario: Callback replay is denied before provider call
- **WHEN** callback verification sees stale state, reused nonce, reused RelayState, reused WebAuthn challenge, invalid redirect URI, invalid issuer/audience/signature, or expired handoff state
- **THEN** Macaca SHALL return typed `replay_rejected`, `denied`, `conflict`, or `stale_data` before invoking downstream side effects
- **AND** audit evidence SHALL include bounded reason codes and replay pointers without raw callback payloads, tokens, assertions, or provider responses

#### Scenario: Token exchange output is bounded
- **WHEN** `auth_handoff.exchange_token_reference` succeeds
- **THEN** Macaca SHALL return opaque `TokenReference` or `AssertionReference` handles with expiry, scope/claim hints, storage boundary, and redaction metadata
- **AND** logs, traces, snapshots, and SDK diagnostics SHALL NOT contain raw authorization codes, access tokens, refresh tokens, ID tokens, SAML assertions, WebAuthn assertion bodies, client secrets, or session cookies

### Requirement: Identity Auth Handoff SHALL normalize handoff state and subject evidence

Identity Auth Handoff SHALL provide normalized DTOs for handoff records,
protocol profiles, correlation metadata, callback verification, token/assertion
references, subject evidence, session binding evidence, audit references,
freshness, attribution, and redaction.

#### Scenario: Callback is verified
- **WHEN** an application or callback adapter invokes `auth_handoff.verify_callback` with authorized callback scope
- **THEN** Macaca SHALL validate correlation metadata, protocol conditions, issuer/audience/signature where applicable, expiry, replay status, and subject evidence references
- **AND** provider-specific missing fields SHALL be represented as explicit unavailable or unknown states rather than fabricated values

#### Scenario: Subject evidence is inspected
- **WHEN** an application invokes `auth_handoff.inspect_subject_evidence`
- **THEN** Macaca SHALL return minimized subject evidence with provider subject, account reference, profile claim references, assurance level, authentication context, organization/tenant hints, freshness, attribution, and redaction metadata
- **AND** the command SHALL NOT create accounts, update profiles, change organization membership, or bind sessions

#### Scenario: Session binding is planned
- **WHEN** an application invokes `auth_handoff.plan_session_binding`
- **THEN** Macaca SHALL validate session reference, subject evidence reference, freshness, approval requirements, policy, and session boundary support
- **AND** planning SHALL NOT mutate the session store

### Requirement: Identity Auth Handoff SHALL separate planning from side effects

Identity Auth Handoff SHALL provide plan-before-side-effect commands for handoff
start, token-reference exchange, session binding, cancellation, expiry cleanup,
and audit export so applications can inspect protocol constraints, correlation
requirements, approval requirements, idempotency needs, replay state, and
redaction bounds before external state changes.

#### Scenario: Handoff is planned
- **WHEN** an application invokes `auth_handoff.plan_handoff`
- **THEN** Macaca SHALL validate protocol profile, redirect allowlist, callback descriptor, scopes, provider hints, state/nonce/PKCE/RelayState/challenge requirements, expiry, idempotency requirement, and approval requirement
- **AND** the planning command SHALL NOT create a provider handoff or redirect URL

#### Scenario: Session binding is applied
- **WHEN** an application invokes `auth_handoff.bind_session` with verified subject evidence, approved plan, valid idempotency key, and supported session boundary
- **THEN** Macaca SHALL call the auth handoff/session facade through the service runtime and return `SessionBindingEvidence`
- **AND** stale subject evidence, missing approval, expired handoff, or session conflict SHALL return typed errors before side effects when detectable

#### Scenario: Handoff is cancelled or expired
- **WHEN** an application invokes `auth_handoff.cancel_handoff` or `auth_handoff.expire_handoff`
- **THEN** Macaca SHALL close the pending handoff state and release bounded resources through the service runtime
- **AND** cancellation or expiry SHALL NOT delete accounts, profiles, organizations, tenants, or sessions

### Requirement: Identity Auth Handoff SHALL preserve account, profile, organization, tenant, session, secrets, browser, device, and application-login boundaries

Identity Auth Handoff SHALL expose references to account, profile,
organization, tenant, session, secrets, browser, device, and host data when
providers include them, but it SHALL NOT execute or own those adjacent
capabilities.

#### Scenario: Account or profile mutation is requested through auth handoff pack
- **WHEN** an application attempts to create accounts, change account lifecycle, update profiles, manage avatars, or write profile preferences through the auth handoff pack
- **THEN** Macaca SHALL return `unsupported` or require separately declared account or profile capabilities
- **AND** auth handoff commands SHALL only carry sanitized account/profile references when available

#### Scenario: Secret or session-store operation is requested through auth handoff pack
- **WHEN** an application attempts to store raw tokens, client secrets, session cookies, password material, or implement a session store through the auth handoff pack
- **THEN** Macaca SHALL return `unsupported` or require separately declared secrets/session capabilities
- **AND** auth handoff traces SHALL record no raw credential, token, cookie, or secret payload

#### Scenario: Application login UI or browser automation is requested
- **WHEN** an application attempts to hardcode login UI behavior, browser automation, device prompt policy, risk scoring, or provider routing into the auth handoff pack
- **THEN** Macaca SHALL reject the behavior as application-owned, provider-owned, browser/device-owned, or risk-service-owned logic
- **AND** auth handoff commands SHALL only return provider-neutral handoff handles, callback descriptors, and subject/session evidence references

### Requirement: Identity Auth Handoff SHALL preserve Macaca boundaries

The Identity Auth Handoff implementation SHALL remain owned by the auth handoff
service provider family. The microkernel, SDK, shells, and generic application
framework SHALL remain provider-neutral and SHALL NOT contain concrete provider
construction, provider-name routing, account/profile mutation logic, token
storage, credential storage, session-store implementation, browser automation,
device prompt policy, risk scoring, or application-specific login UI behavior.

#### Scenario: Boundary gates scan the implementation
- **WHEN** dependency, no-direct-provider-call, and canonical execution-path gates scan the implementation
- **THEN** they SHALL find no concrete auth handoff provider imports in the microkernel, SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned service registrations and typed service commands

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, mock, or unavailable auth handoff provider is selected
- **THEN** callers SHALL observe the same provider-neutral command/result contract
- **AND** trace/audit evidence SHALL identify only sanitized provider class, descriptor metadata, protocol support, callback support, token-reference support, session-bind support, freshness, replay metadata, and bounded result codes

### Requirement: Identity Auth Handoff SHALL provide detailed developer documentation

The Identity Auth Handoff proposal SHALL require a detailed developer guide for
`pack.identity.auth.handoff.v1` that makes the pack usable by application
developers and provider implementers without relying on source-code inspection.

#### Scenario: Developer reads the pack guide
- **WHEN** a developer opens `docs/developer-packs/identity/auth-handoff.md`
- **THEN** the guide SHALL describe purpose, manifest declaration, permission scopes, provider/protocol discovery, command DTOs, result DTOs, examples, unavailable diagnostics, provider replacement, trace/audit behavior, redaction guarantees, redirect allowlists, state/nonce/PKCE/RelayState/challenge handling, replay protection, expiry, token-reference boundaries, session binding, and account/profile/organization/tenant/session/secrets/browser/device/application-login boundaries
- **AND** examples SHALL use generic handles and synthetic callback/token references instead of raw authorization codes, tokens, assertions, client secrets, provider routing keys, callback payloads, cookies, or application-specific login workflows

#### Scenario: SDK discovery links documentation
- **WHEN** SDK discovery returns metadata for `pack.identity.auth.handoff.v1`
- **THEN** the metadata SHALL include the auth-handoff developer-guide link and version compatibility information
- **AND** unavailable diagnostics SHALL point developers to the relevant declaration, permission, entitlement, provider, protocol, redirect, callback, token-reference, replay, session-bind, freshness, or boundary remediation section
