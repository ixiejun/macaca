use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::ai_common::{
    ai_bounded_token, ai_pack_definition, ai_stable_hash, define_ai_command_wrappers,
    AiPackCommandEnvelope, AiPackDescriptor, AiPackError, AiPackPage, AiProviderClass,
};
use super::model::{DomainPackDefinition, DomainPackProviderCapabilityState};

pub const AI_MODEL_EVALUATION_PACK_ID: &str = "pack.ai.model.evaluation.v1";
pub const AI_MODEL_EVALUATION_SERVICE_ID: &str = "service.ai.model_evaluation";

/// Canonical command names described by `pack.ai.model.evaluation.v1`.
pub const AI_MODEL_EVALUATION_COMMANDS: &[&str] = &[
    "model_evaluation.create_eval",
    "model_evaluation.run_eval",
    "model_evaluation.compare_runs",
    "model_evaluation.calculate_metrics",
    "model_evaluation.export_report",
];

const MODEL_EVALUATION_PERMISSION_SCOPES: &[&str] =
    &["ai.eval.run", "ai.eval.dataset", "ai.eval.report"];

const EVAL_RUNNER_METADATA: &[(&str, &str)] = &[
    ("datasets", "reference_only"),
    ("checkpoint_resume", "true"),
    ("raw_outputs_in_trace", "false"),
];
const METRIC_ENGINE_METADATA: &[(&str, &str)] =
    &[("metrics", "versioned"), ("comparisons", "true")];
const REPORT_ENGINE_METADATA: &[(&str, &str)] =
    &[("exports", "redacted"), ("artifact_handles", "true")];
const MOCK_METADATA: &[(&str, &str)] = &[("deterministic", "true"), ("eval_payloads", "synthetic")];
const UNAVAILABLE_METADATA: &[(&str, &str)] =
    &[("callable", "false"), ("reason", "provider_not_installed")];

const MODEL_EVALUATION_PROVIDER_CLASSES: &[AiProviderClass<'_>] = &[
    AiProviderClass {
        provider_class: "eval-runner",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: EVAL_RUNNER_METADATA,
    },
    AiProviderClass {
        provider_class: "metric-engine",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: METRIC_ENGINE_METADATA,
    },
    AiProviderClass {
        provider_class: "report-engine",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: REPORT_ENGINE_METADATA,
    },
    AiProviderClass {
        provider_class: "mock",
        availability: DomainPackProviderCapabilityState::Preview,
        metadata: MOCK_METADATA,
    },
    AiProviderClass {
        provider_class: "unavailable",
        availability: DomainPackProviderCapabilityState::Unavailable,
        metadata: UNAVAILABLE_METADATA,
    },
];

/// Build the model-evaluation pack descriptor without binding a concrete eval provider.
pub fn ai_model_evaluation_pack_definition() -> DomainPackDefinition {
    ai_pack_definition(AiPackDescriptor {
        pack_id: AI_MODEL_EVALUATION_PACK_ID,
        child_change_id: "openspec:add-pack-ai-model-evaluation",
        docs_slug: "model-evaluation",
        sdk_slug: "model.evaluation",
        service_id: AI_MODEL_EVALUATION_SERVICE_ID,
        commands: AI_MODEL_EVALUATION_COMMANDS,
        permission_scopes: MODEL_EVALUATION_PERMISSION_SCOPES,
        provider_classes: MODEL_EVALUATION_PROVIDER_CLASSES,
        health_probe: "model_evaluation.calculate_metrics",
        unavailable_reason: "ai_model_evaluation_provider_not_installed",
        replay_schema: "ai.model_evaluation.replay.v1",
        data_classification: "ai_model_evaluation_reference_metadata",
        retention_policy: "eval_definitions_dataset_refs_sample_refs_runs_metrics_comparisons_and_reports_by_reference",
        redaction_policy: "raw_prompts_outputs_datasets_credentials_model_names_and_provider_payloads_redacted",
        timeout_ms: 300_000,
        budget_units: 16,
        examples: &[
            "Declare `pack.ai.model.evaluation.v1` as optional until an evaluation provider is installed.",
            "Use dataset refs, sample refs, metric refs, run ids, and redacted report artifacts instead of raw prompts or outputs.",
        ],
        migration_notes: &[
            "Evaluation commands become callable only after an approved model-evaluation service provider registers matching schemas.",
            "Provider-native datasets, model outputs, graders, and report payloads stay behind service adapters.",
        ],
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvalDefinition {
    pub eval_ref: String,
    pub dataset: EvalDatasetRef,
    pub graders: Vec<EvalGrader>,
    pub metric_refs: BTreeSet<String>,
    pub visibility: String,
}

impl EvalDefinition {
    /// Validate eval metadata without dereferencing private datasets or prompts.
    pub fn is_bounded(&self, max_graders: usize, max_metrics: usize) -> bool {
        ai_bounded_token(&self.eval_ref, 128)
            && self.dataset.is_immutable(max_metrics as u64 * 1_000)
            && !self.graders.is_empty()
            && self.graders.len() <= max_graders
            && !self.metric_refs.is_empty()
            && self.metric_refs.len() <= max_metrics
            && self.graders.iter().all(EvalGrader::is_bounded)
            && ai_bounded_token(&self.visibility, 64)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvalDatasetRef {
    pub dataset_ref: String,
    pub schema_ref: String,
    pub version_hash: String,
    pub sample_count: u64,
    pub immutable: bool,
}

impl EvalDatasetRef {
    /// Validate dataset immutability, schema compatibility, and bounded sample count.
    pub fn is_immutable(&self, max_samples: u64) -> bool {
        ai_bounded_token(&self.dataset_ref, 128)
            && ai_bounded_token(&self.schema_ref, 128)
            && ai_bounded_token(&self.version_hash, 256)
            && self.sample_count > 0
            && self.sample_count <= max_samples
            && self.immutable
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvalSampleRef {
    pub sample_ref: String,
    pub dataset_ref: String,
    pub input_hash: String,
    pub expected_output_ref: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvalGrader {
    pub grader_ref: String,
    pub grader_kind: String,
    pub policy_scope: String,
    pub version_hash: String,
}

impl EvalGrader {
    /// Validate grader metadata without embedding grader prompts or outputs.
    pub fn is_bounded(&self) -> bool {
        ai_bounded_token(&self.grader_ref, 128)
            && ai_bounded_token(&self.grader_kind, 128)
            && ai_bounded_token(&self.policy_scope, 128)
            && ai_bounded_token(&self.version_hash, 256)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvalRun {
    pub run_ref: String,
    pub eval_ref: String,
    pub state: String,
    pub checkpoint_ref: Option<String>,
    pub completed_sample_count: u64,
}

impl EvalRun {
    /// Interrupted or checkpointed runs must carry a replayable checkpoint reference.
    pub fn can_resume(&self, dataset: &EvalDatasetRef) -> bool {
        ai_bounded_token(&self.run_ref, 128)
            && ai_bounded_token(&self.eval_ref, 128)
            && self.completed_sample_count <= dataset.sample_count
            && matches!(self.state.as_str(), "checkpointed" | "interrupted")
            && self
                .checkpoint_ref
                .as_ref()
                .is_some_and(|checkpoint| ai_bounded_token(checkpoint, 256))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvalMetricResult {
    pub metric_ref: String,
    pub version: String,
    pub aggregate_score_micros: u32,
    pub per_sample_result_ref: Option<String>,
    pub threshold_passed: bool,
}

impl EvalMetricResult {
    /// Validate metric versioning, aggregate score, and per-sample result references.
    pub fn is_versioned(&self) -> bool {
        ai_bounded_token(&self.metric_ref, 128)
            && ai_bounded_token(&self.version, 64)
            && self.aggregate_score_micros <= 1_000_000
            && self
                .per_sample_result_ref
                .as_ref()
                .is_none_or(|reference| ai_bounded_token(reference, 256))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvalComparison {
    pub comparison_ref: String,
    pub baseline_run_ref: String,
    pub candidate_run_ref: String,
    pub metric_results: Vec<EvalMetricResult>,
}

impl EvalComparison {
    /// Validate comparison contracts across baseline and candidate runs.
    pub fn is_comparable(&self) -> bool {
        ai_bounded_token(&self.comparison_ref, 128)
            && ai_bounded_token(&self.baseline_run_ref, 128)
            && ai_bounded_token(&self.candidate_run_ref, 128)
            && self.baseline_run_ref != self.candidate_run_ref
            && !self.metric_results.is_empty()
            && self
                .metric_results
                .iter()
                .all(EvalMetricResult::is_versioned)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvalReport {
    pub report_ref: String,
    pub run_ref: String,
    pub artifact_ref: String,
    pub redaction_profile: String,
    pub bounded_summary_ref: String,
}

impl EvalReport {
    /// Validate report export redaction and bounded artifact handles.
    pub fn is_redacted(&self) -> bool {
        ai_bounded_token(&self.report_ref, 128)
            && ai_bounded_token(&self.run_ref, 128)
            && ai_bounded_token(&self.artifact_ref, 256)
            && ai_bounded_token(&self.redaction_profile, 128)
            && ai_bounded_token(&self.bounded_summary_ref, 256)
    }
}

define_ai_command_wrappers!(
    ModelEvaluationCreateEvalCommand,
    ModelEvaluationRunEvalCommand,
    ModelEvaluationCompareRunsCommand,
    ModelEvaluationCalculateMetricsCommand,
    ModelEvaluationExportReportCommand,
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelEvaluationResultStatus {
    Success,
    Partial,
    Denied,
    Unavailable,
    Unsupported,
    Conflict,
    QuotaExceeded,
    DatasetMutable,
    SchemaMismatch,
    RunInterrupted,
    ReportRedacted,
    ProviderFailure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelEvaluationResultEnvelope<T> {
    pub status: ModelEvaluationResultStatus,
    pub data: Option<T>,
    pub page: Option<AiPackPage<T>>,
    pub error: Option<AiPackError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelEvaluationDescriptorHashes {
    pub command_schema_hash: String,
    pub result_schema_hash: String,
    pub descriptor_hash: String,
    pub provider_capability_hash: String,
    pub eval_hash: String,
    pub run_hash: String,
    pub metric_hash: String,
    pub report_hash: String,
}

pub fn ai_model_evaluation_descriptor_hashes() -> ModelEvaluationDescriptorHashes {
    let dataset = EvalDatasetRef {
        dataset_ref: "dataset".into(),
        schema_ref: "schema".into(),
        version_hash: "dataset-version".into(),
        sample_count: 1,
        immutable: true,
    };
    let metric = EvalMetricResult {
        metric_ref: "metric".into(),
        version: "v1".into(),
        aggregate_score_micros: 900_000,
        per_sample_result_ref: Some("sample-results".into()),
        threshold_passed: true,
    };
    ModelEvaluationDescriptorHashes {
        command_schema_hash: model_evaluation_stable_hash(&AI_MODEL_EVALUATION_COMMANDS),
        result_schema_hash: model_evaluation_stable_hash(&ModelEvaluationResultStatus::Success),
        descriptor_hash: model_evaluation_stable_hash(&ai_model_evaluation_pack_definition()),
        provider_capability_hash: model_evaluation_stable_hash(&BTreeMap::from([(
            "provider_class".to_string(),
            "mock".to_string(),
        )])),
        eval_hash: model_evaluation_stable_hash(&EvalDefinition {
            eval_ref: "eval".into(),
            dataset,
            graders: vec![EvalGrader {
                grader_ref: "grader".into(),
                grader_kind: "reference_metric".into(),
                policy_scope: "ai.eval.run".into(),
                version_hash: "grader-version".into(),
            }],
            metric_refs: BTreeSet::from(["metric".into()]),
            visibility: "tenant".into(),
        }),
        run_hash: model_evaluation_stable_hash(&EvalRun {
            run_ref: "run".into(),
            eval_ref: "eval".into(),
            state: "checkpointed".into(),
            checkpoint_ref: Some("checkpoint".into()),
            completed_sample_count: 1,
        }),
        metric_hash: model_evaluation_stable_hash(&metric),
        report_hash: model_evaluation_stable_hash(&EvalReport {
            report_ref: "report".into(),
            run_ref: "run".into(),
            artifact_ref: "artifact".into(),
            redaction_profile: "eval-report-redaction-v1".into(),
            bounded_summary_ref: "summary".into(),
        }),
    }
}

pub fn model_evaluation_stable_hash<T: Serialize + ?Sized>(value: &T) -> String {
    ai_stable_hash(value)
}
