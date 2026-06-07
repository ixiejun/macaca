# Change: Add Normalized Evolution Benchmarking

## Why

Macaca has limited optimization evidence from earlier runs, but those runs were
not consistently normalized across task families, baseline/candidate inputs, and
quality preservation. A complete self-evolution loop needs a service-owned
paired benchmark contract that can say `Passed`, `Failed`, or `Inconclusive`
from comparable evidence instead of treating lower tool calls or artifact counts
as proof by themselves.

## What Changes

- Add provider-neutral metric DTOs for tokens, elapsed time, tool calls/results,
  retries, failure recovery, quality score, human intervention rate, policy
  decisions, activation/use/success counters, artifact refs, and regression
  reasons.
- Add a paired benchmark command/result to `service.autonomy_evolution`.
- Add a default scoring Strategy that requires quality preservation before
  efficiency gains can pass.
- Make `Inconclusive` a first-class result for missing metrics, missing paired
  evidence, or non-comparable workloads.
- Keep benchmarking generic: task families are provider-neutral identifiers and
  no application workflow names are hardcoded.

## Impact

- Affected specs: `autonomy-evolution-control-plane`
- Affected code:
  - `macaca/crates/services/macaca-autonomy-evolution`
  - `macaca/crates/facade/macaca-sdk`
  - `macaca/crates/runtime/macaca-runtime-host`
- Non-goals:
  - No live workload runner in this slice.
  - No canary/promotion/rollback release chain in this slice.
  - No claim that Run 51 is normalized efficiency proof.
