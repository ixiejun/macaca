# AI Model Evaluation Pack

`pack.ai.model.evaluation.v1` provides provider-neutral evaluation creation,
run execution, run comparison, metric calculation, and redacted report export.
The pack is descriptor-only until a serviceized model-evaluation provider is
registered.

Applications use dataset refs, sample refs, grader refs, run refs, metric refs,
and report artifact refs. They do not expose raw prompts, outputs, datasets,
concrete model names, credentials, or provider payloads in OS logs or traces.

## Manifest Declaration

```toml
[service_contract]
optional_packs = ["pack.ai.model.evaluation.v1"]
```

Optional declarations degrade with `ai_model_evaluation_provider_not_installed`.

## Permission Scopes

- `ai.eval.run`: create and run evaluations.
- `ai.eval.dataset`: reference immutable datasets and samples.
- `ai.eval.report`: compare runs, calculate metrics, and export reports.

## Commands

- `model_evaluation.create_eval`: creates an `EvalDefinition`.
- `model_evaluation.run_eval`: starts or resumes an `EvalRun`.
- `model_evaluation.compare_runs`: creates an `EvalComparison`.
- `model_evaluation.calculate_metrics`: calculates `EvalMetricResult` rows.
- `model_evaluation.export_report`: exports a redacted `EvalReport`.

## DTOs And Results

Core DTOs include `EvalDefinition`, `EvalDatasetRef`, `EvalSampleRef`,
`EvalGrader`, `EvalRun`, `EvalMetricResult`, `EvalComparison`, and
`EvalReport`. Statuses include success, partial, denied, unavailable,
unsupported, conflict, quota exceeded, mutable dataset, schema mismatch,
interrupted run, redacted report, and provider failure.

## Examples

Minimal declaration:

```toml
[service_contract]
optional_packs = ["pack.ai.model.evaluation.v1"]
```

Unavailable diagnostic:

```json
{
  "pack_id": "pack.ai.model.evaluation.v1",
  "required": false,
  "reason_code": "optional_pack_unresolved",
  "message": "ai_model_evaluation_provider_not_installed"
}
```

Canonical run command payload:

```json
{
  "subject_ref": "eval:definition-ref",
  "parameters": {
    "eval_ref": "eval-ref",
    "dataset_ref": "dataset-ref",
    "checkpoint_ref": "checkpoint-ref"
  },
  "idempotency_key": "eval-run-key"
}
```

## Trace And Audit

Trace evidence may include eval refs, immutable dataset refs, sample counts,
grader refs, metric refs, run state, checkpoint refs, report artifact refs, and
bounded status codes. It must not include raw prompts, expected outputs, model
outputs, datasets, credentials, concrete provider payloads, or unbounded report
artifacts.

## Provider Replacement

Provider classes include `eval-runner`, `metric-engine`, `report-engine`,
`mock`, and `unavailable`. Concrete evaluation runners and report exporters are
registered by runtime composition roots and are wrapped with policy, resource,
entitlement, trace, audit, and redaction decorators.
