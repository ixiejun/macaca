use std::collections::{BTreeMap, BTreeSet};

use super::ai_embedding::*;
use super::ai_llm::*;
use super::ai_model_evaluation::*;
use super::ai_rerank::*;
use super::ai_speech::*;
use super::ai_vision::*;

// These tests validate provider-neutral AI pack contracts only. They use
// references, hashes, handles, counters, and redacted metadata; they never call
// hosted models, local runtimes, OCR engines, speech engines, eval runners, or
// any other concrete provider.

#[test]
fn llm_contract_validates_streaming_structured_tool_and_budget_gates() {
    let frames = vec![
        LlmStreamFrame {
            stream_ref: "stream".into(),
            sequence: 1,
            frame_kind: "delta".into(),
            delta_ref: "delta-1".into(),
            terminal: false,
        },
        LlmStreamFrame {
            stream_ref: "stream".into(),
            sequence: 2,
            frame_kind: "final".into(),
            delta_ref: "final".into(),
            terminal: true,
        },
    ];
    let reserved = LlmBudgetEnvelope {
        max_input_tokens: 1_000,
        max_output_tokens: 500,
        max_cost_micros: 10_000,
        retained_output_bytes: 64_000,
    };
    let usage = LlmBudgetEnvelope {
        max_input_tokens: 800,
        max_output_tokens: 200,
        max_cost_micros: 5_000,
        retained_output_bytes: 8_000,
    };
    let schema_ref = "schema-ref";
    let generation = LlmGeneration {
        generation_ref: "generation".into(),
        content: vec![LlmContentBlock {
            block_ref: "block".into(),
            content_kind: "structured_output_ref".into(),
            payload_ref: schema_ref.into(),
            payload_hash: "schema-output-hash".into(),
        }],
        finish_reason: "stop".into(),
        usage: usage.clone(),
        safety_summary_ref: "safety".into(),
    };
    let tool_call = LlmToolCall {
        call_ref: "call".into(),
        tool_ref: "tool".into(),
        capability_scope: "service.knowledge.search".into(),
        argument_schema_ref: "argument-schema".into(),
        argument_hash: "argument-hash".into(),
    };
    let schema_mismatch = LlmResultEnvelope::<LlmGeneration> {
        status: LlmResultStatus::SchemaMismatch,
        data: None,
        page: None,
        error: Some(super::ai_common::AiPackError {
            code: "schema_mismatch".into(),
            message: "schema validation failed".into(),
            retryable: false,
            trace_safe_detail: Some("schema-ref".into()),
        }),
    };

    assert!(LlmStreamFrame::sequence_is_finalized(&frames));
    assert!(generation.matches_structured_schema(schema_ref));
    assert!(tool_call.requires_policy_gate());
    assert!(usage.fits_within(&reserved));
    assert!(matches!(
        schema_mismatch.status,
        LlmResultStatus::SchemaMismatch
    ));

    let late_frame = LlmStreamFrame {
        sequence: 3,
        terminal: false,
        ..frames[1].clone()
    };
    assert!(!LlmStreamFrame::sequence_is_finalized(&[
        frames[0].clone(),
        frames[1].clone(),
        late_frame,
    ]));
    let cancelled = LlmStreamFrame {
        frame_kind: "cancelled".into(),
        terminal: true,
        ..frames[1].clone()
    };
    assert!(LlmStreamFrame::sequence_is_finalized(&[
        frames[0].clone(),
        cancelled,
    ]));
    let serialized = serde_json::to_string(&schema_mismatch).unwrap();
    assert!(!serialized.contains("raw_prompt"));
    assert!(!serialized.contains("provider_payload"));
}

#[test]
fn embedding_contract_preserves_batch_mapping_and_schema_bounds() {
    let schema = VectorSchemaDescriptor {
        schema_ref: "schema".into(),
        dimension: 384,
        numeric_type: "float32".into(),
        metric: "cosine".into(),
        normalization: "unit".into(),
    };
    let request = EmbeddingBatchRequest {
        batch_ref: "batch".into(),
        inputs: vec![
            EmbeddingInput {
                input_ref: "item-a".into(),
                modality: "text".into(),
                content_ref: "content-a".into(),
                content_hash: "hash-a".into(),
                truncation_policy: "bounded".into(),
            },
            EmbeddingInput {
                input_ref: "item-b".into(),
                modality: "image".into(),
                content_ref: "content-b".into(),
                content_hash: "hash-b".into(),
                truncation_policy: "tail".into(),
            },
        ],
        schema,
        idempotency_key: "idem-batch".into(),
    };
    let result = EmbeddingBatchResult {
        batch_ref: "batch".into(),
        vectors: vec![EmbeddingVector {
            item_ref: "item-a".into(),
            vector_ref: "vector-a".into(),
            dimension: 384,
            numeric_type: "float32".into(),
            normalized: true,
        }],
        failed_item_refs: vec!["item-b".into()],
        usage: EmbeddingUsage {
            input_count: 2,
            accepted_count: 1,
            rejected_count: 1,
            cost_micros: 1,
        },
    };

    assert!(request.is_bounded(8));
    assert!(request.schema.is_compatible());
    assert!(request.result_preserves_item_mapping(&result));
    assert!(result.diagnostics_are_bounded(&request));
    assert!(result.vectors[0].matches_schema(&request.schema));

    let serialized = serde_json::to_string(&result).unwrap();
    assert!(!serialized.contains("raw"));
    assert!(!serialized.contains("embedding_values"));

    let duplicate = EmbeddingBatchResult {
        failed_item_refs: vec!["item-a".into()],
        ..result
    };
    assert!(!request.result_preserves_item_mapping(&duplicate));

    let unsupported_modality = EmbeddingBatchRequest {
        inputs: vec![EmbeddingInput {
            modality: "binary".into(),
            ..request.inputs[0].clone()
        }],
        ..request.clone()
    };
    assert!(!unsupported_modality.is_bounded(8));
    assert!(!request.is_bounded(1));
}

#[test]
fn rerank_contract_validates_candidates_ordering_explanations_and_batch_mapping() {
    let request = RerankRequest {
        request_ref: "request".into(),
        query: RerankQuery {
            query_ref: "query".into(),
            text_ref: "query-text-ref".into(),
            query_hash: "query-hash".into(),
        },
        candidates: vec![
            RerankCandidate {
                candidate_ref: "candidate-a".into(),
                content_ref: "content-a".into(),
                content_hash: "hash-a".into(),
                hidden: false,
            },
            RerankCandidate {
                candidate_ref: "candidate-b".into(),
                content_ref: "content-b".into(),
                content_hash: "hash-b".into(),
                hidden: false,
            },
        ],
        top_n: 2,
        score_normalization: "unit".into(),
    };
    let results = vec![
        RerankResult {
            candidate_ref: "candidate-a".into(),
            rank: 1,
            score_micros: 900_000,
            explanation_ref: Some("explanation-a".into()),
        },
        RerankResult {
            candidate_ref: "candidate-b".into(),
            rank: 2,
            score_micros: 900_000,
            explanation_ref: Some("explanation-b".into()),
        },
    ];
    let explanation = RerankExplanation {
        explanation_ref: "explanation-a".into(),
        redacted_summary_ref: "summary-a".into(),
        score_basis_ref: "basis-a".into(),
    };
    let batch = RerankBatchResult {
        batch_ref: "batch".into(),
        query_results: BTreeMap::from([("query".into(), results.clone())]),
        failed_query_refs: vec![],
    };

    assert!(request.is_bounded(8));
    assert!(rerank_results_are_deterministic(&results));
    assert!(explanation.is_redacted());
    assert!(batch.preserves_query_mapping(&BTreeSet::from(["query".into()])));

    let hidden = RerankRequest {
        candidates: vec![RerankCandidate {
            hidden: true,
            ..request.candidates[0].clone()
        }],
        top_n: 1,
        ..request
    };
    assert!(!hidden.is_bounded(8));
}

#[test]
fn vision_contract_validates_regions_ocr_jobs_and_moderation_policy() {
    let region = VisualRegion {
        region_ref: "region".into(),
        coordinate_space: "normalized".into(),
        x_micros: 10,
        y_micros: 20,
        width_micros: 30,
        height_micros: 40,
    };
    let spans = vec![
        OcrTextSpan {
            span_ref: "span-1".into(),
            page_ref: "page-1".into(),
            block_ref: "block-1".into(),
            line_ref: "line-1".into(),
            redacted_text_ref: "text-1".into(),
            region: region.clone(),
        },
        OcrTextSpan {
            span_ref: "span-2".into(),
            page_ref: "page-1".into(),
            block_ref: "block-1".into(),
            line_ref: "line-2".into(),
            redacted_text_ref: "text-2".into(),
            region: region.clone(),
        },
    ];
    let job = VisionJob {
        job_ref: "job".into(),
        state: "partial".into(),
        input_ref: "input".into(),
        progress_basis_points: 5_000,
        result_ref: Some("partial-result".into()),
    };
    let moderation = VisualModerationResult {
        result_ref: "moderation".into(),
        category_refs: vec!["policy-category".into()],
        policy_action: "review".into(),
        evidence_ref: "evidence".into(),
    };

    assert!(region.is_normalized());
    assert!(OcrTextSpan::spans_are_layout_ordered(&spans));
    assert!(job.is_replayable());
    assert!(moderation.is_policy_safe());

    for state in ["queued", "running", "timed_out", "cancelled"] {
        assert!(VisionJob {
            state: state.into(),
            ..job.clone()
        }
        .is_replayable());
    }
    let unavailable = VisionResultEnvelope::<VisionJob> {
        status: VisionResultStatus::Unavailable,
        data: None,
        page: None,
        error: None,
    };
    assert!(matches!(
        unavailable.status,
        VisionResultStatus::Unavailable
    ));

    let mut out_of_bounds = region;
    out_of_bounds.width_micros = 1_000_001;
    assert!(!out_of_bounds.is_normalized());
}

#[test]
fn speech_contract_validates_stream_order_alignment_voice_and_output_format() {
    let frames = vec![
        SpeechStreamFrame {
            stream_ref: "stream".into(),
            sequence: 1,
            frame_kind: "audio_delta".into(),
            audio_chunk_ref: Some("chunk-1".into()),
            transcript_delta_ref: None,
            terminal: false,
        },
        SpeechStreamFrame {
            stream_ref: "stream".into(),
            sequence: 2,
            frame_kind: "final".into(),
            audio_chunk_ref: None,
            transcript_delta_ref: Some("transcript-final".into()),
            terminal: true,
        },
    ];
    let segment = TranscriptSegment {
        segment_ref: "segment".into(),
        speaker_ref: Some("speaker".into()),
        text_ref: "text-redacted".into(),
        start_ms: 1,
        end_ms: 2,
        confidence_micros: 900_000,
    };
    let voice = VoiceDescriptor {
        voice_ref: "voice".into(),
        locale_tags: BTreeSet::from(["en-US".into()]),
        style_tags: BTreeSet::from(["neutral".into()]),
        consent_required: true,
    };
    let request = SpeechSynthesisRequest {
        request_ref: "synthesis".into(),
        text_ref: "text-ref".into(),
        voice_ref: "voice".into(),
        output_format: "opus".into(),
        max_duration_ms: 30_000,
    };
    let alignment = SpeechAlignment {
        alignment_ref: "alignment".into(),
        segment_ref: "segment".into(),
        word_timing_ref: "word-timing".into(),
        language_tag: "en-US".into(),
    };

    assert!(SpeechStreamFrame::sequence_is_finalized(&frames));
    assert!(segment.is_aligned());
    assert!(voice.supports("en-US", "neutral"));
    assert!(request.is_compatible_with(&voice, "en-US", "neutral"));
    assert!(alignment.is_bounded());

    let late_frame = SpeechStreamFrame {
        sequence: 3,
        terminal: false,
        ..frames[1].clone()
    };
    assert!(!SpeechStreamFrame::sequence_is_finalized(&[
        frames[0].clone(),
        frames[1].clone(),
        late_frame,
    ]));

    let cancelled = SpeechStreamFrame {
        frame_kind: "cancelled".into(),
        terminal: true,
        ..frames[1].clone()
    };
    assert!(SpeechStreamFrame::sequence_is_finalized(&[
        frames[0].clone(),
        cancelled,
    ]));
    let serialized = serde_json::to_string(&request).unwrap();
    assert!(!serialized.contains("raw_audio"));
    assert!(!serialized.contains("generated_speech"));
}

#[test]
fn model_evaluation_contract_validates_datasets_metrics_resume_and_redacted_reports() {
    let dataset = EvalDatasetRef {
        dataset_ref: "dataset".into(),
        schema_ref: "schema".into(),
        version_hash: "version".into(),
        sample_count: 100,
        immutable: true,
    };
    let metric = EvalMetricResult {
        metric_ref: "accuracy".into(),
        version: "v1".into(),
        aggregate_score_micros: 950_000,
        per_sample_result_ref: Some("sample-results".into()),
        threshold_passed: true,
    };
    let definition = EvalDefinition {
        eval_ref: "eval".into(),
        dataset: dataset.clone(),
        graders: vec![EvalGrader {
            grader_ref: "grader".into(),
            grader_kind: "reference_metric".into(),
            policy_scope: "ai.eval.run".into(),
            version_hash: "grader-version".into(),
        }],
        metric_refs: BTreeSet::from(["accuracy".into()]),
        visibility: "tenant".into(),
    };
    let run = EvalRun {
        run_ref: "run".into(),
        eval_ref: "eval".into(),
        state: "checkpointed".into(),
        checkpoint_ref: Some("checkpoint".into()),
        completed_sample_count: 40,
    };
    let comparison = EvalComparison {
        comparison_ref: "comparison".into(),
        baseline_run_ref: "baseline".into(),
        candidate_run_ref: "candidate".into(),
        metric_results: vec![metric.clone()],
    };
    let report = EvalReport {
        report_ref: "report".into(),
        run_ref: "run".into(),
        artifact_ref: "artifact".into(),
        redaction_profile: "redacted".into(),
        bounded_summary_ref: "summary".into(),
    };

    assert!(dataset.is_immutable(1_000));
    assert!(definition.is_bounded(4, 4));
    assert!(metric.is_versioned());
    assert!(comparison.is_comparable());
    assert!(run.can_resume(&dataset));
    assert!(report.is_redacted());
    let serialized = serde_json::to_string(&report).unwrap();
    assert!(!serialized.contains("raw_prompt"));
    assert!(!serialized.contains("raw_output"));
    assert!(!serialized.contains("provider_payload"));

    let mutable_dataset = EvalDatasetRef {
        immutable: false,
        ..dataset
    };
    assert!(!mutable_dataset.is_immutable(1_000));
}
