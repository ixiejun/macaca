use std::sync::Arc;

use async_trait::async_trait;
use macaca_context::{
    assemble_context_providers, ActiveRecallCapability, ContextAssembleCommand,
    ContextAssembleInput, ContextAssemblySnapshot, ContextBudget, ContextEngineRegistry,
    ContextEngineSelection, ContextFacade, ContextFacadeAssemblyPolicy,
    ContextPreflightRecallConfig, ContextServiceScope, KnowledgeDigestCapability,
    McpCapabilityCatalog, ProviderAssemblyEnvironment, ProviderFactoryInput, ProviderHealthLedger,
    RuntimeToolCapabilityCatalog, SkillCapabilityCatalog,
};
use macaca_framework::model::{ChatModel, ChatOptions, ChatResponse, ModelError};
use macaca_memory::SharedTombstoneRegistry;
use macaca_persist::{EventLog, SessionLineageStore};
use macaca_proto::{config::ContextConfig, AgentId, ApplicationId};

use crate::context_memory_injection::{apply_active_recall, apply_preflight_memory};
use crate::context_message_codec::{
    framework_messages_to_llm, framework_options_to_llm, llm_messages_to_framework,
    llm_options_to_framework,
};
use crate::context_report_events::persist_context_report_events;
use crate::context_reporting_memory::{
    build_workspace_knowledge_digest_capability, build_workspace_recall_capability, digest_scope,
    recall_scope,
};
use crate::source_artifact::persist_pruned_source_artifacts;

/// ChatModel wrapper that injects context-engine assembly and persists context reports.
///
/// This is the main bridge between framework-level model calls and the new
/// context-engine runtime:
/// - convert framework message/options into LLM-neutral DTOs
/// - run the selected context engine
/// - optionally inject preflight memory recall
/// - emit a structured `context_report` event
/// - call the underlying chat model with the assembled prompt
///
/// ## Capability context (skills / MCP / runtime tools)
/// Tier-1 capability indices are supplied as frozen [`SkillCapabilityCatalog`] /
/// [`McpCapabilityCatalog`] / [`RuntimeToolCapabilityCatalog`] snapshots built when the framework
/// agent is constructed. They flow through [`ContextProviderStage::CapabilityIndex`] providers so
/// skill bodies and MCP resource payloads stay out of the default path (progressive disclosure).
pub(crate) struct ContextReportingChatModel {
    inner: Arc<dyn ChatModel>,
    event_log: Arc<EventLog>,
    persist_backend: Arc<dyn macaca_persist::PersistBackend>,
    app_id: ApplicationId,
    session_id: Option<String>,
    agent_name: String,
    context_selection: ContextEngineSelection,
    context_budget: ContextBudget,
    recall_runtime: macaca_proto::config::ContextRecallRuntimeConfig,
    context_client: Arc<dyn macaca_sdk::SystemContextClient>,
    memory_client: Arc<dyn macaca_sdk::SystemMemoryClient>,
    /// Tunables copied from [`ContextConfig::agent_profile`] for hot-path injection checks.
    agent_profile: macaca_proto::config::AgentProfileContextConfig,
    /// Resolved directory that stores `AGENTS.md` / `IDENTITY.md`, depending on [`AgentProfileRootKind`].
    agent_profile_root: Option<std::path::PathBuf>,
    /// Composer-stage vector memory recall (see [`macaca_proto::config::ActiveVectorMemoryContextConfig`]).
    active_vector_memory: macaca_proto::config::ActiveVectorMemoryContextConfig,
    /// Narrow recall capability (built when [`Self::active_vector_memory`] is enabled and backend exists).
    memory_recall_capability: Option<Arc<dyn ActiveRecallCapability>>,
    /// Optional digest compiler bridge (enabled via [`ContextConfig::knowledge_digest`]).
    knowledge_digest_capability: Option<Arc<dyn KnowledgeDigestCapability>>,
    /// Frozen skill catalog rows (metadata only) for [`macaca_context::SkillContextProvider`].
    skill_capability_catalog: Arc<SkillCapabilityCatalog>,
    /// MCP server/tool summaries from probe (compact; no resource bodies).
    mcp_capability_catalog: Arc<McpCapabilityCatalog>,
    /// Framework toolkit registration names only.
    runtime_tool_capability_catalog: Arc<RuntimeToolCapabilityCatalog>,
    /// MCP servers in [`macaca_runtime_host::McpRuntimeStatusState::Ready`] — dependency gap checks only.
    ready_mcp_server_ids: Arc<Vec<String>>,
    /// Merged `ContextConfig` (families, governance, trust governance, engines, recall, …).
    context_config: ContextConfig,
    /// Optional rolling health snapshots (OS-level; no prompt bodies).
    provider_health_ledger: Option<Arc<ProviderHealthLedger>>,
    /// In-process custom context engines overlaid on top of builtin engines.
    context_engine_registry: Arc<ContextEngineRegistry>,
}

impl ContextReportingChatModel {
    #[allow(clippy::too_many_arguments)]
    /// Build a reporting wrapper from merged app/runtime context configuration.
    ///
    /// `routing_agent_id` should match the kernel [`AgentManifest::id`] for `agent_name` when the
    /// agent is registered; it is required for returning agent-scoped memory rows during active recall.
    pub(crate) fn new(
        inner: Arc<dyn ChatModel>,
        event_log: Arc<EventLog>,
        persist_backend: Arc<dyn macaca_persist::PersistBackend>,
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
        // Resolve capability before moving `workspace_memory` into the struct field.
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
    fn composer_handles_active_vector_recall(&self) -> bool {
        self.active_vector_memory.enabled && self.memory_recall_capability.is_some()
    }

    /// Convert persisted config into the preflight recall runtime config.
    fn preflight_config(&self) -> ContextPreflightRecallConfig {
        ContextPreflightRecallConfig {
            enabled: self.recall_runtime.preflight_recall_enabled,
            allowed_tool_names: self.recall_runtime.preflight_allowed_tools.clone(),
            timeout_ms: self.recall_runtime.preflight_timeout_ms,
            max_chars: self.recall_runtime.preflight_max_chars,
            max_tokens: self.recall_runtime.preflight_max_tokens,
            fatal_on_failure: self.recall_runtime.preflight_fatal_on_failure,
        }
    }

    /// Count previously created compaction successors for the current session root.
    ///
    /// This is a diagnostic-only read used to enrich context reports with
    /// lineage information visible to operators and the UI.
    async fn lineage_compactions(&self, session_id: &str) -> u32 {
        let store = SessionLineageStore::new(Arc::clone(&self.persist_backend));
        store
            .count_compaction_successors(session_id)
            .await
            .unwrap_or(0)
    }

    /// Run context assembly, emit a persisted report, and convert back to framework DTOs.
    ///
    /// This is the hot path used before every framework chat call:
    /// - build `ContextAssembleInput`
    /// - assemble via [`ContextFacade`]（Composer → Context engine）
    /// - enrich the report with lineage/preflight details
    /// - persist the report to the session event log
    /// - hand back assembled messages/options in framework-native JSON format
    #[deprecated(note = "Use Context Service assembly via SystemContextClient for new code")]
    async fn assemble_and_emit_report_legacy_local(
        &self,
        messages: &[serde_json::Value],
        options: &ChatOptions,
    ) -> Option<(Vec<serde_json::Value>, ChatOptions)> {
        let Some(session_id) = self.session_id.as_deref() else {
            return None;
        };
        let model = options.model.clone().unwrap_or_default();
        let input = ContextAssembleInput {
            app_id: Some(self.app_id),
            session_id: Some(session_id.to_string()),
            agent_name: self.agent_name.clone(),
            model: model.clone(),
            base_messages: framework_messages_to_llm(messages),
            options: framework_options_to_llm(options),
            budget: self.context_budget,
        };
        let lineage_count = self.lineage_compactions(session_id).await;
        let preflight_cfg = self.preflight_config();
        let env = ProviderAssemblyEnvironment {
            agent_profile_root: self.agent_profile_root.clone(),
            agent_profile: self.agent_profile.clone(),
            skill_capability_catalog: Some(Arc::clone(&self.skill_capability_catalog)),
            mcp_capability_catalog: Some(Arc::clone(&self.mcp_capability_catalog)),
            runtime_tool_capability_catalog: Some(Arc::clone(
                &self.runtime_tool_capability_catalog,
            )),
            ready_mcp_server_ids: Some(Arc::clone(&self.ready_mcp_server_ids)),
            memory_recall_capability: self.memory_recall_capability.clone(),
            active_vector_memory: self.active_vector_memory.clone(),
            knowledge_digest_capability: self.knowledge_digest_capability.clone(),
            knowledge_digest: self.context_config.knowledge_digest.clone(),
        };
        let factory_input = ProviderFactoryInput {
            agent_name: self.agent_name.clone(),
            session_id: Some(session_id.to_string()),
            params: serde_json::json!({}),
        };
        let (providers, catalog_notes) = match assemble_context_providers(
            &self.context_config,
            &env,
            &factory_input,
            None,
        )
        .await
        {
            Ok(p) => p,
            Err(error) => {
                tracing::warn!(
                    agent = %self.agent_name,
                    error = %error,
                    "catalog assembly failed before context facade"
                );
                return None;
            }
        };
        let policy = ContextFacadeAssemblyPolicy::from_context_config_parts(
            self.context_config.governance.clone(),
            self.context_config.trust_governance.clone(),
            self.context_config.knowledge_digest.clone(),
        );
        match ContextFacade::builtins_with_engine_overlay(
            self.context_selection.clone(),
            &self.context_engine_registry,
        )
        .assemble_model_context(input, &providers, policy)
        .await
        {
            Ok(mut assembled) => {
                assembled.report.decisions.extend(catalog_notes);
                if let (Some(ledger), Some(summary)) = (
                    self.provider_health_ledger.as_ref(),
                    assembled.report.provider_runtime.as_ref(),
                ) {
                    ledger.record_provider_runtime_summary(summary);
                }
                assembled.report.lineage_compaction_count = lineage_count;
                let message_count_before_recall = assembled.messages.len();
                apply_active_recall(
                    &self.recall_runtime,
                    &self.memory_client,
                    recall_scope(self.app_id, self.session_id.as_deref(), None),
                    &preflight_cfg,
                    self.composer_handles_active_vector_recall(),
                    &mut assembled,
                    messages,
                )
                .await;
                apply_preflight_memory(
                    &self.recall_runtime,
                    &self.memory_client,
                    digest_scope(self.app_id, self.session_id.as_deref()),
                    &preflight_cfg,
                    &mut assembled,
                    messages,
                )
                .await;
                let recall_injected_messages =
                    assembled.messages.len() != message_count_before_recall;
                let output_messages =
                    if assembled.report.engine_id == "legacy" && !recall_injected_messages {
                        messages.to_vec()
                    } else {
                        llm_messages_to_framework(&assembled.messages)
                    };
                let output_options = llm_options_to_framework(&assembled.options, options);
                let mut report = assembled.report.clone();
                persist_pruned_source_artifacts(
                    &self.event_log,
                    session_id,
                    &self.agent_name,
                    Some(self.app_id.to_string()),
                    &mut report,
                    messages,
                )
                .await;
                persist_context_report_events(
                    &self.event_log,
                    session_id,
                    &self.agent_name,
                    self.app_id,
                    &model,
                    &report,
                )
                .await;
                Some((output_messages, output_options))
            }
            Err(error) => {
                tracing::warn!(
                    agent = %self.agent_name,
                    error = %error,
                    "failed to assemble legacy context report"
                );
                None
            }
        }
    }

    /// Run context assembly through Context Service, then apply Web-owned report persistence.
    ///
    /// The service command carries a serializable assembly snapshot so the
    /// runtime-host provider can rebuild the same provider chain without
    /// depending on Web. Scope-bound memory recall and knowledge digest handles
    /// remain local Strategies because they depend on the current
    /// application/session/agent visibility policy.
    async fn assemble_and_emit_report(
        &self,
        messages: &[serde_json::Value],
        options: &ChatOptions,
    ) -> Option<(Vec<serde_json::Value>, ChatOptions)> {
        let Some(session_id) = self.session_id.as_deref() else {
            return None;
        };
        let model = options.model.clone().unwrap_or_default();
        let scope = match ContextServiceScope::new(
            self.app_id,
            session_id.to_string(),
            self.agent_name.clone(),
        ) {
            Ok(scope) => scope,
            Err(error) => {
                tracing::warn!(
                    agent = %self.agent_name,
                    error = %error,
                    "context service scope validation failed"
                );
                #[allow(deprecated)]
                return self
                    .assemble_and_emit_report_legacy_local(messages, options)
                    .await;
            }
        };
        let mut trace = macaca_proto::TraceContext::new(uuid::Uuid::new_v4().to_string());
        trace.session_id = Some(session_id.to_string());
        trace.agent = Some(self.agent_name.clone());
        let mut command = match ContextAssembleCommand::new(
            scope,
            trace,
            model.clone(),
            framework_messages_to_llm(messages),
            framework_options_to_llm(options),
            self.context_selection.clone(),
        ) {
            Ok(command) => command,
            Err(error) => {
                tracing::warn!(
                    agent = %self.agent_name,
                    error = %error,
                    "context service assemble command validation failed"
                );
                #[allow(deprecated)]
                return self
                    .assemble_and_emit_report_legacy_local(messages, options)
                    .await;
            }
        };
        command.budget = self.context_budget;
        command.assembly = Some(ContextAssemblySnapshot {
            context_config: self.context_config.clone(),
            agent_profile_root: self.agent_profile_root.clone(),
            skill_capability_catalog: (*self.skill_capability_catalog).clone(),
            mcp_capability_catalog: (*self.mcp_capability_catalog).clone(),
            runtime_tool_capability_catalog: (*self.runtime_tool_capability_catalog).clone(),
            ready_mcp_server_ids: (*self.ready_mcp_server_ids).clone(),
            factory_params: serde_json::json!({}),
        });
        let lineage_count = self.lineage_compactions(session_id).await;
        let preflight_cfg = self.preflight_config();
        let mut assembled = match self.context_client.assemble(command).await {
            Ok(result) => result.assembled,
            Err(error) => {
                tracing::warn!(
                    agent = %self.agent_name,
                    error = %error,
                    "context service assembly failed; falling back to deprecated local assembler"
                );
                #[allow(deprecated)]
                return self
                    .assemble_and_emit_report_legacy_local(messages, options)
                    .await;
            }
        };
        if let (Some(ledger), Some(summary)) = (
            self.provider_health_ledger.as_ref(),
            assembled.report.provider_runtime.as_ref(),
        ) {
            ledger.record_provider_runtime_summary(summary);
        }
        assembled.report.lineage_compaction_count = lineage_count;
        let message_count_before_recall = assembled.messages.len();
        apply_active_recall(
            &self.recall_runtime,
            &self.memory_client,
            recall_scope(self.app_id, self.session_id.as_deref(), None),
            &preflight_cfg,
            self.composer_handles_active_vector_recall(),
            &mut assembled,
            messages,
        )
        .await;
        apply_preflight_memory(
            &self.recall_runtime,
            &self.memory_client,
            digest_scope(self.app_id, self.session_id.as_deref()),
            &preflight_cfg,
            &mut assembled,
            messages,
        )
        .await;
        let recall_injected_messages = assembled.messages.len() != message_count_before_recall;
        let output_messages = if assembled.report.engine_id == "legacy" && !recall_injected_messages
        {
            messages.to_vec()
        } else {
            llm_messages_to_framework(&assembled.messages)
        };
        let output_options = llm_options_to_framework(&assembled.options, options);
        let mut report = assembled.report.clone();
        persist_pruned_source_artifacts(
            &self.event_log,
            session_id,
            &self.agent_name,
            Some(self.app_id.to_string()),
            &mut report,
            messages,
        )
        .await;
        persist_context_report_events(
            &self.event_log,
            session_id,
            &self.agent_name,
            self.app_id,
            &model,
            &report,
        )
        .await;
        Some((output_messages, output_options))
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
            self.assemble_and_emit_report(&messages, options).await
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
