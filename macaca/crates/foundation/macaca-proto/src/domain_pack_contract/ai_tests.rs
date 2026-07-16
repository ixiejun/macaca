use std::collections::{BTreeMap, BTreeSet};

use super::ai_common::{AiPackCommandEnvelope, AiPackError};
use super::ai_embedding::*;
use super::ai_llm::*;
use super::ai_model_evaluation::*;
use super::ai_rerank::*;
use super::ai_speech::*;
use super::ai_vision::*;
use super::*;

// AI tests validate provider-neutral contract shape only. They do not contact
// hosted model, local runtime, remote service, OCR, speech, embedding, rerank,
// evaluation, plugin, or provider APIs. Fixtures use references and hashes
// instead of raw prompts, outputs, media, audio, vectors, datasets, model names,
// credentials, provider payloads, or unbounded diagnostic data.

#[test]
fn ai_descriptors_are_discoverable_and_not_callable() {
    let cases = [
        (
            ai_llm_pack_definition(),
            AI_LLM_PACK_ID,
            AI_LLM_SERVICE_ID,
            AI_LLM_COMMANDS,
            "ai_llm_provider_not_installed",
            "hosted-model",
            "llm.chat",
        ),
        (
            ai_embedding_pack_definition(),
            AI_EMBEDDING_PACK_ID,
            AI_EMBEDDING_SERVICE_ID,
            AI_EMBEDDING_COMMANDS,
            "ai_embedding_provider_not_installed",
            "local-runtime",
            "embedding.batch_embed",
        ),
        (
            ai_rerank_pack_definition(),
            AI_RERANK_PACK_ID,
            AI_RERANK_SERVICE_ID,
            AI_RERANK_COMMANDS,
            "ai_rerank_provider_not_installed",
            "plugin",
            "rerank.explain_scores",
        ),
        (
            ai_vision_pack_definition(),
            AI_VISION_PACK_ID,
            AI_VISION_SERVICE_ID,
            AI_VISION_COMMANDS,
            "ai_vision_provider_not_installed",
            "moderation-service",
            "vision.ocr",
        ),
        (
            ai_speech_pack_definition(),
            AI_SPEECH_PACK_ID,
            AI_SPEECH_SERVICE_ID,
            AI_SPEECH_COMMANDS,
            "ai_speech_provider_not_installed",
            "speech-synthesis",
            "speech.text_to_speech",
        ),
        (
            ai_model_evaluation_pack_definition(),
            AI_MODEL_EVALUATION_PACK_ID,
            AI_MODEL_EVALUATION_SERVICE_ID,
            AI_MODEL_EVALUATION_COMMANDS,
            "ai_model_evaluation_provider_not_installed",
            "eval-runner",
            "model_evaluation.run_eval",
        ),
    ];

    for (definition, pack_id, service_id, commands, unavailable_reason, provider_class, command) in
        cases
    {
        assert_eq!(definition.pack_id, pack_id);
        assert!(!definition.is_callable());
        assert!(DomainPackDefinitionSpec.validate(&definition).is_ok());
        assert_eq!(
            definition.metadata.parent_pack_id.as_deref(),
            Some("pack.ai.v1")
        );
        assert_eq!(
            definition.metadata.diagnostics.unavailable_reason,
            unavailable_reason
        );
        assert!(definition
            .metadata
            .sdk
            .docs_url
            .contains("developer-packs/ai"));
        assert!(definition
            .metadata
            .provider_descriptors
            .contains_key(provider_class));
        assert!(definition
            .metadata
            .service_command_schemas
            .get(service_id)
            .is_some_and(|schemas| schemas.contains(command)));

        let descriptor_commands = definition
            .metadata
            .service_command_schemas
            .get(service_id)
            .expect("ai descriptor exposes command schemas");
        for expected in commands {
            assert!(
                descriptor_commands.contains(*expected),
                "missing command {expected}"
            );
        }
    }
}

#[test]
fn industrial_catalog_uses_specialized_ai_descriptors() {
    let definitions = industrial_reference_domain_pack_definitions();
    let llm = find_pack(&definitions, AI_LLM_PACK_ID);
    let embedding = find_pack(&definitions, AI_EMBEDDING_PACK_ID);
    let rerank = find_pack(&definitions, AI_RERANK_PACK_ID);
    let vision = find_pack(&definitions, AI_VISION_PACK_ID);
    let speech = find_pack(&definitions, AI_SPEECH_PACK_ID);
    let evaluation = find_pack(&definitions, AI_MODEL_EVALUATION_PACK_ID);

    assert_eq!(
        llm.metadata
            .provider_descriptors
            .get("hosted-model")
            .and_then(|descriptor| descriptor.metadata.get("raw_prompts_in_trace"))
            .map(String::as_str),
        Some("false")
    );
    assert_eq!(
        embedding
            .metadata
            .provider_descriptors
            .get("hosted-model")
            .and_then(|descriptor| descriptor.metadata.get("raw_vectors_in_trace"))
            .map(String::as_str),
        Some("false")
    );
    assert_eq!(
        rerank
            .metadata
            .provider_descriptors
            .get("hosted-model")
            .and_then(|descriptor| descriptor.metadata.get("raw_candidates_in_trace"))
            .map(String::as_str),
        Some("false")
    );
    assert_eq!(
        vision
            .metadata
            .provider_descriptors
            .get("hosted-model")
            .and_then(|descriptor| descriptor.metadata.get("raw_pixels_in_trace"))
            .map(String::as_str),
        Some("false")
    );
    assert_eq!(
        speech
            .metadata
            .provider_descriptors
            .get("speech-recognition")
            .and_then(|descriptor| descriptor.metadata.get("raw_audio_in_trace"))
            .map(String::as_str),
        Some("false")
    );
    assert_eq!(
        evaluation
            .metadata
            .provider_descriptors
            .get("eval-runner")
            .and_then(|descriptor| descriptor.metadata.get("raw_outputs_in_trace"))
            .map(String::as_str),
        Some("false")
    );
}

#[test]
fn ai_required_and_optional_pack_admission_reports_unavailable_descriptors() {
    let catalog =
        compose_installed_domain_pack_catalog(industrial_reference_domain_pack_definitions());
    let cases = [
        AI_LLM_PACK_ID,
        AI_EMBEDDING_PACK_ID,
        AI_RERANK_PACK_ID,
        AI_VISION_PACK_ID,
        AI_SPEECH_PACK_ID,
        AI_MODEL_EVALUATION_PACK_ID,
    ];

    for pack_id in cases {
        let required_declaration = AppServiceContractConfig {
            required_packs: vec![pack_id.into()],
            ..Default::default()
        };
        let required = expand_service_capabilities(Some(&required_declaration), catalog.as_ref());
        assert_eq!(
            required.unresolved_required_packs,
            vec![pack_id.to_string()],
            "{pack_id} required declaration should block admission while unavailable"
        );
        assert!(required.resolved_packs.is_empty());
        assert!(required.services.is_empty());
        assert!(
            required.unavailable_pack_reasons.contains_key(pack_id),
            "{pack_id} required declaration should carry unavailable diagnostics"
        );

        let optional_declaration = AppServiceContractConfig {
            optional_packs: vec![pack_id.into()],
            ..Default::default()
        };
        let optional = expand_service_capabilities(Some(&optional_declaration), catalog.as_ref());
        assert_eq!(
            optional.unresolved_optional_packs,
            vec![pack_id.to_string()],
            "{pack_id} optional declaration should degrade explicitly while unavailable"
        );
        assert!(optional.resolved_packs.is_empty());
        assert!(optional.services.is_empty());
        assert!(
            optional.unavailable_pack_reasons.contains_key(pack_id),
            "{pack_id} optional declaration should carry unavailable diagnostics"
        );
    }
}

#[test]
fn ai_command_and_result_dtos_are_serde_compatible() {
    let envelope = AiPackCommandEnvelope {
        subject_ref: "ai:subject".into(),
        parameters: BTreeMap::from([("mode".into(), "synthetic".into())]),
        cursor: None,
        page_size: Some(10),
        idempotency_key: Some("idem-ai".into()),
    };

    let values = [
        serde_json::to_value(LlmChatCommand {
            request: envelope.clone(),
        })
        .unwrap(),
        serde_json::to_value(EmbeddingBatchEmbedCommand {
            request: envelope.clone(),
        })
        .unwrap(),
        serde_json::to_value(RerankExplainScoresCommand {
            request: envelope.clone(),
        })
        .unwrap(),
        serde_json::to_value(VisionOcrCommand {
            request: envelope.clone(),
        })
        .unwrap(),
        serde_json::to_value(SpeechTextToSpeechCommand {
            request: envelope.clone(),
        })
        .unwrap(),
        serde_json::to_value(ModelEvaluationRunEvalCommand { request: envelope }).unwrap(),
        serde_json::to_value(LlmResultEnvelope::<LlmGeneration> {
            status: LlmResultStatus::SchemaMismatch,
            data: None,
            page: None,
            error: Some(AiPackError {
                code: "schema_mismatch".into(),
                message: "synthetic schema mismatch".into(),
                retryable: false,
                trace_safe_detail: Some("schema_ref".into()),
            }),
        })
        .unwrap(),
        serde_json::to_value(EmbeddingResultEnvelope::<EmbeddingVector> {
            status: EmbeddingResultStatus::DimensionMismatch,
            data: None,
            page: None,
            error: None,
        })
        .unwrap(),
        serde_json::to_value(RerankResultEnvelope::<RerankResult> {
            status: RerankResultStatus::ExplanationUnavailable,
            data: None,
            page: None,
            error: None,
        })
        .unwrap(),
        serde_json::to_value(VisionResultEnvelope::<VisualEvidenceRef> {
            status: VisionResultStatus::ModerationBlocked,
            data: None,
            page: None,
            error: None,
        })
        .unwrap(),
        serde_json::to_value(SpeechResultEnvelope::<TranscriptSegment> {
            status: SpeechResultStatus::VoiceUnsupported,
            data: None,
            page: None,
            error: None,
        })
        .unwrap(),
        serde_json::to_value(ModelEvaluationResultEnvelope::<EvalRun> {
            status: ModelEvaluationResultStatus::RunInterrupted,
            data: None,
            page: None,
            error: None,
        })
        .unwrap(),
    ];

    assert!(values.iter().all(|value| value.is_object()));
}

#[test]
fn ai_descriptor_hashes_are_stable_and_distinct() {
    let hash_groups = [
        hash_values(&ai_llm_descriptor_hashes()),
        hash_values(&ai_embedding_descriptor_hashes()),
        hash_values(&ai_rerank_descriptor_hashes()),
        hash_values(&ai_vision_descriptor_hashes()),
        hash_values(&ai_speech_descriptor_hashes()),
        hash_values(&ai_model_evaluation_descriptor_hashes()),
    ];

    for hashes in hash_groups {
        let unique = hashes.into_iter().collect::<BTreeSet<_>>();
        assert!(unique.len() >= 7);
        assert!(unique.iter().all(|hash| !hash.is_empty()));
    }
}

#[test]
fn ai_validation_helpers_are_provider_neutral() {
    let llm = LlmInvocation {
        messages: vec![LlmMessage {
            message_ref: "message".into(),
            ..Default::default()
        }],
        budget: LlmBudgetEnvelope {
            max_input_tokens: 100,
            max_output_tokens: 100,
            max_cost_micros: 10,
            retained_output_bytes: 1_024,
        },
        ..Default::default()
    };
    let embedding = EmbeddingBatchRequest {
        inputs: vec![EmbeddingInput {
            input_ref: "input".into(),
            modality: "text".into(),
            content_ref: "content-ref".into(),
            content_hash: "content-hash".into(),
            truncation_policy: "bounded".into(),
            ..Default::default()
        }],
        schema: VectorSchemaDescriptor {
            schema_ref: "schema".into(),
            dimension: 384,
            numeric_type: "float32".into(),
            metric: "cosine".into(),
            normalization: "unit".into(),
            ..Default::default()
        },
        idempotency_key: "idem".into(),
        ..Default::default()
    };
    let rerank = RerankRequest {
        request_ref: "request".into(),
        query: RerankQuery {
            query_ref: "query".into(),
            text_ref: "query-ref".into(),
            query_hash: "query-hash".into(),
            ..Default::default()
        },
        candidates: vec![RerankCandidate {
            candidate_ref: "candidate".into(),
            content_ref: "content-ref".into(),
            content_hash: "content-hash".into(),
            hidden: false,
            ..Default::default()
        }],
        top_n: 1,
        score_normalization: "unit".into(),
        ..Default::default()
    };
    let region = VisualRegion {
        region_ref: "region".into(),
        coordinate_space: "normalized".into(),
        width_micros: 100,
        height_micros: 100,
        ..Default::default()
    };
    let speech = SpeechAudioInput {
        audio_ref: "audio".into(),
        codec: "opus".into(),
        duration_ms: 1_000,
        content_hash: "audio-hash".into(),
        redaction_class: "private".into(),
        ..Default::default()
    };
    let eval = EvalDefinition {
        eval_ref: "eval".into(),
        dataset: EvalDatasetRef {
            dataset_ref: "dataset".into(),
            schema_ref: "schema".into(),
            version_hash: "dataset-version".into(),
            sample_count: 10,
            immutable: true,
            ..Default::default()
        },
        graders: vec![EvalGrader {
            grader_ref: "grader".into(),
            grader_kind: "reference_metric".into(),
            policy_scope: "ai.eval.run".into(),
            version_hash: "grader-version".into(),
            ..Default::default()
        }],
        metric_refs: BTreeSet::from(["metric".into()]),
        visibility: "tenant".into(),
        ..Default::default()
    };

    assert!(llm.is_bounded(4, 2));
    assert!(embedding.is_bounded(10));
    assert!(rerank.is_bounded(10));
    assert!(region.is_normalized());
    assert!(speech.is_bounded(2_000));
    assert!(eval.is_bounded(4, 4));
}

#[test]
fn invalid_ai_descriptor_is_rejected() {
    let mut invalid = ai_llm_pack_definition();
    invalid.pack_id = "ai.llm.v1".into();

    assert!(DomainPackDefinitionSpec.validate(&invalid).is_err());
}

fn hash_values<T: serde::Serialize>(value: &T) -> Vec<String> {
    let json = serde_json::to_value(value).expect("descriptor hash fixture serializes");
    json.as_object()
        .expect("descriptor hashes serialize as object")
        .values()
        .filter_map(|value| value.as_str().map(str::to_string))
        .collect()
}

fn find_pack<'a>(
    definitions: &'a [DomainPackDefinition],
    pack_id: &str,
) -> &'a DomainPackDefinition {
    definitions
        .iter()
        .find(|definition| definition.pack_id == pack_id)
        .expect("industrial catalog includes specialized ai descriptor")
}
