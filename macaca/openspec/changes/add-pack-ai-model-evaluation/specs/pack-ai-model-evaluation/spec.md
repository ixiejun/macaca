## ADDED Requirements

### Requirement: Macaca SHALL provide the AI Model Evaluation Pack as a serviceized capability

Macaca SHALL provide `pack.ai.model.evaluation.v1` as a provider-neutral industrial pack for model eval suite, dataset reference, metric calculation, regression comparison, and report export. The pack SHALL be declared by applications, resolved by admission/catalog services, and invoked only through typed service commands.

#### Scenario: Required declaration is available
- **WHEN** an application declares `pack.ai.model.evaluation.v1` as required and model evaluation service provider is registered, healthy, entitled, and policy-admissible
- **THEN** admission SHALL expose `pack.ai.model.evaluation.v1` in the effective capability set with command schemas, permission scopes, policy template, health, and replay metadata
- **AND** SDK discovery SHALL mark callable commands as available without exposing provider secrets or raw provider payloads

#### Scenario: Required declaration is unavailable
- **WHEN** an application declares `pack.ai.model.evaluation.v1` as required but provider, permission, entitlement, resource, or host support is absent
- **THEN** admission SHALL block readiness with structured unavailable or denied diagnostics
- **AND** Macaca SHALL NOT crash, hang, silently fall back, or fake success

#### Scenario: Optional declaration is unavailable
- **WHEN** an application declares `pack.ai.model.evaluation.v1` as optional and the pack is unavailable
- **THEN** admission SHALL produce an explicit degraded effective capability report
- **AND** SDK command helpers SHALL refuse to build callable service calls for unavailable commands

### Requirement: AI Model Evaluation Pack commands SHALL use typed canonical service calls

Every `pack.ai.model.evaluation.v1` operation SHALL be represented as a typed command/result DTO and SHALL traverse the canonical service runtime path with trace, policy, resource, entitlement, approval, health, snapshot, and structured error behavior.

#### Scenario: Command succeeds through service runtime
- **WHEN** a declared and policy-allowed command such as `model_evaluation.create_eval` is invoked
- **THEN** Macaca SHALL route the command through SDK/facade helpers into the service runtime and model evaluation service provider
- **AND** it SHALL emit sanitized admission, policy, service-call, result, and replay events with stable trace identifiers

#### Scenario: Command is denied before side effects
- **WHEN** policy, permission, entitlement, approval, or resource checks reject a `pack.ai.model.evaluation.v1` command
- **THEN** Macaca SHALL return a typed denied or quota result before invoking the concrete provider
- **AND** the audit trail SHALL include the bounded reason code without raw user data or provider payloads

#### Scenario: Command is unsupported by the active provider
- **WHEN** a descriptor exists but the active provider does not support a requested command
- **THEN** Macaca SHALL return a typed unsupported result with descriptor and provider capability diagnostics
- **AND** SDK discovery SHALL report the command as non-callable for the current effective capability set

### Requirement: AI Model Evaluation Pack SHALL expose concrete industrial metadata

`pack.ai.model.evaluation.v1` SHALL expose descriptor metadata for command schemas, permission scopes, policy templates, resource budgets, SDK examples, lifecycle state, compatibility, health probes, snapshots, and unavailable diagnostics.

#### Scenario: Developer inspects the pack
- **WHEN** SDK discovery inspects `pack.ai.model.evaluation.v1`
- **THEN** it SHALL return the command namespace `model_evaluation.*`, supported commands, permissions, policy templates, examples, lifecycle, availability, health, diagnostics, and compatibility metadata
- **AND** examples SHALL use generic handles or synthetic data rather than application-specific workflows

#### Scenario: Snapshot is recorded
- **WHEN** the service runtime records a `pack.ai.model.evaluation.v1` snapshot
- **THEN** the snapshot SHALL include descriptor version, provider class, health, command availability, policy template hash, resource counters, and sanitized replay pointers
- **AND** it SHALL exclude raw secrets, credentials, prompts, manifests, package bytes, private keys, signatures, raw provider payloads, and unbounded output

### Requirement: AI Model Evaluation Pack implementation SHALL preserve Macaca boundaries

The `pack.ai.model.evaluation.v1` implementation SHALL remain owned by model evaluation service provider; the microkernel, SDK, shells, and generic application framework SHALL remain provider-neutral and free of application-specific or provider-specific routing branches.

#### Scenario: Boundary gates scan the implementation
- **WHEN** dependency, no-direct-provider-call, and canonical execution-path gates scan the implementation
- **THEN** they SHALL find no concrete provider imports in the microkernel, SDK, shells, or generic application framework
- **AND** all callable operations SHALL be reachable only through descriptor-owned service registrations and typed service commands

#### Scenario: Provider is replaced
- **WHEN** a built-in, plugin, remote, mock, or unavailable provider is selected for `pack.ai.model.evaluation.v1`
- **THEN** callers SHALL observe the same provider-neutral command/result contract
- **AND** trace/audit evidence SHALL identify only sanitized provider class and descriptor metadata rather than provider-specific business branches

### Requirement: AI Model Evaluation Pack SHALL model reproducible eval definitions and datasets

`pack.ai.model.evaluation.v1` SHALL expose typed eval definitions, immutable dataset references, sample schemas, graders, metric definitions, thresholds, and redaction profiles.

#### Scenario: Dataset reference is validated
- **WHEN** `model_evaluation.validate_dataset` is invoked before a run
- **THEN** Macaca SHALL verify dataset schema hash, version, visibility, sample-count band, and immutability evidence
- **AND** raw dataset rows, prompts, answers, and provider payloads SHALL NOT be stored in observability records

#### Scenario: Eval definition is provider neutral
- **WHEN** `model_evaluation.create_eval` defines a suite
- **THEN** it SHALL reference capability descriptors, dataset refs, graders, metrics, thresholds, and policy templates rather than concrete provider or model names
- **AND** OS-layer code SHALL NOT branch on provider-specific benchmark names

#### Scenario: Metric version is recorded
- **WHEN** `model_evaluation.calculate_metrics` computes aggregate or per-sample metrics
- **THEN** every metric result SHALL include metric id, version, calculation policy, aggregate value, confidence band, and result references
- **AND** replay SHALL be able to explain which metric version produced the result

#### Scenario: Report export is sanitized
- **WHEN** `model_evaluation.export_report` exports an eval report
- **THEN** Macaca SHALL include dataset hashes, run parameters, metric summaries, comparison outcome, artifact refs, and bounded diagnostics
- **AND** raw prompts, outputs, datasets, credentials, provider payloads, and unbounded artifacts SHALL NOT be exported unless a separate policy permits a specific artifact reference

### Requirement: AI Model Evaluation Pack SHALL run, resume, compare, and gate evals

`pack.ai.model.evaluation.v1` SHALL support durable eval runs with checkpoints, comparisons, and threshold gates.

#### Scenario: Eval run creates checkpointed progress
- **WHEN** `model_evaluation.run_eval` processes an eval dataset
- **THEN** Macaca SHALL record run state, checkpoint references, completed sample refs, metric progress, resource counters, and trace links
- **AND** interruption SHALL leave enough sanitized state for `model_evaluation.resume_run`

#### Scenario: Interrupted run resumes deterministically
- **WHEN** `model_evaluation.resume_run` resumes an interrupted run
- **THEN** Macaca SHALL continue from validated checkpoints without reprocessing completed samples unless retry policy requires it
- **AND** replay SHALL connect pre-interruption and post-resume events

#### Scenario: Comparison evaluates thresholds
- **WHEN** `model_evaluation.compare_runs` compares a candidate run to a baseline
- **THEN** Macaca SHALL calculate metric deltas, threshold pass/fail state, regression reason codes, and report artifact refs
- **AND** hidden per-sample data SHALL NOT leak through comparison diagnostics

#### Scenario: Gate blocks failed regression
- **WHEN** `model_evaluation.evaluate_gate` finds threshold failure or missing required metric evidence
- **THEN** Macaca SHALL return a blocked gate result before downstream workflow closure
- **AND** the audit trail SHALL include metric ids, threshold ids, and bounded reason codes
