//! Context-reporting ChatModel wrapper (Facade module).
//!
//! Bridges framework-level model calls to the context-engine runtime: assembly,
//! optional scoped memory recall, report persistence, then delegation to the
//! inner chat model.

mod assembly_finalize;
mod assembly_legacy;
mod assembly_service;

#[cfg(test)]
pub(crate) mod contract_source;
#[cfg(test)]
mod tests;

use std::sync::Arc;

use async_trait::async_trait;
use macaca_sdk::context::{
    ActiveRecallCapability, ContextBudget, ContextEngineRegistry, ContextEngineSelection,
    ContextPreflightRecallConfig, KnowledgeDigestCapability, McpCapabilityCatalog,
    ProviderHealthLedger, RuntimeToolCapabilityCatalog, SkillCapabilityCatalog,
};
use macaca_sdk::framework::model::{ChatModel, ChatOptions, ChatResponse, ModelError};
use macaca_proto::{config::ContextConfig, AgentId, ApplicationId};
use macaca_runtime_host::persist::EventLog;
use macaca_sdk::memory::SharedTombstoneRegistry;

use crate::context_reporting_memory::{
    build_workspace_knowledge_digest_capability, build_workspace_recall_capability,
};

/// ChatModel wrapper that injects context-engine assembly and persists context reports.
///
/// See module-level docs for the full pipeline. Capability catalogs are frozen snapshots
/// built when the framework agent is constructed (progressive disclosure).
pub(crate) struct ContextReportingChatModel {
    pub(super) inner: Arc<dyn ChatModel>,
    pub(super) event_log: Arc<EventLog>,
    pub(super) persist_backend: Arc<dyn macaca_runtime_host::persist::PersistBackend>,
    pub(super) app_id: ApplicationId,
    pub(super) session_id: Option<String>,
    pub(super) agent_name: String,
    pub(super) context_selection: ContextEngineSelection,
    pub(super) context_budget: ContextBudget,
    pub(super) recall_runtime: macaca_proto::config::ContextRecallRuntimeConfig,
    pub(super) context_client: Arc<dyn macaca_sdk::SystemContextClient>,
    pub(super) memory_client: Arc<dyn macaca_sdk::SystemMemoryClient>,
    pub(super) agent_profile: macaca_proto::config::AgentProfileContextConfig,
    pub(super) agent_profile_root: Option<std::path::PathBuf>,
    pub(super) active_vector_memory: macaca_proto::config::ActiveVectorMemoryContextConfig,
    pub(super) memory_recall_capability: Option<Arc<dyn ActiveRecallCapability>>,
    pub(super) knowledge_digest_capability: Option<Arc<dyn KnowledgeDigestCapability>>,
    pub(super) skill_capability_catalog: Arc<SkillCapabilityCatalog>,
    pub(super) mcp_capability_catalog: Arc<McpCapabilityCatalog>,
    pub(super) runtime_tool_capability_catalog: Arc<RuntimeToolCapabilityCatalog>,
    pub(super) ready_mcp_server_ids: Arc<Vec<String>>,
    pub(super) context_config: ContextConfig,
    pub(super) provider_health_ledger: Option<Arc<ProviderHealthLedger>>,
    pub(super) context_engine_registry: Arc<ContextEngineRegistry>,
}

impl ContextReportingChatModel {
    #[allow(clippy::too_many_arguments)]
    /// Build a reporting wrapper from merged app/runtime context configuration.
    pub(crate) fn new(
        inner: Arc<dyn ChatModel>,
        event_log: Arc<EventLog>,
        persist_backend: Arc<dyn macaca_runtime_host::persist::PersistBackend>,
        app_id: ApplicationId,
        session_id: Option<String>,
        agent_name: String,
        merged_context_config: ContextConfig,
        agent_profile_root: Option<std::path::PathBuf>,
        context_client: Arc<dyn macaca_sdk::SystemContextClient>,
        memory_client: Arc<dyn macaca_sdk::SystemMemoryClient>,
        workspace_memory_tombstones: Option<Arc<SharedTombstoneRegistry>>,
        routing_agent_id: Option<AgentId>,
        skill_capability_catalog: Arc<SkillCapabilityCatalog>,
        mcp_capability_catalog: Arc<McpCapabilityCatalog>,
        runtime_tool_capability_catalog: Arc<RuntimeToolCapabilityCatalog>,
        ready_mcp_server_ids: Arc<Vec<String>>,
        provider_health_ledger: Option<Arc<ProviderHealthLedger>>,
        context_engine_registry: Arc<ContextEngineRegistry>,
    ) -> Self {
        let memory_recall_capability = build_workspace_recall_capability(
            &merged_context_config,
            Arc::clone(&memory_client),
            workspace_memory_tombstones.as_ref(),
            app_id,
            session_id.as_deref(),
            routing_agent_id,
        );
        let knowledge_digest_capability = build_workspace_knowledge_digest_capability(
            &merged_context_config,
            Arc::clone(&memory_client),
            workspace_memory_tombstones.as_ref(),
            app_id,
            session_id.as_deref(),
        );
        Self {
            inner,
            event_log,
            persist_backend,
            app_id,
            session_id,
            agent_name,
            context_selection: ContextEngineSelection {
                engine_id: merged_context_config.default_engine.clone(),
                fallback_engine_id: merged_context_config.fallback_engine.clone(),
            },
            context_budget: ContextBudget::new(
                merged_context_config.max_tokens,
                merged_context_config.reserve_output_tokens,
            ),
            recall_runtime: merged_context_config.recall.clone(),
            context_client,
            memory_client,
            agent_profile: merged_context_config.agent_profile.clone(),
            agent_profile_root,
            active_vector_memory: merged_context_config.active_vector_memory.clone(),
            memory_recall_capability,
            knowledge_digest_capability,
            skill_capability_catalog,
            mcp_capability_catalog,
            runtime_tool_capability_catalog,
            ready_mcp_server_ids,
            context_config: merged_context_config.clone(),
            provider_health_ledger,
            context_engine_registry,
        }
    }

    /// True when active recall already ran through the composer pipeline for this configuration.
    pub(crate) fn composer_handles_active_vector_recall(&self) -> bool {
        self.active_vector_memory.enabled && self.memory_recall_capability.is_some()
    }

    /// Convert persisted config into the preflight recall runtime config.
    pub(super) fn preflight_config(&self) -> ContextPreflightRecallConfig {
        ContextPreflightRecallConfig {
            enabled: self.recall_runtime.preflight_recall_enabled,
            allowed_tool_names: self.recall_runtime.preflight_allowed_tools.clone(),
            timeout_ms: self.recall_runtime.preflight_timeout_ms,
            max_chars: self.recall_runtime.preflight_max_chars,
            max_tokens: self.recall_runtime.preflight_max_tokens,
            fatal_on_failure: self.recall_runtime.preflight_fatal_on_failure,
        }
    }

    /// Build the Web-scoped active recall fallback policy for one model call.
    pub(crate) fn active_recall_fallback_config(
        active_vector_memory: &macaca_proto::config::ActiveVectorMemoryContextConfig,
        preflight_cfg: &ContextPreflightRecallConfig,
        composer_active_recall_proven: bool,
    ) -> ContextPreflightRecallConfig {
        if !active_vector_memory.enabled || composer_active_recall_proven {
            return preflight_cfg.clone();
        }
        ContextPreflightRecallConfig {
            enabled: true,
            allowed_tool_names: preflight_cfg.allowed_tool_names.clone(),
            timeout_ms: active_vector_memory.timeout_ms,
            max_chars: active_vector_memory.max_chars,
            max_tokens: active_vector_memory.max_tokens,
            fatal_on_failure: false,
        }
    }

    /// Count compaction successors for diagnostic lineage enrichment.
    pub(super) async fn lineage_compactions(&self, session_id: &str) -> u32 {
        let store =
            macaca_runtime_host::persist::SessionLineageStore::new(Arc::clone(&self.persist_backend));
        store
            .count_compaction_successors(session_id)
            .await
            .unwrap_or(0)
    }

    /// Run context assembly through Context Service (preferred path).
    pub(super) async fn assemble_and_emit_report(
        &self,
        messages: &[serde_json::Value],
        options: &ChatOptions,
    ) -> Option<(Vec<serde_json::Value>, ChatOptions)> {
        assembly_service::assemble_and_emit_report(self, messages, options).await
    }

    /// Deprecated local assembler fallback when Context Service is unavailable.
    #[deprecated(note = "Use Context Service assembly via SystemContextClient for new code")]
    pub(super) async fn assemble_and_emit_report_legacy_local(
        &self,
        messages: &[serde_json::Value],
        options: &ChatOptions,
    ) -> Option<(Vec<serde_json::Value>, ChatOptions)> {
        assembly_legacy::assemble_and_emit_report_legacy_local(self, messages, options).await
    }
}

#[async_trait]
impl ChatModel for ContextReportingChatModel {
    /// Run context assembly before delegating the final call to the wrapped model.
    async fn chat(
        &self,
        messages: Vec<serde_json::Value>,
        options: &ChatOptions,
    ) -> Result<ChatResponse, ModelError> {
        if let Some((assembled_messages, assembled_options)) =
            self.assemble_and_emit_report(&messages, &options).await
        {
            self.inner
                .chat(assembled_messages, &assembled_options)
                .await
        } else {
            self.inner.chat(messages, options).await
        }
    }

    fn name(&self) -> &str {
        self.inner.name()
    }
}
