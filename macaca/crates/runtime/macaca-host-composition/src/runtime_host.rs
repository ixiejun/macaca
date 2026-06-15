//! Explicit runtime-host facade surface for SDK composition roots.
//!
//! The SDK intentionally does not re-export the whole runtime-host crate.  This
//! module keeps the current shell composition paths stable while making every
//! exposed runtime-host contract visible for dependency-boundary review.

pub use macaca_runtime_host::{
    agent_context_service_descriptor, agent_execution_service_descriptor, alert_service_descriptor,
    apply_concurrency_isolation, bootstrap_autonomy_services,
    bootstrap_default_application_execution_service, bootstrap_delegate_task_tool,
    bootstrap_domain_pack_services, bootstrap_driver_service, bootstrap_get_task_result_tool,
    bootstrap_interaction_service, bootstrap_list_agents_tool,
    bootstrap_local_app_protocol_service, bootstrap_local_approval_service,
    bootstrap_local_base_tools, bootstrap_local_code_intelligence_service,
    bootstrap_local_config_service, bootstrap_local_diagnostics_service,
    bootstrap_local_file_service, bootstrap_local_git_service, bootstrap_local_hook_service,
    bootstrap_local_plugin_marketplace_service, bootstrap_local_process_service,
    bootstrap_local_remote_environment_service, bootstrap_local_review_service,
    bootstrap_local_sandbox_service, bootstrap_local_skill_assets,
    bootstrap_local_skill_service_provider, bootstrap_local_task_service,
    bootstrap_optional_services, bootstrap_task_toolkit_tools, bootstrap_tool_planning_service,
    bootstrap_unavailable_realtime_service, bootstrap_workspace_toolkit_tools,
    execution_control_execution_id, execution_control_scope, execution_control_service_descriptor,
    extract_single_shell_fence, heartbeat_exact_shell_contract, industrial_tool_planning_service,
    mcp_service_descriptor, memory_service_descriptor, normalize_workspace_tool_input,
    plugin_capability_service_descriptor, plugin_control_service_descriptor,
    plugin_hook_service_descriptor, probe_definition_statuses,
    resolve_execution_control_policy_local, resolve_workspace_tool_path, runtime_agent_max_iters,
    runtime_agent_tool_choice, should_skip_heartbeat_without_source, user_prompt_with_context,
    wire_kernel_to_agent_execution_service, ActiveRecallCapability, ActiveRecallRequest,
    ActiveRecallResult, AgentContextBackend, AgentContextSystemServiceProvider,
    AgentExecutionBackend, AgentExecutionEvidenceCollector, AgentExecutionHostAdapter,
    AgentExecutionOutputHasher, AgentExecutionSystemServiceProvider, AgentInfo, AgentRunner,
    AlertSystemServiceProvider, ApplicationExecutorRegistry, ApplicationOrchestrationBackend,
    ApplicationSystemServiceProvider, AuditLogger, AutonomyProviderMode, AutonomyRuntimeBundle,
    AutonomyRuntimeConfig, ComposedAgentExecutionBackend, ConfiguredMemoryManager,
    ConstructedRuntimeAgent, ContextSystemServiceProvider, DefaultActiveRecallStrategy,
    DefaultKernelFacade, DelegateTaskToolBootstrapPorts, DomainPackProviderRegistration,
    ExecutionControlForkJoinCoordinator, ExecutionControlGoalLifecycleCoordinator,
    ExecutionControlLocalNotification, ExecutionControlLocalNotificationRuntime,
    ExecutionControlRuntimeCapability, ExecutionControlSessionLoopCoordinator,
    ExecutionControlSystemServiceProvider, FabricMemoryRuntime, FileMemory, ForgetMemory,
    ForkJoinParentResumeRequest, ForkSessionMappingRecorder, FrameworkAgentConstructionPort,
    FrameworkAgentMaterializationPort, GoalLifecycleParentResumeRequest,
    GoalLifecycleParentWaitRequest, GoalRecordedObserver, InMemoryVectorStore, Kernel,
    KernelBuilder, KernelFacade, KernelPersistencePort, KnowledgeCompileCapability,
    KnowledgeCompileRequest, KnowledgeCompileResult, KnowledgeCompiler, LlmProvider,
    LlmProviderCatalogProfile, LlmProviderProfile, LlmRouter, LlmSystemServiceProvider,
    LocalPlanLoopController, LocalWorkerLoopController, McpDefinitionSource, McpLifecycleScope,
    McpRegistryConfig, McpRuntimeContext, McpRuntimeFacade, McpRuntimeStatus,
    McpRuntimeStatusState, McpServerDefinition, McpServerFactory, McpSystemServiceProvider,
    McpToolPolicy, MemoryBackendConfig, MemoryBackendFactory, MemoryCapabilitySet,
    MemoryDeleteRequest, MemoryFacade, MemoryForgetCommand, MemoryGetCommand, MemoryGetRequest,
    MemoryGetResult, MemoryPolicyHints, MemoryPrefetchCommand, MemoryPrefetchRequest,
    MemoryRecallCommand, MemoryRecallResult, MemoryRememberCommand, MemoryRememberResult,
    MemoryRuntimeFacade, MemoryRuntimeStatus, MemoryScope, MemorySearchRequest,
    MemoryServiceSnapshot, MemoryServiceSnapshotCommand, MemoryStatusCommand, MemoryStatusReport,
    MemorySystemServiceProvider, MemoryWriteRequest, MockEmbedding, ModelSelection,
    ModelSelectionRequest, ModelTarget, OpaqueExecutionControlHandle,
    OptionalServiceBootstrapInputs, PluginCapabilitySystemServiceProvider,
    PluginControlSystemServiceProvider, PluginHookSystemServiceProvider, RecallQuery, RecallResult,
    RememberText, RuntimeEnvBuilder, RuntimeHostFrameworkAgentConstructionService,
    ServiceAuditRuntimeBundle, ServiceBackedFrameworkRuntimeAgentPort,
    ServiceProviderFactoryContext, ServiceProviderInstance, ServiceRuntime, ServiceRuntimeConfig,
    SessionLoopKind, SessionLoopLocalRuntime, SessionLoopRegisterRequest,
    SessionLoopShutdownRequest, SessionLoopWakeRequest, SessionMemory, SharedTombstoneRegistry,
    StaticServiceProviderFactory, SystemService, TaskContext, TaskResult,
    TaskToolkitBootstrapRequest, TaskToolkitPolicy, TestMemoryManager, TodoStore, TombstoneIndex,
    UnavailableAgentExecutionPort, WorkspaceToolkitBootstrapRequest, MEMORY_FORGET_COMMAND,
    MEMORY_GET_COMMAND, MEMORY_PREFETCH_COMMAND, MEMORY_RECALL_COMMAND, MEMORY_REMEMBER_COMMAND,
    MEMORY_SERVICE_ID, MEMORY_SNAPSHOT_COMMAND, MEMORY_STATUS_COMMAND,
    SERVICE_CALL_AUDIT_SERVICE_ID, SESSION_LOOP_PLAN_WAKE_EVENT, SESSION_LOOP_TASK_CAPABILITY_ID,
    SESSION_LOOP_WORKER_WAKE_EVENT,
};

pub use macaca_runtime_host::{
    active_recall_degraded, active_recall_diagnostics_from_prefetch, assemble_context_providers,
    estimate_text_tokens, filter_digest_items_by_tombstones, list_builtin_engine_infos,
    list_builtin_family_descriptors, mcp_tool_collisions, memory_source,
    render_active_recall_fence, ActiveRecallDiagnostics, ActiveRecallPolicy, BudgetPolicy,
    CapabilityCandidate, CapabilityCollisionRecord, CompactionDecision, CompactionSummaryEnvelope,
    ConfidenceScore, ContextActiveRecallBudget, ContextActiveRecallCapability,
    ContextActiveRecallCommand, ContextActiveRecallServiceResult, ContextAdapterSafetyPolicy,
    ContextAfterTurnInput, ContextAssembleCommand, ContextAssembleInput, ContextAssembleResult,
    ContextAssembleServiceResult, ContextAssemblySnapshot, ContextBudget, ContextCacheClass,
    ContextCandidate, ContextCandidateKind, ContextComposeContext, ContextComposer,
    ContextDecisionReport, ContextDecisionSeverity, ContextEngine, ContextEngineConformance,
    ContextEngineInfo, ContextEngineInventoryCommand, ContextEngineRegistry,
    ContextEngineSelection, ContextFacade, ContextFacadeAssemblyPolicy, ContextFallbackPolicy,
    ContextManagerFacade, ContextOptionsPatch, ContextPlan, ContextPlanBuilder,
    ContextPlanDecision, ContextPolicyHints, ContextPreflightRecallConfig, ContextProvider,
    ContextProviderDiagnostics, ContextProviderFactory, ContextProviderInventoryCommand,
    ContextProviderInventoryItem, ContextProviderOutcome, ContextProviderRegistry,
    ContextProviderStage, ContextRenderInput, ContextRenderMode, ContextReport,
    ContextReportBuilder, ContextReportResult, ContextRuntimeFacade, ContextScope,
    ContextServiceRuntimeCapabilities, ContextServiceScope, ContextServiceSnapshot,
    ContextServiceSnapshotCommand, ContextSnippet, ContextSourceKind, ContextSourceProvenance,
    ContextSourceReference, ContextSourceReport, ContextTarget, DeclaredCapabilityDependency,
    DefaultActiveRecallProvider, DefaultBudgetPolicy, DefaultContextComposer, DefaultPruningPolicy,
    DefaultSourceRenderer, DigestStrengthSnapshot, ExternalAdapterContextEngine,
    ExternalContextAdapter, ExternalContextAdapterInfo, GovernedCollection,
    KnowledgeDigestCapability, KnowledgeDigestContextProvider, KnowledgeDigestItem,
    McpCapabilityCatalog, McpContextProvider, McpServerCapabilitySummary,
    MemoryActiveRecallContextProvider, MemoryPrefetchResult, MemoryProviderHookInput,
    MemoryRecallItem, MemoryRecallQuery, MemorySourceProvider, NoopCompactionLifecycleHook,
    OpaqueExternalPayload, PassthroughContextEngine, PrivacyTier, ProfileFileContextProvider,
    PromptComposer, PromptSection, PromptSectionBuilder, PromptStability,
    ProviderAssemblyEnvironment, ProviderFactoryInput, ProviderFamilyDescriptor,
    ProviderHealthLedger, ProviderHealthSnapshot, ProviderInvocationSummary,
    ProviderRuntimeSummary, PruningContextEngine, PruningDecision, PruningPolicy, RecallCandidate,
    RecallDecision, RuntimeToolCapabilityCatalog, RuntimeToolCapabilityProvider,
    SkillCapabilityCatalog, SkillCapabilityGovernanceReport, SkillCapabilityRecord,
    SkillContextProvider, SkillFilterDiagnostic, SummaryContextEngine, ThresholdCompactionPolicy,
    ToolCapabilityIndex, ToolCapabilityReport, TrustLevel, VersionedContextProvider,
    WindowedContextEngine, CONTEXT_ACTIVE_RECALL_COMMAND, CONTEXT_ASSEMBLE_COMMAND,
    CONTEXT_ENGINE_INVENTORY_COMMAND, CONTEXT_PROVIDER_INVENTORY_COMMAND, CONTEXT_SERVICE_ID,
    CONTEXT_SNAPSHOT_COMMAND, FAMILY_AGENT_PROFILE, FAMILY_KNOWLEDGE_DIGEST, FAMILY_MCP_CAPABILITY,
    FAMILY_MEMORY_ACTIVE_RECALL, FAMILY_RUNTIME_TOOL_CAPABILITY, FAMILY_SKILL_CAPABILITY,
    FAMILY_TOOL_CAPABILITY,
};

pub use macaca_runtime_host::{
    CheckTodoProgressTool, ClaimTaskTool, CompositeToolSet, CreateGoalTool, CreateTodoTool,
    CreateTodosTool, DefaultToolSet, DelegateTaskTool, FileReadTool, FileWriteTool,
    GetTaskResultCallback, GetTaskResultTool, ListAgentsTool, ListMyTasksTool, OnGoalCreated,
    OnGoalRecorded, OnTodoReviewed, ReassignTaskTool, ReviewTodoTool, ShellTool, StartTaskTool,
    SubmitTaskForReviewTool, TaskResultData, Tool, ToolCatalog, ToolCommand, ToolCommandContext,
    ToolCommandExecutor, ToolCommandMiddleware, ToolCommandPipeline, ToolSchemaProvider,
    TraceEvent, TraceToolCommandMiddleware, UpdateTaskProgressTool,
};

pub use macaca_runtime_host::{
    chat_options_to_llm_options, client_from_transport, llm_messages_to_json_values,
    messages_from_json_values, register_mcp_tools, register_mcp_tools_with_options, Agent,
    AgentBuildIntent, AgentBuildRequest, AgentBuildRequestBuilder, AgentCapabilitySet, AgentError,
    AgentExecutionInput, AgentExecutionLauncher, AgentExecutionOutput, AgentIdentity,
    AgentLifecycleConfig, AgentResult, AgentRunCommand, AgentRunResult, AgentServices,
    AgentSessionError, AgentSessionHealth, AgentSessionHealthState, AgentSessionStore,
    AgentSessionStoreSnapshot, AgentState, AgentToolConfig, AgentTraceContext,
    AgentTransitionReason, AnthropicFormatter, ApplicationPromptParts, ApplicationSemantics,
    ApplicationToolPolicy, AudioBlock, CachedRead, ChatModel, ChatOptions, ChatResponse, ChatUsage,
    ContentBlock, DashScopeFormatter, DataBlock, ExecutionContext, ExecutionStatus, Formatter,
    FormatterError, GenerateReason, HintBlock, Hook, HookRegistry, HookedAgent, ImageBlock,
    InMemoryAgentSessionStore, McpCallResult, McpClient, McpError, McpResourceDef, McpResourceRead,
    McpResourceTemplateDef, McpSessionMode, McpTimeouts, McpToolDef, McpToolHandler,
    McpToolNameConflictPolicy, McpToolRegistrationOptions, McpTransportConfig, MessageUsage,
    ModelError, Msg, MsgContent, MsgValidationError, OpenAiFormatter, PendingToolState,
    PermissionState, Plan, PlanError, PlanModeState, PlanNotebook, PlanState, ReActAgent,
    RegisteredTool, Role, RuntimeContext, RuntimeContextError, SessionKey, SubTask, SubTaskState,
    TaskContextState, TextBlock, ThinkingBlock, ToolCallState, ToolChoice, ToolError, ToolGroup,
    ToolHandler, ToolMiddleware, ToolResponse, ToolResultBlock, ToolResultState, ToolTraceEvent,
    ToolUseBlock, Toolkit, ToolkitContributor, ToolkitResource, TracedAgentFactory, VideoBlock,
};

pub use macaca_runtime_host::{
    app_agent_manifest_view, app_agent_prompt_semantics, app_entry_agent_name,
    app_task_planning_contract, application_service_descriptor, discovered_app_agent_names,
    expand_service_capabilities, validate_ui_runtime_config, AgentSource, AppContextConfig,
    AppLayer, AppLlmConfig, AppLoader, AppRegistry, AppRuntime, AppUiBridgeConfig, AppUiCspMode,
    AppUiFramework, AppUiNetworkPolicy, AppUiPresentationConfig, AppUiRuntimeConfig,
    AppUiRuntimeKind, AppUiSandboxConfig, AppUiSandboxIsolation, AppUiSurfaceChrome,
    AppUiSurfaceConfig, AppUiSurfaceMode, AppUiThemeConfig, AppUiThemeMode, DiscoveredApp,
    SharedDomainPackCatalog,
};

pub use macaca_runtime_host::{
    skill_service_descriptor, FilteredSkill, SelfEvolutionEvaluationCheckpoint,
    SelfEvolutionEvaluationCheckpointKind, SelfEvolutionEvaluationLifecycle,
    SelfEvolutionEvaluationRecord, SelfEvolutionReportRefs, SelfEvolutionRunMetrics,
    SelfEvolutionScore, SelfEvolutionWhiteBoxEvidence, SkillAliasKind, SkillAliasResolutionPolicy,
    SkillAliasResolutionStatus, SkillAliasResolveCommand, SkillAliasResolveResult,
    SkillAliasSnapshotCommand, SkillAliasSnapshotResult, SkillAliasUpsertCommand,
    SkillAliasUpsertResult, SkillAuthorKind, SkillAutonomousMaterializationRunCommand,
    SkillAutonomousMaterializationRunResult, SkillAutonomousMaterializationSnapshotCommand,
    SkillAutonomousMaterializationSnapshotResult, SkillCleanupCommand, SkillCurationDryRunCommand,
    SkillCurationDryRunResult, SkillCurationLifecycleAction, SkillCurationLifecycleCommand,
    SkillCurationLifecycleResult, SkillCurationRollbackCommand, SkillCurationRollbackResult,
    SkillCurationRunCommand, SkillCurationRunResult, SkillCurationSnapshotCommand,
    SkillCurationSnapshotResult, SkillEvaluationCheckpointAppendCommand,
    SkillEvaluationCheckpointAppendResult, SkillEvaluationReportCommand,
    SkillEvaluationReportResult, SkillEvaluationScoreCommand, SkillEvaluationScoreResult,
    SkillEvolutionCandidateClassification, SkillEvolutionPromoteDraftCommand,
    SkillEvolutionPromoteDraftResult, SkillEvolutionProposalAction,
    SkillEvolutionProposePatchCommand, SkillEvolutionProposePatchResult,
    SkillEvolutionRejectDraftCommand, SkillEvolutionRejectDraftResult, SkillExecutableLoadCommand,
    SkillExecutableLoadResult, SkillExperienceCandidate, SkillExperienceCandidateDestination,
    SkillExperienceEvidenceGateStatus, SkillExperienceProposalCommand,
    SkillExperienceProposalResult, SkillExperienceProposalSnapshotCommand,
    SkillExperienceProposalSnapshotResult, SkillGovernanceProvenance, SkillGovernanceRecord,
    SkillGovernanceRecordUsageCommand, SkillGovernanceRecordUsageResult,
    SkillGovernanceSnapshotCommand, SkillGovernanceSnapshotResult, SkillInstallSpec,
    SkillLifecycleState, SkillMcpServerConfig, SkillOperatorCatalogListCommand,
    SkillOperatorCatalogListResult, SkillOperatorChangedCommand, SkillOperatorChangedResult,
    SkillOperatorConfigWriteCommand, SkillOperatorConfigWriteResult,
    SkillOperatorEnablementCommand, SkillOperatorEnablementResult,
    SkillOperatorMarkdownReadCommand, SkillOperatorMarkdownReadResult, SkillOperatorUnwatchCommand,
    SkillOperatorWatchCommand, SkillOperatorWatchResult, SkillPackageOwnershipClass, SkillPolicy,
    SkillProposalProcessingRunCommand, SkillProposalProcessingRunResult,
    SkillProposalProcessingSnapshotCommand, SkillProposalProcessingSnapshotResult,
    SkillSemanticReviewResult, SkillServicePolicyHints, SkillServiceScope, SkillServiceSnapshot,
    SkillServiceSnapshotCommand, SkillSnapshot, SkillSnapshotEntry, SkillSnapshotServiceCommand,
    SkillSnapshotServiceResult, SkillSourceScope, SkillStatusCommand, SkillStatusResult,
    SkillToolCatalogCommand, SkillToolCatalogResult, SkillToolInvokeCommand, SkillToolInvokeResult,
    SkillUsageEventKind, SkillUsageObservation, SkillUsageTelemetry, SKILL_ALIAS_RESOLVE_COMMAND,
    SKILL_ALIAS_SNAPSHOT_COMMAND, SKILL_ALIAS_UPSERT_COMMAND, SKILL_CLEANUP_COMMAND,
    SKILL_CURATION_ARCHIVE_COMMAND, SKILL_CURATION_DRY_RUN_COMMAND, SKILL_CURATION_PIN_COMMAND,
    SKILL_CURATION_QUARANTINE_COMMAND, SKILL_CURATION_REJECT_COMMAND,
    SKILL_CURATION_RELEASE_QUARANTINE_COMMAND, SKILL_CURATION_RESTORE_COMMAND,
    SKILL_CURATION_ROLLBACK_COMMAND, SKILL_CURATION_RUN_COMMAND, SKILL_CURATION_SNAPSHOT_COMMAND,
    SKILL_CURATION_UNPIN_COMMAND, SKILL_EVALUATION_CHECKPOINT_APPEND_COMMAND,
    SKILL_EVALUATION_REPORT_COMMAND, SKILL_EVALUATION_SCORE_COMMAND,
    SKILL_EVOLUTION_MATERIALIZATION_OPERATOR_RUN_COMMAND,
    SKILL_EVOLUTION_MATERIALIZATION_OPERATOR_SNAPSHOT_COMMAND,
    SKILL_EVOLUTION_PROCESSING_RUN_COMMAND, SKILL_EVOLUTION_PROCESSING_SNAPSHOT_COMMAND,
    SKILL_EVOLUTION_PROMOTE_DRAFT_COMMAND, SKILL_EVOLUTION_PROPOSE_FROM_TASK_COMMAND,
    SKILL_EVOLUTION_PROPOSE_PATCH_COMMAND, SKILL_EVOLUTION_REJECT_DRAFT_COMMAND,
    SKILL_EVOLUTION_SNAPSHOT_COMMAND, SKILL_EXECUTABLE_LOAD_COMMAND,
    SKILL_GOVERNANCE_RECORD_USAGE_COMMAND, SKILL_GOVERNANCE_SNAPSHOT_COMMAND,
    SKILL_OPERATOR_CATALOG_LIST_COMMAND, SKILL_OPERATOR_CHANGED_COMMAND,
    SKILL_OPERATOR_CONFIG_WRITE_COMMAND, SKILL_OPERATOR_ENABLEMENT_COMMAND,
    SKILL_OPERATOR_MARKDOWN_READ_COMMAND, SKILL_OPERATOR_UNWATCH_COMMAND,
    SKILL_OPERATOR_WATCH_COMMAND, SKILL_SERVICE_ID, SKILL_SERVICE_SNAPSHOT_COMMAND,
    SKILL_SNAPSHOT_COMMAND, SKILL_STATUS_COMMAND, SKILL_TOOL_CATALOG_COMMAND,
    SKILL_TOOL_INVOKE_COMMAND,
};

/// Executor contracts still consumed by Web shell adapters and tests.
pub mod executor {
    pub use macaca_runtime_host::executor::{
        AgentInfo, AgentRunner, ApplicationExecutor, ExecutorEvent, ExecutorEventFactory, TaskId,
        TaskResult, TokenUsage,
    };

    /// Stable nested path for the application executor type.
    pub mod app_executor {
        pub use macaca_runtime_host::executor::app_executor::ApplicationExecutor;
    }

    /// Fork/join hook events projected by the SSE and session adapters.
    pub mod fork_manager {
        pub use macaca_runtime_host::executor::fork_manager::HookEvent;
    }
}

/// Persistence contracts exposed only through runtime-host composition paths.
pub mod persist {
    pub use macaca_runtime_host::persist::{
        AppendEventCommand, EntitlementStore, EventLog, EventLogQuery, InMemoryEntitlementStore,
        InMemoryPaymentStore, PaymentStore, PersistBackend, PersistStore, RedbStore,
        SessionLineageStore,
    };
}

/// Skill-to-MCP mapping helpers used by MCP shell resolution adapters.
pub mod skill_mcp_mapping_registry {
    pub use macaca_runtime_host::skill_mcp_mapping_registry::default_skill_mcp_mapping_registry;
}

/// WASM runtime-host bridge contracts used by application discovery bootstrap.
pub mod wasm_runtime_provider {
    pub use macaca_runtime_host::wasm_runtime_provider::{
        WasmHostImportBridge, WasmHostImportBridgeConfig,
    };
}
