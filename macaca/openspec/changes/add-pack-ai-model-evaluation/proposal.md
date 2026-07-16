# Change: Add AI Model Evaluation Pack

## Why

Developers need `pack.ai.model.evaluation.v1` as a real industrial capability for model eval suite, dataset reference, metric calculation, regression comparison, and report export. The pack must not be a catalog label only: if an application declares it and the provider is installed, the SDK must expose callable typed commands; if policy, entitlement, provider, or host support is absent, Macaca must return explicit unavailable or denied diagnostics.

This child proposal intentionally narrows the broad industrial catalog into one implementable service-backed pack. It records the research basis, capability boundary, command surface, permission model, observability model, and acceptance gates needed before this pack can be called production-ready.

## Research And Borrowed Platform Patterns

The proposal borrows stable ideas from mature application platforms without copying their product-specific APIs:

- Platform privacy models: raw user data must not enter logs or diagnostics when processed by intelligent services.
- Android runtime permission pattern: AI operations that touch private data inherit source permissions.
- Windows capability declaration pattern: model-backed operations must be visible in app capability metadata.
- Apple entitlement/privacy pattern: sensitive processing requires policy and developer-declared purpose.

The Macaca interpretation is: app manifests declare pack intent; admission validates capability and permission metadata; runtime permission and approval gates happen before sensitive side effects; concrete providers stay behind service descriptors; traces and audits prove the execution path.

## What Changes

- Add the provider-neutral `pack.ai.model.evaluation.v1` contract under the `ai` family.
- Define a concrete command namespace, initial industrial command set, result/error DTO requirements, permission scopes, policy defaults, resource/entitlement behavior, and unavailable diagnostics.
- Bind implementation ownership to model evaluation service provider; the kernel, SDK, shells, and generic application framework remain provider-neutral.
- Add SDK discovery metadata and examples for create eval, run eval, compare runs, calculate metrics, export report.
- Add trace, audit, health, snapshot, replay, and boundary gates proving the pack uses the canonical service path.

## Impact

- Affected specs: `pack-ai-model-evaluation`, `developer-pack-industrial-capability-catalog`, `sdk-system-facade`, `service-runtime`, `unified-execution-path`.
- Affected code later: provider-neutral DTOs in protocol crates, descriptor/admission validators, SDK discovery/command helpers, model evaluation service provider, optional provider registration, trace/audit tests, and dependency-boundary gates.
- Non-goals: no application-specific workflow, no provider-name routing in OS layers, no concrete provider construction in SDK/shell/kernel, and no fake success for absent providers.

## Supplier/API Baseline To Match

- OpenAI Evals and similar model-evaluation workflows: eval definitions,
  datasets, graders, runs, metrics, and regression reports.
- MLflow and Weights & Biases style tracking: experiment/run identity,
  artifacts, metrics, comparisons, lineage, and report export.
- RAGAS/IR evaluation patterns: dataset references, metric calculators,
  per-example results, aggregate metrics, and reproducibility metadata.
- CI quality gates: baseline comparison, threshold checks, pass/fail status,
  report artifacts, and audit history.

Macaca's model-evaluation pack evaluates model/provider behavior through
declared datasets and metric contracts without embedding application-specific
benchmarks, provider names, or model names in OS code.

## Industrial Acceptance Bar

This proposal is not complete until implementation tasks produce a developer
guide at `docs/developer-packs/ai/model-evaluation.md`, typed eval/dataset/
grader/run/metric/comparison/report DTOs, deterministic tests for dataset
immutability and threshold gates, and audit replay proving eval conclusions are
reproducible from sanitized dataset references, run parameters, metric versions,
and result hashes.
