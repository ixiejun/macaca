//! Focused SDK surfaces for remaining runtime-host-gated composition paths.
//!
//! These modules intentionally keep public caller paths narrow while the
//! runtime-host bootstrap dependency is retired. They are facades over existing
//! contracts, not provider constructors or shell-owned behavior.

/// Kernel primitive surface used only by composition roots.
///
/// Keeping these types under an explicit module makes direct kernel ownership
/// visible during boundary audits. Callers that need business behavior should
/// use `SystemFacade` or focused service clients instead of reaching through
/// this low-level bootstrap surface.
pub mod kernel {
    pub use crate::runtime_host::{
        AuditLogger, Kernel, KernelBuilder, KernelPersistencePort, SystemService,
        UnavailableAgentExecutionPort,
    };
}

/// LLM routing surface currently exposed for shell composition roots.
///
/// Runtime behavior must still flow through service clients; this module only
/// preserves type identity for the remaining composition owners.
pub mod llm {
    pub use crate::runtime_host::{
        LlmProvider, LlmRouter, ModelSelection, ModelSelectionRequest, ModelTarget,
    };
    pub use macaca_proto::{
        llm_service_descriptor, LlmCatalogReadCommand, LlmChatCommand, LlmPolicyHints,
        LlmRouteResolveCommand, LlmRouteSummary, LlmServiceScope, LlmServiceSnapshotCommand,
        LLM_MODEL_LIST_COMMAND, LLM_PROVIDER_CAPABILITIES_READ_COMMAND, LLM_ROUTE_RESOLVE_COMMAND,
        LLM_SERVICE_ID,
    };
}

/// MCP runtime helper surface currently exposed for shell composition roots.
///
/// This keeps host-local MCP probing and definition loading behind a focused
/// SDK module while the canonical MCP service client absorbs the remaining
/// runtime bootstrap surface.
pub mod mcp_runtime {
    pub use crate::runtime_host::{
        apply_concurrency_isolation, probe_definition_statuses, McpDefinitionSource,
        McpLifecycleScope, McpRegistryConfig, McpRuntimeContext, McpRuntimeFacade,
        McpRuntimeStatus, McpRuntimeStatusState, McpServerDefinition, McpServerFactory,
        McpToolPolicy, RuntimeEnvBuilder,
    };

    /// Declarative Skill-to-MCP mapping registry used by Skill MCP adapters.
    pub mod skill_mcp_mapping_registry {
        pub use macaca_runtime_host::skill_mcp_mapping_registry::default_skill_mcp_mapping_registry;
    }
}

/// Executor event and application-executor contracts used by shell adapters.
///
/// Shells may observe executor events and inject host-local executor handles
/// during composition, but service ownership remains in runtime-host.
pub mod executor {
    pub use crate::runtime_host::executor::{
        AgentInfo, AgentRunner, ApplicationExecutor, ExecutorEvent, ExecutorEventFactory, TaskId,
        TaskResult, TokenUsage,
    };
    pub use crate::runtime_host::ApplicationExecutorRegistry;

    /// Stable nested path for the application executor type.
    pub mod app_executor {
        pub use crate::runtime_host::executor::app_executor::ApplicationExecutor;
    }

    /// Fork/join hook events projected by the SSE and session adapters.
    pub mod fork_manager {
        pub use crate::runtime_host::executor::fork_manager::HookEvent;
    }
}

/// Persistence contracts used by shell composition and trace projection code.
///
/// This module keeps concrete store helpers out of the broad runtime-host root
/// while the longer-term persistence client boundary continues to converge.
pub mod persist {
    pub use crate::runtime_host::persist::{
        AppendEventCommand, EntitlementStore, EventLog, EventLogQuery, InMemoryEntitlementStore,
        InMemoryPaymentStore, PaymentStore, PersistBackend, PersistStore, RedbStore,
        SessionLineageStore,
    };
}

/// Host service-runtime composition contracts used by shell bootstrap.
///
/// This surface is intentionally limited to provider-neutral runtime, factory,
/// and audit handles. Provider implementations stay in runtime-host.
pub mod service_runtime {
    pub use crate::runtime_host::{
        ServiceAuditRuntimeBundle, ServiceProviderFactoryContext, ServiceProviderInstance,
        ServiceRuntime, ServiceRuntimeConfig, StaticServiceProviderFactory,
        SERVICE_CALL_AUDIT_SERVICE_ID,
    };
}

/// Execution-control and session-loop handles used by shell adapters.
///
/// These types coordinate pause/resume, fork/join, and session-loop wakeups
/// through runtime-host services. The focused path keeps those contracts visible
/// without exposing the whole runtime-host root.
pub mod execution_control {
    pub use crate::runtime_host::{
        execution_control_service_descriptor, ExecutionControlForkJoinCoordinator,
        ExecutionControlGoalLifecycleCoordinator, ExecutionControlLocalNotification,
        ExecutionControlLocalNotificationRuntime, ExecutionControlRuntimeCapability,
        ExecutionControlSessionLoopCoordinator, ExecutionControlSystemServiceProvider,
        ForkJoinParentResumeRequest, GoalLifecycleParentResumeRequest,
        GoalLifecycleParentWaitRequest, OpaqueExecutionControlHandle, SessionLoopKind,
        SessionLoopLocalRuntime, SessionLoopRegisterRequest, SessionLoopShutdownRequest,
        SessionLoopWakeRequest, SESSION_LOOP_PLAN_WAKE_EVENT, SESSION_LOOP_TASK_CAPABILITY_ID,
        SESSION_LOOP_WORKER_WAKE_EVENT,
    };
}

/// Autonomy service bootstrap contracts used by shell composition roots.
///
/// The shell keeps lifecycle handles for shutdown and diagnostics, while
/// runtime-host remains the owner of supervisor/provider construction.
pub mod autonomy_runtime {
    pub use crate::runtime_host::{
        bootstrap_autonomy_services, AutonomyProviderMode, AutonomyRuntimeBundle,
        AutonomyRuntimeConfig,
    };
}

/// Agent-execution service and construction ports used by shell composition.
///
/// The unified execution implementation remains in runtime-host. Web imports
/// these contracts only to inject host-local adapters and register the service.
pub mod agent_execution {
    pub use crate::runtime_host::{
        agent_context_service_descriptor, agent_execution_service_descriptor,
        execution_control_execution_id, execution_control_scope, extract_single_shell_fence,
        heartbeat_exact_shell_contract, resolve_execution_control_policy_local,
        runtime_agent_max_iters, runtime_agent_tool_choice, should_skip_heartbeat_without_source,
        user_prompt_with_context, wire_kernel_to_agent_execution_service, AgentContextBackend,
        AgentContextSystemServiceProvider, AgentExecutionBackend, AgentExecutionEvidenceCollector,
        AgentExecutionHostAdapter, AgentExecutionOutputHasher, AgentExecutionSystemServiceProvider,
        ComposedAgentExecutionBackend, ConstructedRuntimeAgent, FrameworkAgentMaterializationPort,
        RuntimeHostFrameworkAgentConstructionService, ServiceBackedFrameworkRuntimeAgentPort,
    };
}

/// Application, plugin, domain-pack, and WASM bootstrap contracts.
///
/// These exports are for approved host composition roots. They keep provider
/// construction visibly grouped while preserving runtime-host ownership.
pub mod application_bootstrap {
    pub use crate::runtime_host::{
        application_service_descriptor, bootstrap_domain_pack_services,
        plugin_capability_service_descriptor, plugin_control_service_descriptor,
        plugin_hook_service_descriptor, ApplicationOrchestrationBackend,
        ApplicationSystemServiceProvider, DomainPackProviderRegistration,
        PluginCapabilitySystemServiceProvider, PluginControlSystemServiceProvider,
        PluginHookSystemServiceProvider,
    };

    /// WASM host-import bridge contracts consumed by application bootstrap.
    pub mod wasm_runtime_provider {
        pub use crate::runtime_host::wasm_runtime_provider::{
            WasmHostImportBridge, WasmHostImportBridgeConfig,
        };
    }
}

/// Host service-provider bootstrap functions used by composition roots.
///
/// This Facade groups runtime-host provider registration entry points behind a
/// narrow SDK path. It does not move provider ownership into the SDK.
pub mod service_bootstrap {
    pub use crate::runtime_host::{
        alert_service_descriptor, bootstrap_default_application_execution_service,
        bootstrap_driver_service, bootstrap_interaction_service,
        bootstrap_local_app_protocol_service, bootstrap_local_approval_service,
        bootstrap_local_code_intelligence_service, bootstrap_local_config_service,
        bootstrap_local_diagnostics_service, bootstrap_local_file_service,
        bootstrap_local_git_service, bootstrap_local_hook_service,
        bootstrap_local_plugin_marketplace_service, bootstrap_local_process_service,
        bootstrap_local_remote_environment_service, bootstrap_local_review_service,
        bootstrap_local_sandbox_service, bootstrap_local_skill_assets,
        bootstrap_local_skill_service_provider, bootstrap_local_task_service,
        bootstrap_optional_services, bootstrap_tool_planning_service,
        bootstrap_unavailable_realtime_service, industrial_tool_planning_service,
        mcp_service_descriptor, AlertSystemServiceProvider, ContextSystemServiceProvider,
        LlmProviderCatalogProfile, LlmProviderProfile, LlmSystemServiceProvider,
        McpSystemServiceProvider, MemorySystemServiceProvider, OptionalServiceBootstrapInputs,
    };
}

/// Runtime-host tool assembly helpers used by shell composition roots.
///
/// Concrete tool construction remains runtime-host owned; shells provide only
/// generic ports, workspace roots, and trace labels.
pub mod tool_bootstrap {
    pub use crate::runtime_host::{
        bootstrap_delegate_task_tool, bootstrap_get_task_result_tool, bootstrap_list_agents_tool,
        bootstrap_local_base_tools, bootstrap_task_toolkit_tools,
        bootstrap_workspace_toolkit_tools, normalize_workspace_tool_input,
        resolve_workspace_tool_path, DelegateTaskToolBootstrapPorts, ForkSessionMappingRecorder,
        GoalRecordedObserver, TaskToolkitBootstrapRequest, TaskToolkitPolicy,
        WorkspaceToolkitBootstrapRequest,
    };
}

/// Memory service surface currently exposed for shell composition roots.
pub mod memory {
    pub use crate::runtime_host::{
        memory_service_descriptor, ActiveRecallCapability, ActiveRecallRequest, ActiveRecallResult,
        ConfiguredMemoryManager, DefaultActiveRecallStrategy, FabricMemoryRuntime, FileMemory,
        ForgetMemory, InMemoryVectorStore, KnowledgeCompileCapability, KnowledgeCompileRequest,
        KnowledgeCompileResult, KnowledgeCompiler, MemoryBackendConfig, MemoryBackendFactory,
        MemoryCapabilitySet, MemoryDeleteRequest, MemoryFacade, MemoryForgetCommand,
        MemoryGetCommand, MemoryGetRequest, MemoryGetResult, MemoryPolicyHints,
        MemoryPrefetchCommand, MemoryPrefetchRequest, MemoryRecallCommand, MemoryRecallResult,
        MemoryRememberCommand, MemoryRememberResult, MemoryRuntimeFacade, MemoryRuntimeStatus,
        MemoryScope, MemorySearchRequest, MemoryServiceSnapshot, MemoryServiceSnapshotCommand,
        MemoryStatusCommand, MemoryStatusReport, MemoryWriteRequest, MockEmbedding, RecallQuery,
        RecallResult, RememberText, SessionMemory, SharedTombstoneRegistry, TestMemoryManager,
        TombstoneIndex, MEMORY_SERVICE_ID,
    };
}

/// Task service surface currently exposed for shell composition roots.
pub mod task {
    pub use macaca_proto::{
        diagnose_session_claims, BuildDecompositionPromptCommand, BuildGoalEvaluationPromptCommand,
        ClaimTaskCommand, CompleteGoalCommand, CreateGoalCommand, CreateTaskAssignmentCommand,
        FailTaskCommand, GoalEvaluationResult, ParseGoalEvaluationCommand, PlanEvent,
        QueryAgentTodosCommand, QueryTaskBoardCommand, QueryTaskClaimDiagnosticsCommand,
        QueryTaskGoalsCommand, QueryTaskProgressCommand, ResumeCoordinatorCommand,
        ReviewTaskCommand, SessionClaimDiagnostics, StartTaskCommand, SubmitReviewCommand,
        TaskBoardQueryResult, TaskGoalsResult, TaskProgressSummary, TaskPromptResult,
        TaskServiceSnapshot, TaskServiceSnapshotCommand, TaskSummary, WorkerEvent,
    };
}

/// Tool service surface currently exposed for shell composition roots.
pub mod tools {
    pub use crate::runtime_host::{
        CheckTodoProgressTool, ClaimTaskTool, CompositeToolSet, CreateGoalTool, CreateTodoTool,
        CreateTodosTool, DefaultToolSet, DelegateTaskTool, FileReadTool, FileWriteTool,
        GetTaskResultCallback, GetTaskResultTool, ListAgentsTool, ListMyTasksTool, OnGoalCreated,
        OnGoalRecorded, OnTodoReviewed, ReassignTaskTool, ReviewTodoTool, ShellTool, StartTaskTool,
        SubmitTaskForReviewTool, TaskResultData, TodoStore, Tool, ToolCatalog, ToolCommand,
        ToolCommandContext, ToolCommandExecutor, ToolCommandMiddleware, ToolCommandPipeline,
        ToolSchemaProvider, TraceEvent, TraceToolCommandMiddleware, UpdateTaskProgressTool,
    };
}

/// Context service surface currently exposed for shell composition roots.
pub mod context {
    pub use crate::runtime_host::{
        active_recall_degraded, active_recall_diagnostics_from_prefetch,
        assemble_context_providers, estimate_text_tokens, filter_digest_items_by_tombstones,
        list_builtin_engine_infos, list_builtin_family_descriptors, mcp_tool_collisions,
        memory_source, render_active_recall_fence, ActiveRecallDiagnostics, ActiveRecallPolicy,
        BudgetPolicy, CapabilityCandidate, CapabilityCollisionRecord, CompactionDecision,
        CompactionSummaryEnvelope, ConfidenceScore, ContextAdapterSafetyPolicy,
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
        DefaultActiveRecallProvider, DefaultBudgetPolicy, DefaultContextComposer,
        DefaultPruningPolicy, DefaultSourceRenderer, DigestStrengthSnapshot,
        ExternalAdapterContextEngine, ExternalContextAdapter, ExternalContextAdapterInfo,
        GovernedCollection, KnowledgeDigestCapability, KnowledgeDigestContextProvider,
        KnowledgeDigestItem, McpCapabilityCatalog, McpContextProvider, McpServerCapabilitySummary,
        MemoryActiveRecallContextProvider, MemoryPrefetchResult, MemoryProviderHookInput,
        MemoryRecallItem, MemoryRecallQuery, MemorySourceProvider, NoopCompactionLifecycleHook,
        OpaqueExternalPayload, PassthroughContextEngine, PrivacyTier, ProfileFileContextProvider,
        PromptComposer, PromptSection, PromptSectionBuilder, PromptStability,
        ProviderAssemblyEnvironment, ProviderFactoryInput, ProviderFamilyDescriptor,
        ProviderHealthLedger, ProviderHealthSnapshot, ProviderInvocationSummary,
        ProviderRuntimeSummary, PruningContextEngine, PruningDecision, PruningPolicy,
        RecallCandidate, RecallDecision, RuntimeToolCapabilityCatalog,
        RuntimeToolCapabilityProvider, SkillCapabilityCatalog, SkillCapabilityGovernanceReport,
        SkillCapabilityRecord, SkillContextProvider, SkillFilterDiagnostic, SummaryContextEngine,
        ThresholdCompactionPolicy, ToolCapabilityIndex, ToolCapabilityReport, TrustLevel,
        VersionedContextProvider, WindowedContextEngine, CONTEXT_ACTIVE_RECALL_COMMAND,
        CONTEXT_ASSEMBLE_COMMAND, CONTEXT_ENGINE_INVENTORY_COMMAND,
        CONTEXT_PROVIDER_INVENTORY_COMMAND, CONTEXT_SERVICE_ID, CONTEXT_SNAPSHOT_COMMAND,
    };
    pub use crate::runtime_host::{
        ContextActiveRecallBudget as ActiveRecallBudget,
        ContextActiveRecallCapability as ActiveRecallCapability,
    };

    /// Context catalog constants kept as a narrow SDK path for shell adapters.
    pub mod catalog {
        /// Provider-family constants used by context lifecycle visibility code.
        pub mod constants {
            pub use crate::runtime_host::{
                FAMILY_AGENT_PROFILE, FAMILY_KNOWLEDGE_DIGEST, FAMILY_MCP_CAPABILITY,
                FAMILY_MEMORY_ACTIVE_RECALL, FAMILY_RUNTIME_TOOL_CAPABILITY,
                FAMILY_SKILL_CAPABILITY, FAMILY_TOOL_CAPABILITY,
            };
        }
    }
}

/// Agent framework surface currently exposed for shell composition roots.
pub mod framework {
    /// Agent traits and hook adapters still consumed by framework shell adapters.
    pub mod agent {
        pub use crate::runtime_host::{
            Agent, AgentError, AgentResult, Hook, HookRegistry, HookedAgent,
        };
    }

    /// Agent construction DTOs used while Web construction ownership is retired.
    pub mod construction {
        pub use crate::runtime_host::{
            AgentBuildIntent, AgentBuildRequest, AgentBuildRequestBuilder, AgentCapabilitySet,
            AgentExecutionInput, AgentExecutionLauncher, AgentExecutionOutput, AgentIdentity,
            AgentLifecycleConfig, AgentServices, AgentToolConfig, AgentTraceContext,
            AgentTransitionReason, ApplicationPromptParts, ApplicationSemantics,
            ApplicationToolPolicy, ToolkitContributor, TracedAgentFactory,
        };
    }

    /// Request execution state carried across shell and runtime-host adapters.
    pub mod execution {
        pub use crate::runtime_host::{ExecutionContext, ExecutionStatus};
    }

    /// Formatter Strategies used by the current framework runner factory.
    pub mod formatter {
        pub use crate::runtime_host::{
            AnthropicFormatter, DashScopeFormatter, Formatter, FormatterError, OpenAiFormatter,
        };
    }

    /// OpenAI-style chat JSON bridge used by context/reporting adapters.
    pub mod llm_wire {
        pub use crate::runtime_host::{
            chat_options_to_llm_options, llm_messages_to_json_values, messages_from_json_values,
        };
    }

    /// MCP DTOs needed by shell capability tests and service probes.
    pub mod mcp {
        pub use crate::runtime_host::{
            client_from_transport, register_mcp_tools, register_mcp_tools_with_options,
            McpCallResult, McpClient, McpError, McpResourceDef, McpResourceRead,
            McpResourceTemplateDef, McpSessionMode, McpTimeouts, McpToolDef, McpToolHandler,
            McpToolNameConflictPolicy, McpToolRegistrationOptions, McpTransportConfig,
        };
    }

    /// AgentScope-style message DTOs exposed to shell event adapters.
    pub mod message {
        pub use crate::runtime_host::{
            AudioBlock, ContentBlock, DataBlock, GenerateReason, HintBlock, ImageBlock,
            MessageUsage, Msg, MsgContent, MsgValidationError, Role, TextBlock, ThinkingBlock,
            ToolCallState, ToolResultBlock, ToolResultState, ToolUseBlock, VideoBlock,
        };
    }

    /// Chat model contracts used by framework runners and context wrappers.
    pub mod model {
        pub use crate::runtime_host::{
            ChatModel, ChatOptions, ChatResponse, ChatUsage, ModelError, ToolChoice,
        };
    }

    /// Planner notebook DTOs still stored by shell loop projections.
    pub mod plan {
        pub use crate::runtime_host::{
            Plan, PlanError, PlanNotebook, PlanState, SubTask, SubTaskState,
        };
    }

    /// ReAct agent implementation used until construction fully moves behind runtime-host services.
    pub mod react_agent {
        pub use crate::runtime_host::{AgentRunCommand, AgentRunResult, ReActAgent};
    }

    /// Runtime context and session-store contracts used by shell state adapters.
    pub mod runtime_context {
        pub use crate::runtime_host::{
            AgentSessionError, AgentSessionHealth, AgentSessionHealthState, AgentSessionStore,
            AgentSessionStoreSnapshot, AgentState, CachedRead, InMemoryAgentSessionStore,
            PendingToolState, PermissionState, PlanModeState, RuntimeContext, RuntimeContextError,
            SessionKey, TaskContextState,
        };
    }

    /// Toolkit contracts used by tool assembly and capability catalog adapters.
    pub mod tool {
        pub use crate::runtime_host::{
            RegisteredTool, ToolError, ToolGroup, ToolHandler, ToolMiddleware, ToolResponse,
            ToolTraceEvent, Toolkit, ToolkitResource,
        };
    }
}

/// Application ABI and package surface currently exposed for shell composition roots.
pub mod app {
    pub use crate::runtime_host::{
        app_agent_manifest_view, app_agent_prompt_semantics, app_entry_agent_name,
        app_task_planning_contract, application_service_descriptor, discovered_app_agent_names,
        expand_service_capabilities, AppLayer, AppLlmConfig, AppLoader, AppRegistry, AppRuntime,
        DiscoveredApp,
    };

    /// Application manifest model DTOs used by framework and shell adapters.
    pub mod model {
        pub use crate::runtime_host::{AgentSource, AppContextConfig};
    }

    /// Application-owned UI runtime DTOs consumed by UI route adapters.
    pub mod ui_runtime {
        pub use crate::runtime_host::{
            validate_ui_runtime_config, AppUiBridgeConfig, AppUiCspMode, AppUiFramework,
            AppUiNetworkPolicy, AppUiPresentationConfig, AppUiRuntimeConfig, AppUiRuntimeKind,
            AppUiSandboxConfig, AppUiSandboxIsolation, AppUiSurfaceChrome, AppUiSurfaceConfig,
            AppUiSurfaceMode, AppUiThemeConfig, AppUiThemeMode,
        };
    }
}
