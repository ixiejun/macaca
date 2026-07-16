# Identity Account Pack

`pack.identity.account.v1` is the provider-neutral account-management contract
for Macaca applications. It describes account records, minimized identifiers,
lifecycle state, linked identity references, recovery references, account audit
references, and bounded artifact handles. It does not own login handoff,
sessions, raw credentials, profile preferences, organization membership, tenant
policy, or application onboarding workflow.

## Manifest

Declare the pack as optional until an account provider is installed:

```toml
[service_contract]
optional_packs = ["pack.identity.account.v1"]
```

Use `required_packs` only when the application cannot run without account
management. A required declaration must fail admission while the descriptor is
`preview_unavailable`.

## Permission Scopes

- `identity.account.read`
- `identity.account.create`
- `identity.account.update`
- `identity.account.lifecycle`
- `identity.account.link_identity`
- `identity.account.audit_export`

Mutating commands require policy, idempotency, and approval when they create
accounts, change lifecycle state, link identities, change recovery references,
or retain audit exports.

## Commands

- `account.inspect_provider`
- `account.describe_schema`
- `account.plan_create`
- `account.create_account`
- `account.read_account`
- `account.search_accounts`
- `account.plan_update`
- `account.update_account`
- `account.plan_lifecycle_transition`
- `account.lifecycle_transition_request`
- `account.link_identity`
- `account.unlink_identity`
- `account.sync_status`
- `account.set_recovery_reference`
- `account.inspect_account_audit`
- `account.plan_audit_export`
- `account.audit_export_request`
- `account.get_artifact_handle`

Planning commands validate identifiers, lifecycle legality, version tokens,
conflict state, and approval requirements before provider side effects.

## DTO Model

Primary DTOs include `AccountScope`, `AccountProviderCapability`,
`AccountRecord`, `AccountIdentifier`, `AccountAttributePatch`,
`AccountLifecycleState`, `AccountLifecycleTransitionPlan`,
`LinkedIdentityReference`, `AccountRecoveryReference`,
`AccountAuditReference`, and `AccountArtifactHandle`.

Account identifiers are represented by refs and hashes. Raw passwords, password
hashes, reset tokens, recovery codes, MFA secrets, access tokens, refresh
tokens, raw provider payloads, identity documents, and unbounded audit exports
must never appear in logs, traces, snapshots, SDK diagnostics, or examples.

## Unavailable Behavior

The reference descriptor is discoverable but not callable until a serviceized
provider registers command schemas for `service.identity.account`. SDK discovery
returns `identity_account_provider_not_installed` for optional degradation and
required-pack admission failure.

## App-Facing Examples

- Plan account creation with minimized identifiers, idempotency, version, and
  approval refs before requesting provider side effects.
- Read or search accounts with bounded pages and redacted account handles.
- Plan metadata updates, lifecycle transitions, linked-identity changes, and
  recovery-reference changes before mutation commands.
- Sync account status, inspect audit references, and export audit evidence only
  through bounded artifact handles.
- Handle version conflicts, lifecycle-invalid states, link conflicts,
  unavailable providers, denied permissions, quota, stale data, and
  artifact-denied diagnostics as typed results.

## Provider Replacement

Provider classes are declarative: `account-directory`, `account-lifecycle`,
`linked-identity`, `mock`, and `unavailable`. Concrete identity providers are
bound only through approved runtime-host or plugin composition roots. SDK,
kernel, shells, and generic application framework code must not construct
providers or branch on provider names.

## Trace And Audit

Trace evidence records pack id, command, trace id, tenant/session/task refs,
account/subject handles, provider class, descriptor hash, idempotency hash,
version hash, bounded result code, and artifact handles. Sensitive identity
payloads are always redacted or represented by references.

## Boundaries

Use `pack.identity.auth.handoff.v1` for login, callback verification, and token
references. Use `pack.identity.profile.v1` for profile fields and preferences.
Use `pack.identity.organization.v1` for membership and roles. Use
`pack.identity.tenant.v1` for isolation policy and quota references. Use
`pack.foundation.secrets-reference.v1` for secret handles.
