# Change: Add Autonomy Evolution Live Orchestrator

## Why

The previous autonomy evolution changes established the governed building
blocks for complete self-evolution: control-plane lifecycle state, admission
quality gates, normalized paired benchmarking, release safety gates, a
governance ledger boundary, and a non-mutating OS-code proposal adapter. They do
not yet run as one unattended policy loop.

Macaca needs a service-owned live orchestrator that can continuously connect
observer evidence to candidate discovery, control-plane transitions, admission,
benchmark workload collection, release safety, target adapter dispatch,
governance ledger persistence, and API-first audit reconstruction without
placing orchestration semantics in the kernel, Web, CLI, frontend, or
application-specific code.

## What Changes

- Add an Autonomy Evolution Live Orchestrator capability that owns loop leases,
  idempotency, phase ordering, bounded progress checkpoints, and audit stitching.
- Add provider-neutral live tick and audit reconstruction commands/results.
- Add replaceable Strategies for candidate discovery, paired workload
  collection, target adapter dispatch, and audit reconstruction.
- Require each live tick to use trace, policy refs, bounded observer evidence,
  governance ledger append/replay, and fail-closed unavailable/denied/rejected
  results when any required service or adapter is absent.
- Wire the orchestrator to existing autonomy evolution building blocks instead
  of duplicating admission, benchmark, release, ledger, or target-specific
  mutation semantics.
- Keep OS-code evolution non-mutating by delegating OS-code targets only to the
  existing proposal adapter until a separate source-mutation proposal is
  approved.

## Impact

- Affected specs: `autonomy-evolution-control-plane`
- Affected code:
  - `macaca/crates/services/macaca-autonomy-evolution`
  - `macaca/crates/facade/macaca-sdk/src/autonomy_evolution_client.rs`
  - `macaca/crates/runtime/macaca-runtime-host/src/autonomy_evolution_service_provider.rs`
  - targeted service, SDK, and runtime-host tests
- Follow-up proof:
  - A controlled live `/api/chat/v2` task should produce observer evidence that
    the live orchestrator can advance through discovery, admission, benchmark,
    release safety, target dispatch, ledger append/replay, and API audit.

## Non-Goals

- Do not move self-evolution orchestration into the kernel.
- Do not make Web, CLI, or frontend own discovery, scoring, promotion, rollback,
  or audit semantics.
- Do not write Skill files directly from the orchestrator.
- Do not write OS source files, apply patches, run shell commands, execute
  tests, or commit code from this orchestrator.
- Do not implement application-specific benchmark workloads or branch on
  application names, workflow names, provider names, driver names, model names,
  or business domains.
- Do not make local JSONL the production Store/EventLog backend.
- Do not store raw prompts, provider payloads, manifests, package bytes,
  credentials, private keys, raw signatures, or unbounded output in logs,
  snapshots, ledgers, or audit results.
