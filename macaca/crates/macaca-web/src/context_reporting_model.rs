use std::sync::Arc;

use async_trait::async_trait;
use macaca_context::{
    mcp_capability_provider_arc, memory_active_recall_provider_arc, profile_provider_arc,
    runtime_tool_capability_provider_arc, skill_capability_provider_arc, ActiveRecallBudget,
    ActiveRecallCapability, ActiveRecallPolicy, ContextAssembleInput, ContextBudget,
    ContextEngineSelection, ContextFacade, ContextPreflightRecallConfig, ContextProvider,
    DefaultActiveRecallProvider, McpCapabilityCatalog, RuntimeToolCapabilityCatalog,
    SkillCapabilityCatalog,
};
use macaca_framework::model::{ChatModel, ChatOptions, ChatResponse, ModelError};
use macaca_memory::TestMemoryManager;
use macaca_persist::{AppendEventCommand, EventLog, SessionLineageStore};
use macaca_proto::{config::ContextConfig, AgentId, ApplicationId};

use crate::context_memory_injection::{apply_active_recall, apply_preflight_memory};
use crate::context_message_codec::{
    framework_messages_to_llm, framework_options_to_llm, llm_messages_to_framework,
    llm_options_to_framework,
};
use crate::workspace_memory_recall_source::WorkspaceMemoryRecallSource;

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
    workspace_memory: Option<Arc<TestMemoryManager>>,
    /// Tunables copied from [`ContextConfig::agent_profile`] for hot-path injection checks.
    agent_profile: macaca_proto::config::AgentProfileContextConfig,
    /// Resolved directory that stores `AGENTS.md` / `IDENTITY.md`, depending on [`AgentProfileRootKind`].
    agent_profile_root: Option<std::path::PathBuf>,
    /// Composer-stage vector memory recall (see [`macaca_proto::config::ActiveVectorMemoryContextConfig`]).
    active_vector_memory: macaca_proto::config::ActiveVectorMemoryContextConfig,
    /// Narrow recall capability (built when [`Self::active_vector_memory`] is enabled and backend exists).
    memory_recall_capability: Option<Arc<dyn ActiveRecallCapability>>,
    /// Frozen skill catalog rows (metadata only) for [`macaca_context::SkillContextProvider`].
    skill_capability_catalog: Arc<SkillCapabilityCatalog>,
    /// MCP server/tool summaries from probe (compact; no resource bodies).
    mcp_capability_catalog: Arc<McpCapabilityCatalog>,
    /// Framework toolkit registration names only.
    runtime_tool_capability_catalog: Arc<RuntimeToolCapabilityCatalog>,
    /// MCP servers in [`macaca_runtime_host::McpRuntimeStatusState::Ready`] — dependency gap checks only.
    ready_mcp_server_ids: Arc<Vec<String>>,
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
        workspace_memory: Option<Arc<TestMemoryManager>>,
        routing_agent_id: Option<AgentId>,
        skill_capability_catalog: Arc<SkillCapabilityCatalog>,
        mcp_capability_catalog: Arc<McpCapabilityCatalog>,
        runtime_tool_capability_catalog: Arc<RuntimeToolCapabilityCatalog>,
        ready_mcp_server_ids: Arc<Vec<String>>,
    ) -> Self {
        // Resolve capability before moving `workspace_memory` into the struct field.
        let memory_recall_capability = Self::build_workspace_recall_capability(
            &merged_context_config,
            workspace_memory.as_ref(),
            routing_agent_id,
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
            workspace_memory,
            agent_profile: merged_context_config.agent_profile.clone(),
            agent_profile_root,
            active_vector_memory: merged_context_config.active_vector_memory.clone(),
            memory_recall_capability,
            skill_capability_catalog,
            mcp_capability_catalog,
            runtime_tool_capability_catalog,
            ready_mcp_server_ids,
        }
    }

    /// Builds the [`ActiveRecallCapability`] stack used by [`MemoryActiveRecallContextProvider`].
    ///
    /// Returns `None` when the feature is disabled or no in-process workspace memory backend is
    /// wired — this keeps unit tests lightweight while preserving production wiring through
    /// [`TestMemoryManager`].
    ///
    /// `routing_agent_id` should be the [`AgentId`] from the kernel manifest for the same
    /// `agent_name` as this chat model. When `None`, agent-private memory rows (tagged with
    /// `MemoryEntry::agent_id`) are not returned, matching the fail-closed policy in
    /// [`crate::workspace_memory_recall_source::workspace_memory_entry_visible_for_recall`].
    fn build_workspace_recall_capability(
        cfg: &ContextConfig,
        workspace_memory: Option<&Arc<TestMemoryManager>>,
        routing_agent_id: Option<AgentId>,
    ) -> Option<Arc<dyn ActiveRecallCapability>> {
        if !cfg.active_vector_memory.enabled {
            return None;
        }
        let wm = workspace_memory?;
        let source = Arc::new(WorkspaceMemoryRecallSource::new(
            Arc::clone(wm),
            cfg.active_vector_memory.max_hits,
            routing_agent_id,
        ));
        let policy = ActiveRecallPolicy {
            budget: ActiveRecallBudget {
                max_hits: cfg.active_vector_memory.max_hits,
                max_chars: cfg.active_vector_memory.max_chars,
                max_tokens: cfg.active_vector_memory.max_tokens,
                timeout_ms: cfg.active_vector_memory.timeout_ms,
            },
            ..ActiveRecallPolicy::default()
        };
        Some(Arc::new(DefaultActiveRecallProvider::new(
            "workspace-active-recall",
            source,
            policy,
        )))
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
    async fn assemble_and_emit_report(
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
        let mut providers: Vec<std::sync::Arc<dyn ContextProvider>> = Vec::new();
        if self.agent_profile.enabled {
            if let Some(root) = self.agent_profile_root.clone() {
                providers.push(profile_provider_arc(root, self.agent_profile.clone()));
            }
        }
        providers.push(skill_capability_provider_arc(
            Arc::clone(&self.skill_capability_catalog),
            Arc::clone(&self.ready_mcp_server_ids),
        ));
        providers.push(mcp_capability_provider_arc(Arc::clone(
            &self.mcp_capability_catalog,
        )));
        providers.push(runtime_tool_capability_provider_arc(Arc::clone(
            &self.runtime_tool_capability_catalog,
        )));
        if let Some(capability) = self.memory_recall_capability.clone() {
            if self.active_vector_memory.enabled {
                providers.push(memory_active_recall_provider_arc(
                    capability,
                    self.active_vector_memory.clone(),
                ));
            }
        }
        match ContextFacade::builtins(self.context_selection.clone())
            .assemble_model_context(input, &providers)
            .await
        {
            Ok(mut assembled) => {
                assembled.report.lineage_compaction_count = lineage_count;
                let message_count_before_recall = assembled.messages.len();
                apply_active_recall(
                    &self.recall_runtime,
                    self.workspace_memory.as_ref(),
                    &preflight_cfg,
                    self.composer_handles_active_vector_recall(),
                    &mut assembled,
                    messages,
                )
                .await;
                apply_preflight_memory(
                    &self.recall_runtime,
                    self.workspace_memory.as_ref(),
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
                let report = assembled.report.clone();
                let fallback_decisions = assembled
                    .report
                    .decisions
                    .iter()
                    .filter(|decision| decision.code == "context_engine_fallback")
                    .cloned()
                    .collect::<Vec<_>>();
                self.event_log
                    .append_command(
                        AppendEventCommand::new(
                            session_id,
                            "context_report",
                            &self.agent_name,
                            serde_json::json!({
                                "agent": self.agent_name,
                                "engine_id": report.engine_id,
                                "requested_engine_id": report.requested_engine_id,
                                "engine_fallback_applied": report.engine_fallback_applied,
                                "lineage_compaction_count": report.lineage_compaction_count,
                                "request_id": report.request_id,
                                "model": model,
                                "created_at": report.created_at,
                                "estimated_total_tokens": report.estimated_total_tokens,
                                "token_budget": report.token_budget,
                                "stable_prompt_tokens": report.stable_prompt_tokens,
                                "dynamic_prompt_tokens": report.dynamic_prompt_tokens,
                                "history_tokens": report.history_tokens,
                                "tool_schema_tokens": report.tool_schema_tokens,
                                "skill_tokens": report.skill_tokens,
                                "memory_tokens": report.memory_tokens,
                                "trace_tokens": report.trace_tokens,
                                "pruned_tokens": report.pruned_tokens,
                                "stable_prompt_hash": report.stable_prompt_hash,
                                "prompt_hash": report.prompt_hash,
                                "source_count": report.sources.len(),
                                "decision_count": report.decisions.len(),
                                "active_recall": report.active_recall.iter().map(|recall| {
                                    serde_json::json!({
                                        "provider_id": recall.provider_id,
                                        "total_candidates": recall.total_candidates,
                                        "selected_candidates": recall.selected_candidates,
                                        "latency_ms": recall.latency_ms,
                                        "source_count": recall.source_breakdown.len(),
                                        "decisions": recall.decisions.iter().map(|decision| {
                                            serde_json::json!({
                                                "code": decision.code,
                                                "severity": decision.severity,
                                                "message": decision.message,
                                            })
                                        }).collect::<Vec<_>>(),
                                    })
                                }).collect::<Vec<_>>(),
                                "source_breakdown": report.sources.iter().map(|source| {
                                    serde_json::json!({
                                        "id": source.id,
                                        "kind": source.kind,
                                        "label": source.label,
                                        "estimated_tokens": source.estimated_tokens,
                                        "byte_size": source.byte_size,
                                        "included": source.included,
                                        "pruned_tokens": source.pruned_tokens,
                                        "render_mode": source.render_mode,
                                        "trust_level": source.trust_level,
                                        "source_ref": source.source_ref,
                                    })
                                }).collect::<Vec<_>>(),
                                "decisions": report.decisions.iter().map(|decision| {
                                    serde_json::json!({
                                        "code": decision.code,
                                        "severity": decision.severity,
                                        "message": decision.message,
                                    })
                                }).collect::<Vec<_>>(),
                                "warnings": report.decisions.iter()
                                    .filter(|decision| decision.severity != macaca_context::ContextDecisionSeverity::Info)
                                    .map(|decision| {
                                        serde_json::json!({
                                            "code": decision.code,
                                            "severity": decision.severity,
                                            "message": decision.message,
                                        })
                                    })
                                    .collect::<Vec<_>>(),
                            }),
                        )
                        .with_app_id(self.app_id.to_string())
                        .with_agent_name(self.agent_name.clone()),
                    )
                    .await;
                for decision in fallback_decisions {
                    self.event_log
                        .append_command(
                            AppendEventCommand::new(
                                session_id,
                                "context_engine_fallback",
                                &self.agent_name,
                                serde_json::json!({
                                    "agent": self.agent_name,
                                    "engine_id": report.engine_id,
                                    "requested_engine_id": report.requested_engine_id,
                                    "request_id": report.request_id,
                                    "code": decision.code,
                                    "severity": decision.severity,
                                    "message": decision.message,
                                }),
                            )
                            .with_app_id(self.app_id.to_string())
                            .with_agent_name(self.agent_name.clone()),
                        )
                        .await;
                }
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
