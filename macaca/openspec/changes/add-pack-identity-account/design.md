# Identity Account Pack Design

## Context

`pack.identity.account.v1` is Macaca's provider-neutral account management
capability. It owns account records, minimized identifiers, lifecycle state,
linked identity references, status synchronization, recovery references, and
account audit export. It does not own auth handoff, credential secret handling,
profile preferences, organization membership, tenant policy, or application
business workflow.

Identity APIs vary by provider. Okta has rich lifecycle transitions, Auth0 has
connection identities and block/verification states, Microsoft Graph and Google
Directory expose enterprise directory users, and SCIM defines a common user
resource model. Macaca normalizes the account slice and keeps provider-specific
schema extensions behind service provider Strategy adapters.

## Supplier Capability Matrix

| Supplier family | Relevant capabilities | Constraints Macaca must model |
| --- | --- | --- |
| Okta Users | User objects, profile/credentials, lifecycle activate/deactivate/suspend/unlock, provider links | Lifecycle transition support varies by state; credentials are sensitive and must become references only |
| Auth0 Management Users | Users, identities, connections, blocked state, email verification, metadata, logs, deletion | Account linking and metadata must be bounded; auth/session/token behavior is out of scope |
| Microsoft Graph Users | Entra ID users, accountEnabled, identities, user principal names, directory relationships, audit references | Enterprise directory schema, soft delete, identity relationships, license/group references |
| Google Admin Directory | Users, aliases, suspension, organizations, custom schemas, tokens, undelete | Workspace directory constraints, recovery data sensitivity, custom schema redaction |
| SCIM 2.0 | Users, externalId, active, emails, name, groups, metadata, filtering, PATCH, schema extension | Interoperability, pagination, patch semantics, active vs disabled mapping |
| WorkOS/Clerk-like platforms | User records, external identities, invitations, sessions, organization references, audit events | Developer-platform abstractions, org/session references, provider event freshness |

## Goals

- Provide provider inspection, schema discovery, account planning, account
  creation, read/search, update, lifecycle transition planning, lifecycle
  transition request, linked identity attach/detach, status sync, recovery
  reference management, audit export, and artifact retrieval.
- Preserve account identifier uniqueness, lifecycle state semantics, linked
  identity provenance, minimized account attributes, version tokens,
  idempotency, freshness, and audit evidence.
- Keep authentication handoff, credentials, tokens, MFA execution, profile
  preferences, organization membership, tenant policy, and application workflow
  as separate capability boundaries.
- Route every command through canonical service runtime with trace, policy,
  entitlement, resource, approval when required, health, snapshot, and
  structured errors.

## Non-Goals

- OAuth/OIDC/SAML handoff, token exchange, session binding, password storage,
  password verification, MFA challenge execution, raw credential recovery,
  identity document verification, profile preferences, organization membership,
  tenant isolation policy, or app-specific onboarding/offboarding workflows.
- Provider-specific HRIS provisioning flows, compliance workflow, user naming
  policy, or directory routing in OS layers.
- Raw credentials, password hashes, password reset tokens, recovery codes, MFA
  secrets, access/refresh tokens, raw provider payloads, identity documents, or
  unbounded audit exports in observability.

## Ownership And Boundaries

- Pack id: `pack.identity.account.v1`.
- Family: `identity`.
- Backing service owner: account service provider family.
- SDK surface: `sdk.packs.identity.account`.
- Command namespace: `account.*`.
- Kernel ownership: identity, service-call evidence, policy facade, trace/audit
  primitives, and resource primitives only.
- Application framework ownership: manifest declaration, app-scoped permission
  declarations, lifecycle projection, and effective-capability mementos.
- Runtime-host ownership: provider registration, decorators, mock/unavailable
  providers, and adapter composition through approved composition roots.
- Service ownership: capability discovery, account lifecycle validation,
  provider Strategy dispatch, account normalization, redaction, and sanitized
  audit.

## Command Surface

| Command | Purpose | Side-effect class |
| --- | --- | --- |
| `account.inspect_provider` | Return lifecycle, schema, identifier, search, linked-identity, audit, freshness, and attribution support | Read-only |
| `account.describe_schema` | Return account, identifier, lifecycle, linked identity, recovery, audit, and artifact schema | Read-only |
| `account.plan_create` | Validate identifiers, minimized attributes, source, idempotency, approval, and provider constraints | Planning |
| `account.create_account` | Create an account record through approved side-effect path | Mutating |
| `account.read_account` | Read one normalized account record | Read-only |
| `account.search_accounts` | Search accounts by authorized filters, cursor, identifier, state, or linked identity | Read-only |
| `account.plan_update` | Validate allowed account attributes, version token, and policy | Planning |
| `account.update_account` | Apply account metadata update without raw credential changes | Mutating |
| `account.plan_lifecycle_transition` | Validate activate/disable/suspend/unsuspend/lock/unlock/delete/archive/recover transition | Planning |
| `account.lifecycle_transition_request` | Apply approved account lifecycle transition | Mutating |
| `account.link_identity` | Attach external identity reference to account when provider supports it | Mutating |
| `account.unlink_identity` | Detach external identity reference with approval and conflict checks | Mutating |
| `account.sync_status` | Refresh lifecycle, identifier, linked identity, and freshness metadata | Read-only or provider sync |
| `account.set_recovery_reference` | Store bounded recovery reference metadata without secrets | Mutating metadata |
| `account.inspect_account_audit` | Read bounded account audit references | Read-only |
| `account.plan_audit_export` | Plan account audit export scope, format, redaction, and retention | Planning |
| `account.audit_export_request` | Produce account audit artifact handle | Mutating/export |
| `account.get_artifact_handle` | Retrieve artifact metadata without raw payload leakage | Read-only |

Every command must define typed command DTOs, success DTOs, partial/async shapes,
denied/unavailable/unsupported/conflict/quota/stale-data/failure results,
idempotency for side effects, redaction policy, and replay metadata.

## Provider-Neutral DTO Model

- `AccountScope`: application, tenant, session, task, provider scope, account
  handle, subject handle, identity provider reference, and permission scope.
- `AccountProviderCapability`: create/update/search support, lifecycle
  transitions, linked identity support, recovery reference support, audit export
  support, schema extension support, pagination, versioning, freshness, limits,
  attribution, and entitlement.
- `AccountRecord`: account handle, stable subject reference, identifiers,
  minimized attributes, lifecycle state, linked identities, organization/tenant
  references, recovery references, audit references, version token, freshness,
  and redaction class.
- `AccountIdentifier`: username, email, phone, user principal name, alias,
  external id, SCIM id, directory id, provider subject id, and verification
  state with redaction metadata.
- `AccountAttributePatch`: bounded mutable attributes such as display label,
  locale hint, contact reference, status note, or custom schema references;
  rich profile fields belong to the profile pack.
- `AccountLifecycleState`: planned, staged, provisioned, active, locked,
  suspended, disabled, archived, deprovisioned, deleted, recovered,
  password_expired_reference, unknown, and provider custom state mappings.
- `LinkedIdentityReference`: provider class, issuer/connection reference,
  external subject, assurance level, link state, freshness, and replay pointer.
- `AccountRecoveryReference`: recovery email/phone reference, reset-flow
  reference, support case reference, and redaction profile without raw tokens or
  secrets.
- `AccountAuditReference` and `AccountArtifactHandle`: event type, actor
  reference, timestamp, bounded reason code, checksum, expiry, retention,
  redaction, and replay pointer.

## Permission, Policy, Resource, Entitlement, And Approval

Initial scopes:

- `identity.account.read`
- `identity.account.create`
- `identity.account.update`
- `identity.account.lifecycle`
- `identity.account.link_identity`
- `identity.account.audit_export`

Policy defaults:

- Scope every call to application id, tenant id, session id, task id, trace id,
  provider scope, account handle, subject reference, and identity provider
  reference.
- Require approval for account creation, disabling, suspension, deletion,
  recovery, linked identity changes, recovery reference changes, and retained
  audit exports.
- Require idempotency keys for mutating commands and export requests.
- Validate tenant isolation, identifier uniqueness, version tokens, lifecycle
  transition legality, linked identity conflicts, recovery reference sensitivity,
  and freshness before provider calls when detectable.
- Return typed `denied`, `unavailable`, `unsupported`, `conflict`,
  `quota_exceeded`, `stale_data`, or `failure` before provider calls when
  preconditions fail.
- Enforce resource budgets for account search, status sync, audit export size,
  provider quotas, storage, and snapshots.

## Trace, Audit, Health, Snapshot, And Replay

Required event families:

- `account_pack_declared`
- `account_pack_admission_validated`
- `account_pack_policy_decision`
- `account_pack_provider_inspected`
- `account_pack_service_call_requested`
- `account_pack_service_call_succeeded`
- `account_pack_service_call_failed`
- `account_pack_create_planned`
- `account_pack_lifecycle_planned`
- `account_pack_identity_link_changed`
- `account_pack_unavailable`
- `account_pack_snapshot_recorded`

Events include pack id, command name, trace id, application/session/task/tenant
identifiers, account/subject handles, lifecycle transition, linked identity
class, policy decision, provider class, descriptor hash, latency, freshness,
version token hash, idempotency hash, bounded resource counters, result code,
and sanitized artifact references. Events must exclude raw credentials,
password hashes, reset tokens, recovery codes, MFA secrets, access tokens,
refresh tokens, raw provider payloads, identity documents, and unbounded audit
exports.

Snapshots include descriptor version, provider health, command availability,
schema/lifecycle/link/audit support, policy-template hash, redaction profile,
freshness, resource counters, and replay pointers.

## SDK And Developer Documentation

SDK discovery must return pack metadata, lifecycle, service mapping, command
schemas, permission scopes, policy templates, examples, availability, health,
provider class, compatibility, diagnostics, and documentation links.

The required developer guide at `docs/developer-packs/identity/account.md` must
cover:

- Manifest declaration and permission scopes.
- Provider/schema discovery and unavailable diagnostics.
- DTO reference for scopes, provider capability, account records, identifiers,
  minimized attributes, lifecycle states, linked identity references, recovery
  references, audit references, freshness, redaction, and artifacts.
- Examples for planning/creating accounts, reading/searching accounts, updating
  account metadata, lifecycle transitions, linking/unlinking identities, syncing
  status, setting recovery references, inspecting audit, exporting audit
  evidence, and handling conflicts.
- Provider replacement, mock/unavailable provider behavior, trace/audit
  interpretation, redaction guarantees, idempotency, version tokens, freshness,
  and boundaries with profile, auth handoff, organization, tenant, session,
  secrets, and application workflow.

## Design Patterns

- **Facade**: SDK exposes discovery and command builders while hiding providers.
- **Command**: every account operation is a typed command/result DTO.
- **Strategy**: Okta-like, Auth0-like, Graph-like, Google-like, SCIM-like,
  WorkOS-like, Clerk-like, and other providers adapt behind one contract.
- **Decorator**: trace, policy, entitlement, approval, resource, idempotency,
  metering, versioning, and redaction wrap every call.
- **State**: account lifecycle, linked identity, recovery reference, status sync,
  audit export, and provider health are explicit states.
- **Specification**: admission validates declarations, scopes, identifier
  uniqueness, tenant isolation, lifecycle transitions, linked identity
  conflicts, and resource limits.
- **Observer**: trace, audit, provider, lifecycle, linked identity, and snapshot
  events are subscribable.
- **Memento**: effective capability reports, lifecycle evidence, linked identity
  evidence, audit references, and artifact handles are replayable bounded
  records.
- **Abstract Factory**: providers register only through approved runtime-host or
  plugin composition roots.

## Risks And Mitigations

- Risk: account pack becomes auth/token or credential storage. Mitigation:
  auth handoff, sessions, tokens, secrets, passwords, and MFA challenges are
  separate capabilities; this pack accepts references only.
- Risk: account pack duplicates profile, organization, or tenant ownership.
  Mitigation: account records carry minimized attributes and references; rich
  profile, membership, roles, and tenant policy remain separate packs.
- Risk: sensitive identity data leaks into observability. Mitigation: redaction
  profiles, artifact handles, bounded reason codes, and redaction tests are
  mandatory.
- Risk: lifecycle semantics differ by provider. Mitigation: provider capability
  discovery, planning commands, state mapping metadata, conflict/stale-data
  errors, and replayable transition evidence are first-class.
- Risk: SDK helpers become a second execution path. Mitigation: helpers only
  build canonical service-call commands and no-direct-provider-call gates cover
  every command.
