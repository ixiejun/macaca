use macaca_proto::domain_pack_contract::{
    ai_embedding::{AI_EMBEDDING_PACK_ID, AI_EMBEDDING_SERVICE_ID},
    ai_llm::{AI_LLM_PACK_ID, AI_LLM_SERVICE_ID},
    ai_model_evaluation::{AI_MODEL_EVALUATION_PACK_ID, AI_MODEL_EVALUATION_SERVICE_ID},
    ai_rerank::{AI_RERANK_PACK_ID, AI_RERANK_SERVICE_ID},
    ai_speech::{AI_SPEECH_PACK_ID, AI_SPEECH_SERVICE_ID},
    ai_vision::{AI_VISION_PACK_ID, AI_VISION_SERVICE_ID},
};
use macaca_proto::{
    compose_installed_domain_pack_catalog, reference_domain_pack_definitions,
    AppServiceContractConfig,
};

use super::*;

// AI SDK tests validate catalog discovery only. The SDK must not create hosted
// model, local runtime, remote service, OCR, speech, embedding, rerank,
// evaluation, plugin, mock, or unavailable providers; it only reports
// provider-neutral descriptors and explicit unavailable diagnostics.

#[tokio::test]
async fn catalog_client_discovers_ai_contract_metadata() {
    let catalog = compose_installed_domain_pack_catalog(reference_domain_pack_definitions());
    let client = CatalogBackedDomainPackClient::new(catalog);

    let cases = [
        (
            AI_LLM_PACK_ID,
            AI_LLM_SERVICE_ID,
            "llm.chat",
            "ai_llm_provider_not_installed",
            "hosted-model",
        ),
        (
            AI_EMBEDDING_PACK_ID,
            AI_EMBEDDING_SERVICE_ID,
            "embedding.batch_embed",
            "ai_embedding_provider_not_installed",
            "local-runtime",
        ),
        (
            AI_RERANK_PACK_ID,
            AI_RERANK_SERVICE_ID,
            "rerank.explain_scores",
            "ai_rerank_provider_not_installed",
            "plugin",
        ),
        (
            AI_VISION_PACK_ID,
            AI_VISION_SERVICE_ID,
            "vision.ocr",
            "ai_vision_provider_not_installed",
            "moderation-service",
        ),
        (
            AI_SPEECH_PACK_ID,
            AI_SPEECH_SERVICE_ID,
            "speech.text_to_speech",
            "ai_speech_provider_not_installed",
            "speech-synthesis",
        ),
        (
            AI_MODEL_EVALUATION_PACK_ID,
            AI_MODEL_EVALUATION_SERVICE_ID,
            "model_evaluation.run_eval",
            "ai_model_evaluation_provider_not_installed",
            "eval-runner",
        ),
    ];

    for (pack_id, service_id, command, unavailable_reason, provider_class) in cases {
        let inspect = client
            .inspect_pack(&DomainPackInspectCommand::new(pack_id).expect("valid ai id"))
            .await
            .unwrap();
        let pack = inspect.pack.expect("ai descriptor exists");

        assert!(!pack.is_callable());
        assert_eq!(
            pack.metadata.diagnostics.unavailable_reason,
            unavailable_reason
        );
        assert!(pack
            .metadata
            .service_command_schemas
            .get(service_id)
            .is_some_and(|commands| commands.contains(command)));
        assert!(pack
            .metadata
            .provider_descriptors
            .contains_key(provider_class));
        assert!(pack.metadata.sdk.docs_url.contains("developer-packs/ai"));
    }
}

#[tokio::test]
async fn catalog_client_reports_ai_unavailable_reasons() {
    let catalog = compose_installed_domain_pack_catalog(reference_domain_pack_definitions());
    let client = CatalogBackedDomainPackClient::new(catalog);
    let command = DomainPackResolveCommand {
        declaration: AppServiceContractConfig {
            optional_packs: vec![
                AI_LLM_PACK_ID.into(),
                AI_EMBEDDING_PACK_ID.into(),
                AI_RERANK_PACK_ID.into(),
                AI_VISION_PACK_ID.into(),
                AI_SPEECH_PACK_ID.into(),
                AI_MODEL_EVALUATION_PACK_ID.into(),
            ],
            ..Default::default()
        },
    };

    let result = client.resolve_declaration(&command).await.unwrap();

    for (pack_id, reason) in [
        (AI_LLM_PACK_ID, "ai_llm_provider_not_installed"),
        (AI_EMBEDDING_PACK_ID, "ai_embedding_provider_not_installed"),
        (AI_RERANK_PACK_ID, "ai_rerank_provider_not_installed"),
        (AI_VISION_PACK_ID, "ai_vision_provider_not_installed"),
        (AI_SPEECH_PACK_ID, "ai_speech_provider_not_installed"),
        (
            AI_MODEL_EVALUATION_PACK_ID,
            "ai_model_evaluation_provider_not_installed",
        ),
    ] {
        assert!(result
            .effective
            .unresolved_optional_packs
            .contains(&pack_id.to_string()));
        assert_eq!(
            result.effective.unavailable_pack_reasons.get(pack_id),
            Some(&reason.to_string())
        );
    }
}
