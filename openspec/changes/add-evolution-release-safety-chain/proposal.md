# Change: Add Evolution Release Safety Chain

## Why

The current autonomy evolution control plane can record lifecycle transitions,
admit candidates, and score normalized paired benchmarks, but it still lacks an
executable release safety chain. A fully autonomous agent OS needs quarantine,
canary, promotion, active monitoring, rollback, and supersedence decisions to be
policy-gated and replayable before any candidate can become an active
capability.

## What Changes

- Add a typed release safety command/result for quarantine, canary, promotion,
  monitoring, rollback, supersedence, rejection, and inconclusive outcomes.
- Add an executable release policy gate that evaluates capability diff,
  package ownership, tenant/application scope, trust level, resource permission,
  executable change flags, blast-radius score, benchmark decision, and rollback
  memento readiness.
- Add rollback memento DTOs that carry replayable refs only; no package bytes,
  raw manifests, raw prompts, raw provider payloads, or application-specific
  data enter the service surface.
- Expose the release command through the Autonomy Evolution service contract,
  SDK facade, and runtime-host adapter with structured unavailable behavior.
- Add tests for dry-run policy evaluation, canary pass, canary failure,
  rollback after canary failure, SDK unavailable behavior, and runtime-host
  command decoding.

## Impact

- Affected specs: `autonomy-evolution-control-plane`
- Affected code:
  - `macaca/crates/services/macaca-autonomy-evolution`
  - `macaca/crates/facade/macaca-sdk/src/autonomy_evolution_client.rs`
  - `macaca/crates/runtime/macaca-runtime-host/src/autonomy_evolution_service_provider.rs`
  - targeted service, SDK, and runtime-host tests
