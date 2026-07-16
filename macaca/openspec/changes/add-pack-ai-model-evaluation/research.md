# AI Model Evaluation Pack Research

## Purpose

This note records borrowed platform patterns, Macaca mapping, existing platform
inventory, and GitNexus memo evidence for
`pack.ai.model.evaluation.v1`. The pack must provide eval definitions, dataset
refs, runs, metrics, comparisons, and report export through provider-neutral
commands. It must not own application-specific quality criteria, raw dataset
storage, hidden prompts, or provider-native evaluation dashboards.

## Source Baseline

- OpenAI Evals and evaluation guidance:
  <https://platform.openai.com/docs/guides/evals>
- AWS Bedrock model evaluation documentation:
  <https://docs.aws.amazon.com/bedrock/latest/userguide/model-evaluation.html>
- Google Vertex AI model evaluation and Gen AI evaluation documentation:
  <https://cloud.google.com/vertex-ai/docs/evaluation/introduction>
- Azure AI Foundry evaluation documentation:
  <https://learn.microsoft.com/en-us/azure/ai-foundry/how-to/evaluate-generative-ai-app>
- MLflow model evaluation documentation:
  <https://mlflow.org/docs/latest/model-evaluation/index.html>

## Borrowed Platform Patterns

- Evaluation platforms converge on immutable datasets, samples, graders,
  metrics, thresholds, runs, comparisons, artifacts, and reports.
- Generative AI evaluation often mixes automated metrics, model-graded rubrics,
  human review, and safety/quality dimensions. Macaca should model grader
  provenance and metric versioning rather than hardcoding a business rubric.
- Long-running runs need checkpoints, interrupted-run resume, per-sample
  results, aggregate metrics, and report export with redaction.
- Dataset samples may contain prompts, expected outputs, retrieved context, and
  sensitive provider responses. Macaca should reference datasets/artifacts and
  redact report exports.
- Evaluation is a governance capability. It should inspect model/provider
  behavior through service calls and audit, not bypass LLM/rerank/vision/speech
  pack policy.

## Macaca Mapping

- Descriptor: `pack.ai.model.evaluation.v1`, command namespace
  `model_evaluation.*`, scopes `ai.eval.run`, `ai.eval.dataset`, and
  `ai.eval.report`.
- Commands: `model_evaluation.create_eval`, `model_evaluation.run_eval`,
  `model_evaluation.compare_runs`, `model_evaluation.calculate_metrics`, and
  `model_evaluation.export_report`.
- DTOs: `EvalDefinition`, `EvalDatasetRef`, `EvalSampleRef`, `EvalGrader`,
  `EvalRun`, `EvalMetricResult`, `EvalComparison`, and `EvalReport`.
- Policy: validate dataset visibility, immutability, sample count, metric
  versions, grader permissions, delegated AI pack scopes, report redaction,
  resource budget, entitlement, and provider capability before dispatch.
- Trace/audit: record dataset hashes, sample counts, metric ids/versions,
  thresholds, run refs, checkpoint refs, aggregate counters, comparison refs,
  report refs, and bounded errors only.

## Existing Macaca Platform Inventory

- Existing application package certification and conformance checker code
  demonstrates deterministic Specification-style evaluation of rules and
  sanitized reports. It can inform eval-run structure but does not complete the
  model evaluation pack.
- Autonomy evolution client tests and release/admission evaluation tests show
  service-backed evaluation patterns and unavailable clients that can inform SDK
  design.
- Generic service descriptors, `SystemFacade`, trace-required service calls,
  persistence/event-log lineage, scheduler/resource DTOs, and unavailable
  clients provide the reusable platform base for long-running evaluation runs.
- No current evidence proves model-evaluation DTOs, provider traits, datasets,
  metrics, SDK helpers, report export redaction tests, or developer docs are
  complete.

## GitNexus Memo

No Rust symbol was edited for this research task. GitNexus CRITICAL/HIGH
findings remain memo-only for this refactor per the active user instruction and
will be recorded again before implementation commits that touch code symbols.
