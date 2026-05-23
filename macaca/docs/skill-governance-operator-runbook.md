# Skill Governance Operator Runbook

This runbook documents how operators inspect and steer Macaca Skill
governance without taking semantic ownership away from the Skill service.  The
commands below are generic service-command examples: presentation shells may
render them, but only `service.skill` owns lifecycle, curation, alias, mutation,
and replay decisions.

## Ownership Model

- Skill service owns evolution, lifecycle, curation, alias resolution, safe
  mutation contracts, and sanitized governance snapshots.
- Store/EventLog providers own durable append-only records and replay.
- Policy and Entitlement services own approval, package ownership, paid, and
  encrypted mutation decisions.
- Context, Task, Scheduler, Web, CLI, and frontend code are adapters. They read
  snapshots or submit typed commands and must not infer curation semantics.
- Kernel code remains outside skill curation, semantic review, package
  mutation, and marketplace policy.

## Governance Store Records

The Skill Governance Store is an append-only event source.  Operators should
reason from records and refs rather than raw skill bodies or provider payloads.

| Record class | Purpose | Operator use |
| --- | --- | --- |
| Governance record | Current lifecycle, source scope, trust, telemetry, provenance, and bounded diagnostics for one skill. | Inspect whether a skill is visible, stale, pinned, protected, or blocked. |
| Alias record | Redirect, warn-and-redirect, deny, or miss metadata from an old skill id/name to an effective skill id/name. | Preserve old references after supersede or merge without rewriting consumer state. |
| Proposal record | Draft, promoted, rejected, or patch proposal summary with evidence refs. | Audit why a skill change was proposed and whether it was approved. |
| Curation run record | Run id, dry-run/apply mode, phase summary, candidate counts, report refs, policy refs, and audit ids. | Prove what a curation run evaluated and what it changed. |
| Snapshot ref record | Durable pre/post governance snapshot reference. | Compare state around a curation run or mutation without embedding raw payloads. |
| Rollback ref record | Memento reference that can restore lifecycle, telemetry, alias, report, and package refs. | Rewind approved changes through the service-owned rollback command. |

Replay rebuilds the read model by folding these events in order.  Replay output
must remain sanitized: raw prompts, provider payloads, package bytes, manifests,
credentials, raw signatures, and full skill bodies are not operator-facing
state.

## Lifecycle Rules

Skill lifecycle states are `Draft`, `Active`, `Stale`, `Archived`,
`Quarantined`, `Superseded`, and `Rejected`.

- `Draft -> Active` requires promotion through the Skill service and writes
  provenance, telemetry, and audit refs.
- `Draft -> Rejected` requires explicit rejection evidence.
- `Active -> Stale`, `Archived`, `Quarantined`, or `Superseded` is a curation
  or lifecycle command decision and must pass ownership policy.
- `Superseded` requires an alias or redirect record so old references have an
  auditable route.
- `Archived -> Active` and quarantine release require restore/release commands
  with policy checks and audit refs.
- Pinned or protected skills can be reported but cannot be automatically
  mutated by curation.

## Curation Flow

Curation is a service-owned command sequence, not a shell workflow.

1. `skill.curation.status` returns interval, idle/running state, last run,
   provider availability, and bounded diagnostics.
2. `skill.curation.run` evaluates deterministic phases first: lifecycle
   health, telemetry, stale signals, duplicate or merge candidates, package
   ownership, approval state, and rollback eligibility.
3. Optional semantic review runs as a replaceable Strategy.  If no provider is
   installed, the run records semantic review as `Unavailable` and still
   completes deterministic curation.
4. Dry-run mode writes reports but does not mutate active governance, aliases,
   files, or package refs.
5. Apply mode writes rollback mementos before side effects, then records
   lifecycle, alias, report, policy, and audit refs.
6. `skill.curation.rollback` restores from a rollback ref through the Skill
   service; operators should not edit governance files directly.

Reports and route responses expose ids, counts, lifecycle decisions, policy
decisions, rollback refs, and audit ids.  They do not echo raw prompts,
provider payloads, manifests, package bytes, credentials, signatures, or full
skill bodies.

## Ownership Policy

Approval expectations are based on package ownership class and operation type.

| Ownership class | Automatic active mutation | Alias/metadata | Required operator posture |
| --- | --- | --- | --- |
| Agent-private | Allowed when policy admits the operation. | Allowed. | Verify audit refs and rollback refs. |
| Central-user | Requires explicit approval for active lifecycle or content mutation. | Allowed. | Approve only with bounded evidence and rollback eligibility. |
| Tenant | Requires explicit approval for active lifecycle or content mutation. | Allowed. | Confirm tenant scope and audit refs. |
| Application-owned | Requires application-scope policy approval. | Allowed. | Keep decisions application-scoped without hardcoding application behavior. |
| Marketplace | Requires a local overlay draft for active mutation. | Allowed. | Do not patch marketplace package bytes directly. |
| Bundled | Denied for active package/lifecycle mutation. | Allowed. | Treat as protected OS/package material. |
| Plugin-provided | Denied for active package/lifecycle mutation. | Allowed. | Let the provider/plugin lifecycle own package updates. |
| Paid | Requires mutation entitlement for active mutation. | Allowed. | Confirm entitlement before approving mutation. |
| Encrypted | Requires mutation entitlement for active mutation. | Allowed. | Never expose decrypted content in reports or logs. |

## Context Composer And Alias Visibility

Context Composer consumes Skill service snapshots only.  It should include
skills that are active and policy-visible for the current scope, and it should
filter archived, quarantined, rejected, and superseded skills unless an alias
resolution returns an effective visible skill.

Alias decisions are audit facts:

- `Miss` means no alias was found and the requested id/name remains unchanged.
- `Redirected` changes the effective id/name.
- `WarnAndRedirected` changes the effective id/name and carries bounded warning
  metadata.
- `Denied` blocks resolution and must not silently fall back to a protected
  skill.
- Loop, expired, or denied aliases are logged as bounded decision facts.

Task, Scheduler, Autonomy, and Context callers should pass requested skill
identity metadata to the Skill service and use the returned effective identity.
They should not implement their own alias maps.

## Generic Operation Examples

Read curation status:

```json
{
  "service_id": "service.skill",
  "command_name": "skill.curation.status",
  "payload": {
    "scope": {
      "application_id": null,
      "session_id": null,
      "tenant_id": "tenant.generic",
      "agent_name": null
    }
  }
}
```

Run deterministic dry-run curation:

```json
{
  "service_id": "service.skill",
  "command_name": "skill.curation.run",
  "payload": {
    "scope": {
      "application_id": null,
      "session_id": null,
      "tenant_id": "tenant.generic",
      "agent_name": null
    },
    "dry_run": true,
    "requested_by": "operator.generic"
  }
}
```

Resolve an alias before a context or task read:

```json
{
  "service_id": "service.skill",
  "command_name": "skill.alias.resolve",
  "payload": {
    "requested_id": "skill.example.old",
    "requested_name": "Generic Example Skill",
    "scope": {
      "application_id": null,
      "session_id": null,
      "tenant_id": "tenant.generic",
      "agent_name": null
    }
  }
}
```

Rollback by memento ref:

```json
{
  "service_id": "service.skill",
  "command_name": "skill.curation.rollback",
  "payload": {
    "rollback_ref": "skill.rollback://example-ref",
    "requested_by": "operator.generic",
    "reason": "restore previous governed state after approved review"
  }
}
```

These examples intentionally use generic ids.  Do not place workflow names,
application names, provider names, raw prompts, secrets, provider payloads, or
full skill bodies in operator-visible command examples, logs, reports, or
runbooks.
