//! Context Service assembly path (preferred).
//!
//! Builds a serializable assembly snapshot so the runtime-host provider can
//! rebuild the provider chain without depending on Web shell types.

use macaca_sdk::context::{
    ContextAssembleCommand, ContextAssemblySnapshot, ContextServiceScope,
};
use macaca_sdk::framework::model::ChatOptions;

use crate::context_message_codec::{framework_messages_to_llm, framework_options_to_llm};

use super::ContextReportingChatModel;

/// Assemble context via SystemContextClient, then apply Web-owned recall + persistence.
pub(super) async fn assemble_and_emit_report(
    model: &ContextReportingChatModel,
    messages: &[serde_json::Value],
    options: &ChatOptions,
) -> Option<(Vec<serde_json::Value>, ChatOptions)> {
    let Some(session_id) = model.session_id.as_deref() else {
        return None;
    };
    let chat_model = options.model.clone().unwrap_or_default();
    let scope = match ContextServiceScope::new(
        model.app_id,
        session_id.to_string(),
        model.agent_name.clone(),
    ) {
        Ok(scope) => scope,
        Err(error) => {
            tracing::warn!(
                agent = %model.agent_name,
                error = %error,
                command = "context.assemble.service",
                "context service scope validation failed"
            );
            #[allow(deprecated)]
            return model
                .assemble_and_emit_report_legacy_local(messages, options)
                .await;
        }
    };
    let mut trace = macaca_proto::TraceContext::new(uuid::Uuid::new_v4().to_string());
    trace.session_id = Some(session_id.to_string());
    trace.agent = Some(model.agent_name.clone());
    let mut command = match ContextAssembleCommand::new(
        scope,
        trace,
        chat_model.clone(),
        framework_messages_to_llm(messages),
        framework_options_to_llm(options),
        model.context_selection.clone(),
    ) {
        Ok(command) => command,
        Err(error) => {
            tracing::warn!(
                agent = %model.agent_name,
                error = %error,
                command = "context.assemble.service",
                "context service assemble command validation failed"
            );
            #[allow(deprecated)]
            return model
                .assemble_and_emit_report_legacy_local(messages, options)
                .await;
        }
    };
    command.budget = model.context_budget;
    command.assembly = Some(ContextAssemblySnapshot {
        context_config: model.context_config.clone(),
        agent_profile_root: model.agent_profile_root.clone(),
        skill_capability_catalog: (*model.skill_capability_catalog).clone(),
        mcp_capability_catalog: (*model.mcp_capability_catalog).clone(),
        runtime_tool_capability_catalog: (*model.runtime_tool_capability_catalog).clone(),
        ready_mcp_server_ids: (*model.ready_mcp_server_ids).clone(),
        factory_params: serde_json::json!({}),
    });
    let lineage_count = model.lineage_compactions(session_id).await;
    let mut assembled = match model.context_client.assemble(command).await {
        Ok(result) => result.assembled,
        Err(error) => {
            tracing::warn!(
                agent = %model.agent_name,
                error = %error,
                command = "context.assemble.service",
                "context service assembly failed; falling back to deprecated local assembler"
            );
            #[allow(deprecated)]
            return model
                .assemble_and_emit_report_legacy_local(messages, options)
                .await;
        }
    };
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
