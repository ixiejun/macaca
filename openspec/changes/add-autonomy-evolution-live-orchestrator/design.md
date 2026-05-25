## Context

`add-autonomy-evolution-control-plane`,
`add-evolution-admission-quality-gates`,
`add-normalized-evolution-benchmarking`,
`add-evolution-release-safety-chain`,
`serviceize-evolution-governance-ledger`, and
`add-os-code-evolution-proposal-adapter` created the service contracts and
gates needed for complete self-evolution. The remaining gap is runtime
composition: the platform must run these contracts as a single unattended,
resumable, auditable policy loop.

## Goals

- Add a thin live orchestrator service surface for evolution loop ticks and
  audit reconstruction.
- Keep the orchestrator generic: it owns leases, idempotency, phase ordering,
  and audit stitching only.
- Delegate lifecycle validation, admission, benchmark scoring, release safety,
  ledger persistence, and target apply/rollback to existing service-owned
  contracts.
- Preserve restart replay through governance ledger cursors.
- Fail closed when evidence, policy refs, leases, services, adapters, benchmark
  inputs, release gates, or rollback mementos are missing.

## Non-Goals

- Do not create a monolithic self-evolution worker.
- Do not move task execution, Skill materialization, release safety, benchmark
  scoring, or OS-code mutation into the orchestrator.
- Do not execute live workloads in this OpenSpec artifact itself; the
  implementation will add a provider-neutral workload runner Strategy that can
  later be backed by real task execution services.
- Do not turn the development JSONL ledger into a production Store/EventLog
  backend.

## Ownership

| Area | Owner |
| --- | --- |
| Lease acquisition, loop tick idempotency, phase ordering, audit stitching | Autonomy Evolution Live Orchestrator |
| Run lifecycle validation and transition acceptance | Autonomy Evolution Control Plane |
| Candidate metadata and Skill Creator quality gates | Admission Specification |
| Baseline/candidate measurement collection | Benchmark Workload Runner Strategy |
| Paired benchmark scoring | Normalized Benchmark Strategy |
| Quarantine, canary, promotion, rollback, supersedence decisions | Release Safety Strategy |
| Skill package apply/rollback | Skill Target Adapter through Skill service commands |
| OS-code evolution | Non-mutating OS-code proposal adapter |
| Durable replay and audit records | Governance Ledger Strategy |
| Presentation and diagnostics | SDK/SystemFacade and Web/CLI thin adapters |

## Design Patterns

- **Command:** live tick and audit reconstruction use typed command/result DTOs.
- **State:** each tick records current phase and next phase while lifecycle
  validity remains owned by the control-plane state machine.
- **Strategy:** discovery, workload collection, target dispatch, canary
  observation, and audit reconstruction are replaceable.
- **Decorator:** trace, policy, entitlement, resource, and metering gates wrap
  side-effecting calls at the service runtime boundary.
- **Observer:** observer, benchmark, release, target adapter, rollback, and
  ledger events become bounded evidence refs.
- **Memento:** workload baselines, candidate measurements, rollback refs, and
  ledger cursors make restart replay and rollback possible.
- **Specification:** admission quality, benchmark comparability, release safety,
  idempotency, and duplicate suppression are executable gates.
- **Facade:** SDK/SystemFacade exposes focused live-orchestrator clients and
  Null Object unavailable behavior.

## Loop Flow

1. A live tick command arrives with lease id, idempotency key, actor id,
   `TraceContext`, `EvolutionScope`, observer evidence refs, policy refs, audit
   refs, and optional replay cursor.
2. The orchestrator validates command shape and checks idempotency against the
   governance ledger or provider checkpoint.
3. Candidate discovery produces one or more bounded candidate descriptors. It
   must not read raw prompts, provider payloads, manifests, package bytes, or
   application-specific content.
4. The orchestrator advances control-plane transitions using existing transition
   commands and records each accepted transition in the governance ledger.
5. Admission evaluates each candidate through existing admission gates.
6. Accepted candidates enter quarantine and benchmark preparation.
7. The workload runner collects baseline and candidate measurements for the
   same generic task family, then normalized benchmark scoring returns
   `Passed`, `Failed`, or `Inconclusive`.
8. Release safety evaluates quarantine, canary, promotion, monitoring, rollback,
   supersedence, rejection, or inconclusive outcomes.
9. Target dispatch applies, monitors, supersedes, or rolls back only through the
   target adapter Strategy. Unsupported adapters return structured unavailable.
10. The audit reconstruction command replays ledger records and bounded service
    evidence refs to reconstruct the complete chain after restart.

## Failure Semantics

- Missing lease, trace id, actor id, idempotency key, observer evidence refs, or
  policy refs returns `Denied`.
- Duplicate idempotency keys return the existing checkpoint rather than
  repeating side effects.
- Missing providers or unsupported target adapters return `Unavailable`.
- Missing metrics or non-comparable workloads return `Inconclusive`.
- Failed benchmark or release gates return `Rejected` or `RolledBack` depending
  on the current phase and rollback memento availability.
- Missing rollback memento for rollback-required phases returns `Denied`.

## Observability

Every phase logs service id, command name, run id, lease id, trace id, phase,
decision, and bounded reason codes. Logs, snapshots, ledger records, and audit
results must exclude raw prompts, provider payloads, manifests, package bytes,
secrets, credentials, private keys, raw signatures, and unbounded output.
