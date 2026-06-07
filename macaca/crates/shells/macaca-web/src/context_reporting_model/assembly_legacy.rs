//! Legacy local context assembly path (deprecated fallback).
//!
//! Used when Context Service scope/command validation fails or the service
//! returns an error. Retained until the service path is universally available.

use std::sync::Arc;

use macaca_context::{
    assemble_context_providers, ContextAssembleInput, ContextFacade, ContextFacadeAssemblyPolicy,
    ProviderAssemblyEnvironment, ProviderFactoryInput,
};
use macaca_framework::model::ChatOptions;

use crate::context_message_codec::{
    framework_messages_to_llm, framework_options_to_llm,
};

use super::ContextReportingChatModel;

/// Run local context facade assembly and emit a persisted report.
#[allow(deprecated)]
pub(super) async fn assemble_and_emit_report_legacy_local(
    model: &ContextReportingChatModel,
    messages: &[serde_json::Value],
    options: &ChatOptions,
) -> Option<(Vec<serde_json::Value>, ChatOptions)> {
    let Some(session_id) = model.session_id.as_deref() else {
        return None;
    };
    let chat_model = options.model.clone().unwrap_or_default();
    let input = ContextAssembleInput {
        app_id: Some(model.app_id),
        session_id: Some(session_id.to_string()),
        agent_name: model.agent_name.clone(),
        model: chat_model.clone(),
        base_messages: framework_messages_to_llm(messages),
        options: framework_options_to_llm(options),
        budget: model.context_budget,
    };
    let lineage_count = model.lineage_compactions(session_id).await;
    let env = ProviderAssemblyEnvironment {
        agent_profile_root: model.agent_profile_root.clone(),
        agent_profile: model.agent_profile.clone(),
        skill_capability_catalog: Some(Arc::clone(&model.skill_capability_catalog)),
        mcp_capability_catalog: Some(Arc::clone(&model.mcp_capability_catalog)),
        runtime_tool_capability_catalog: Some(Arc::clone(
            &model.runtime_tool_capability_catalog,
        )),
        tool_capability_index: None,
        ready_mcp_server_ids: Some(Arc::clone(&model.ready_mcp_server_ids)),
        memory_recall_capability: model.memory_recall_capability.clone(),
        active_vector_memory: model.active_vector_memory.clone(),
        knowledge_digest_capability: model.knowledge_digest_capability.clone(),
        knowledge_digest: model.context_config.knowledge_digest.clone(),
    };
    let factory_input = ProviderFactoryInput {
        agent_name: model.agent_name.clone(),
        session_id: Some(session_id.to_string()),
        params: serde_json::json!({}),
    };
    let (providers, catalog_notes) = match assemble_context_providers(
        &model.context_config,
        &env,
        &factory_input,
        None,
    )
    .await
    {
        Ok(p) => p,
        Err(error) => {
            tracing::warn!(
                agent = %model.agent_name,
                error = %error,
                command = "context.assemble.legacy",
                "catalog assembly failed before context facade"
            );
            return None;
        }
    };
    let policy = ContextFacadeAssemblyPolicy::from_context_config_parts(
        model.context_config.governance.clone(),
        model.context_config.trust_governance.clone(),
        model.context_config.knowledge_digest.clone(),
    );
    match ContextFacade::builtins_with_engine_overlay(
        model.context_selection.clone(),
        &model.context_engine_registry,
    )
    .assemble_model_context(input, &providers, policy)
    .await
    {
        Ok(mut assembled) => {
            assembled.report.decisions.extend(catalog_notes);
            if let (Some(ledger), Some(summary)) = (
                model.provider_health_ledger.as_ref(),
                assembled.report.provider_runtime.as_ref(),
            ) {
                ledger.record_provider_runtime_summary(summary);
            }
            let output = model
                .finalize_assembled_context(
                    session_id,
                    &chat_model,
                    messages,
                    options,
                    &mut assembled,
                    lineage_count,
                )
                .await;
            Some(output)
        }
        Err(error) => {
            tracing::warn!(
                agent = %model.agent_name,
                error = %error,
                command = "context.assemble.legacy",
                "failed to assemble legacy context report"
            );
            None
        }
    }
}
