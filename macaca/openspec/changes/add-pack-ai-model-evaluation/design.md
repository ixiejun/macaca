# AI Model Evaluation Pack Design

## Context

`pack.ai.model.evaluation.v1` is a child proposal of the developer-pack industrial capability catalog. It makes model eval suite, dataset reference, metric calculation, regression comparison, and report export available through Macaca's microkernel-compatible service model. The pack must behave like an operating-system capability: declarative in application manifests, policy checked before use, provider-neutral at the SDK boundary, observable through trace/audit evidence, and replaceable by built-in, plugin, remote, mock, or unavailable providers.

## Research Synthesis

Mature platforms converge on the same pattern:

- Platform privacy models: raw user data must not enter logs or diagnostics when processed by intelligent services.
- Android runtime permission pattern: AI operations that touch private data inherit source permissions.
- Windows capability declaration pattern: model-backed operations must be visible in app capability metadata.
- Apple entitlement/privacy pattern: sensitive processing requires policy and developer-declared purpose.

Macaca should adapt those ideas as pack descriptors, service commands, policy decorators, and replayable audit records. The design does not copy platform API names; it preserves Macaca's own microkernel boundary and service runtime execution path.

## Goals

- Provide model eval suite, dataset reference, metric calculation, regression comparison, and report export.
- Expose stable pack id `pack.ai.model.evaluation.v1`, command namespace `model_evaluation.*`, permission scopes, SDK metadata, health, snapshot, and unavailable diagnostics.
- Route every operation through `SystemFacade` or focused SDK clients into the canonical service runtime path.
- Return structured `unavailable`, `unsupported`, `denied`, `conflict`, `quota_exceeded`, and `failure` results.
- Emit sanitized trace/audit events for declaration, admission, policy, entitlement/resource decisions, service calls, provider health, snapshots, and replay.

## Non-Goals

- Do not implement an application-specific feature, workflow, UI, or business rule.
- Do not put concrete provider construction in the microkernel, SDK, shells, or generic application framework.
- Do not expose raw secrets, prompts, manifests, package bytes, credentials, raw provider payloads, private keys, signatures, or unbounded output in logs, traces, snapshots, SDK diagnostics, or examples.
- Do not silently degrade to a different provider, fake success, or bypass policy when the declared provider is absent.

## Ownership And Boundaries

- Pack id: `pack.ai.model.evaluation.v1`.
- Family: `ai`.
- Backing service owner: model evaluation service provider.
- SDK surface: `sdk.packs.ai.model.evaluation`.
- Command namespace: `model_evaluation.*`.
- Microkernel ownership: identity, service-call evidence, policy facade, trace/audit primitives only.
- Application framework ownership: manifest declaration, app-scoped permission declarations, lifecycle/effective-capability projection.
- Runtime-host ownership: provider registration and decorators only through approved composition roots.

## Command Surface

| Command | Purpose | Required behavior |
| --- | --- | --- |
| `model_evaluation.create_eval` | Typed command/result DTO for create eval | Requires trace, policy decision, structured result, and sanitized audit evidence |
| `model_evaluation.run_eval` | Typed command/result DTO for run eval | Requires trace, policy decision, structured result, and sanitized audit evidence |
| `model_evaluation.compare_runs` | Typed command/result DTO for compare runs | Requires trace, policy decision, structured result, and sanitized audit evidence |
| `model_evaluation.calculate_metrics` | Typed command/result DTO for calculate metrics | Requires trace, policy decision, structured result, and sanitized audit evidence |
| `model_evaluation.export_report` | Typed command/result DTO for export report | Requires trace, policy decision, structured result, and sanitized audit evidence |

Every command must define a typed command DTO, typed success result, typed partial-result shape when streaming or asynchronous, typed error result, redaction policy, idempotency semantics where side effects exist, and replay metadata.

## Permission, Policy, Resource, Entitlement, And Approval Model

Initial permission scopes:

- `ai.eval.run`
- `ai.eval.dataset`
- `ai.eval.report`

Policy defaults:

- Scope every command to application id, tenant id, session id, task id, and trace id when available.
- Apply model/provider neutrality, budget/rate policy, prompt/output redaction, evaluation trace, and no model-name routing in OS code.
- Require explicit approval for commands that cross user-sensitive, financial, identity, device, host, network, external-recipient, or irreversible side-effect boundaries.
- Enforce resource budgets for time, memory, storage, network, provider quota, streaming output, and retained snapshots.
- Return `denied` for policy rejection, `unavailable` for absent providers or entitlements, `unsupported` for unknown commands, and `quota_exceeded` for bounded-resource rejection.

## SDK Discovery And Examples

SDK discovery must return pack metadata, lifecycle, service mappings, command schemas, permission scopes, policy templates, examples, availability, health, provider class, version compatibility, and sanitized diagnostics.

- SDK helper example: `sdk.packs.ai.model.evaluation.create_eval(command)` builds a canonical traced service call; it never constructs providers.
- SDK helper example: `sdk.packs.ai.model.evaluation.run_eval(command)` builds a canonical traced service call; it never constructs providers.
- SDK helper example: `sdk.packs.ai.model.evaluation.compare_runs(command)` builds a canonical traced service call; it never constructs providers.

Examples must use generic handles and synthetic data. They must not bake in application names, provider names, credentials, business workflows, or domain-specific routing.

## Trace, Audit, Health, Snapshot, And Replay

Required event names:

- `model_evaluation_pack_declared`
- `model_evaluation_pack_admission_validated`
- `model_evaluation_pack_policy_decision`
- `model_evaluation_pack_service_call_requested`
- `model_evaluation_pack_service_call_succeeded`
- `model_evaluation_pack_service_call_failed`
- `model_evaluation_pack_unavailable`
- `model_evaluation_pack_snapshot_recorded`

Events include pack id, service id, command name, trace id, application/session/task/tenant identifiers when available, policy decision, provider class, latency, bounded resource counters, stable capability hash, and bounded error code. Snapshots include descriptor version, provider health, command availability, policy template hash, and sanitized replay pointers.

## Implementation Slices

1. Descriptor and contract slice: pack descriptor, command schemas, permissions, policy template, health/snapshot DTOs, unavailable diagnostics.
2. Admission and resolver slice: required/optional declaration handling, lifecycle checks, service mapping checks, permission validation, effective capability memento.
3. Service slice: model evaluation service provider command handlers or unavailable provider, lifecycle, health, snapshot, shutdown, and structured error behavior.
4. SDK slice: discovery APIs, typed command helper builders, examples, diagnostics, and Null Object behavior.
5. Observability slice: trace/audit events, replay tests, snapshot sanitization, and metrics.
6. Gates slice: OpenSpec validation, DTO compatibility, dependency-boundary tests, no-direct-provider-call tests, canonical execution-path tests, file-size gates.

## Design Patterns

- **Facade**: SDK exposes discovery and command builders; it does not construct providers.
- **Command**: every operation is a typed command/result DTO.
- **Strategy**: provider selection, unavailable behavior, policy routing, and version compatibility are replaceable.
- **Decorator**: trace, policy, resource, entitlement, approval, metering, and redaction wrap every call.
- **Specification**: admission validates pack id, lifecycle, commands, permissions, policy, and service mapping.
- **Observer**: trace, audit, health, and service events are subscribable and replayable.
- **Memento**: effective capability reports and snapshots preserve bounded recovery state.
- **Abstract Factory**: optional providers register only through approved composition roots.

## Risks And Mitigations

- Risk: broad capability becomes an OS-layer business workflow. Mitigation: keep the pack contract generic and place domain/provider semantics in replaceable services.
- Risk: SDK helpers become a second execution path. Mitigation: helpers only build canonical service-call commands and are covered by no-direct-provider-call gates.
- Risk: preview or unavailable providers look callable. Mitigation: availability validators require descriptor, service registration, command schema, permission, entitlement, and health evidence before callable state.
- Risk: observability leaks sensitive data. Mitigation: event schema permits identifiers, hashes, counters, bounded codes, and sanitized snippets only.

## Supplier Capability Matrix

| Supplier pattern | Macaca contract element |
| --- | --- |
| Eval suite definition | `EvalDefinition`, tasks, metrics, graders |
| Dataset/version reference | `EvalDatasetRef`, sample schema, immutable hash |
| Run tracking | `EvalRun`, status, parameters, artifact refs |
| Metric calculation | `EvalMetricResult`, aggregate and per-sample refs |
| CI regression gate | `EvalComparison`, thresholds, pass/fail evidence |

## Domain Model

- `EvalDefinition`: provider-neutral evaluation contract with dataset refs,
  input/output schema, graders, metrics, thresholds, and redaction profile.
- `EvalDatasetRef`: immutable dataset reference with schema hash, sample count
  band, version, provenance, and visibility policy.
- `EvalRun`: execution record with status, parameters, sampled dataset refs,
  model/capability descriptor refs, metric versions, and artifact refs.
- `EvalMetricResult`: metric id, version, aggregate value, confidence band,
  per-sample result references, and calculation diagnostics.
- `EvalReport`: exportable sanitized report artifact with comparison and gate
  outcome metadata.

## Additional Industrial Commands

- `model_evaluation.validate_dataset`: verify dataset schema, immutability, and
  visibility before a run.
- `model_evaluation.resume_run`: resume interrupted runs from checkpoints.
- `model_evaluation.evaluate_gate`: evaluate thresholds for CI/workflow gates.

## Evaluation-Specific Risks

- Risk: eval pack hardcodes benchmarks. Mitigation: datasets, metrics, graders,
  and thresholds are declared resources; the OS owns only generic contracts.
- Risk: evaluation leaks prompt/output data. Mitigation: reports use sample
  references, hashes, aggregate values, and bounded snippets only when permitted.
- Risk: non-reproducible metrics. Mitigation: metric versions, dataset hashes,
  run parameters, and provider-neutral capability descriptors are required.
