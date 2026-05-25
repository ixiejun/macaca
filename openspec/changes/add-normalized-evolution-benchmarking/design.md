## Context

The control plane can now track evolution runs and evaluate admission quality.
The next gap is normalized evaluation. Historical evidence showed a bounded
optimization signal in one run, but comparable paired benchmarking was not yet a
platform contract.

## Goals

- Define a standard metric schema for autonomous evolution evaluation.
- Compare baseline and candidate measurements through one typed command.
- Require quality preservation before efficiency gains count as a pass.
- Return `Passed`, `Failed`, or `Inconclusive` with bounded reason codes and
  replayable evidence refs.
- Keep benchmark inputs provider-neutral and application-agnostic.

## Non-Goals

- Do not execute tasks or call providers.
- Do not read raw artifacts, prompts, manifests, package bytes, or provider
  payloads.
- Do not promote, quarantine, canary, or roll back candidates.
- Do not embed app names, workflow names, model names, provider names, or
  domain-specific scoring rules.

## Design Patterns

- **Command**: `EvolutionBenchmarkCommand` carries typed baseline/candidate
  measurements and returns `EvolutionBenchmarkResult`.
- **Strategy**: `EvolutionBenchmarkScoringStrategy` owns pass/fail/inconclusive
  scoring and can be replaced by future providers.
- **Specification**: comparability and required-metric checks are executable
  gates before scoring.
- **Facade**: SDK exposes a focused benchmark method and unavailable behavior.
- **Adapter**: runtime-host only decodes service envelopes and forwards calls.
- **Observer/Memento**: benchmark results carry bounded evidence refs and safe
  metric snapshots for replay.

## Scoring Rules

The default Strategy uses conservative rules:

- If task family ids differ, the result is `Inconclusive`.
- If either side lacks required evidence refs or required metrics, the result is
  `Inconclusive`.
- If candidate quality is below baseline quality beyond tolerance, the result is
  `Failed` even when efficiency improves.
- If regression reasons are present, the result is `Failed`.
- If quality is preserved and candidate improves at least one efficiency axis
  without regressing the others beyond tolerance, the result is `Passed`.
- Otherwise the result is `Inconclusive`.

Efficiency axes are elapsed time, token counts, tool calls/results, retries,
failure recovery, human intervention rate, and activation/use/success counters.

## Boundary Decisions

Benchmarking lives in `macaca-autonomy-evolution` for this slice because it is a
control-plane decision over candidate progression. A future dedicated Evaluation
service can replace the Strategy behind the same command contract. Runtime-host
and SDK remain adapters/facades and do not own scoring semantics.

## Observability

Logs record benchmark start, comparability result, scoring outcome, and bounded
reason codes. Results include trace id, actor id, run id, task family id,
policy decision refs, evidence refs, metric snapshots, score delta, and decision
reason. They never include raw prompts, provider payloads, package bytes,
manifests, credentials, raw artifacts, or unbounded output.
