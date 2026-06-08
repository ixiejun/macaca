//! Stable public re-export surface for `macaca-runtime-host`.
//!
//! The crate root (`lib.rs`) owns module declarations and test wiring.  This
//! Facade module centralizes outward-facing `pub use` aliases so composition
//! roots import one predictable surface without growing the root file past the
//! file-size governance gate.

pub use crate::agent_context_service_provider::{
    agent_context_service_descriptor, AgentContextBackend, AgentContextSystemServiceProvider,
};
pub use crate::agent_execution_dispatch::{
    kernel_execution_port_from_agent_execution_service, service_client_execution_port,
    wire_kernel_to_agent_execution_service, ServiceRuntimeAgentExecutionDispatch,
};
pub use crate::agent_execution_orchestration::{
    build_agent_context_snapshot_via_service, execution_control_execution_id,
    execution_control_scope, extract_single_shell_fence, failed_execution_result,
    heartbeat_exact_shell_contract, resolve_execution_control_policy_local,
    runtime_agent_max_iters, runtime_agent_tool_choice, should_skip_heartbeat_without_source,
    structured_execution_context, user_prompt_with_context,
    ARTIFACT_BACKED_RUNTIME_AGENT_MAX_ITERS, DEFAULT_RUNTIME_AGENT_MAX_ITERS,
};
pub use crate::agent_execution_ports::{
    AgentExecutionEvidenceCollector, AgentExecutionHostAdapter, AgentExecutionOutputHasher,
    ConstructedRuntimeAgent, FrameworkAgentConstructionPort, FrameworkRuntimeAgentPort,
    OpaqueExecutionControlHandle,
};
pub use crate::framework_runtime_agent_service::ServiceBackedFrameworkRuntimeAgentPort;
pub use crate::agent_execution_service_provider::{
    agent_execution_service_descriptor, AgentExecutionBackend, AgentExecutionSystemServiceProvider,
};
pub use crate::app_protocol_service_commands::app_protocol_service_command;
pub use crate::app_protocol_service_provider::{
    bootstrap_local_app_protocol_service, AppProtocolSystemServiceProvider,
};
pub use crate::application_execution_event_store::ApplicationExecutionEventStore;
pub use crate::application_execution_external_backend::ExternalApplicationBackendProvider;
pub use crate::application_execution_external_backend_transport::{
    ExternalApplicationBackendControlRequest, ExternalApplicationBackendStartRequest,
    ExternalApplicationBackendTransport, HttpExternalApplicationBackendTransport,
};
pub use crate::application_execution_hosted::{
    ApplicationAbiHostedExecutionAdapter, HostedApplicationExecutionAdapter,
    HostedApplicationExecutionOutcome, MacacaHostedApplicationExecutionProvider,
};
pub use crate::application_execution_projection::project_application_execution_state;
pub use crate::application_execution_provider_registry::{
    ApplicationExecutionProvider, ApplicationExecutionProviderRegistry,
    ApplicationExecutionProviderSelection, ApplicationExecutionProviderSelectionCriteria,
    ApplicationExecutionProviderSelectionPolicy, ApplicationExecutionProviderSelectionRecord,
};
pub use crate::application_execution_remote_agent::{
    RemoteAgentApplicationExecutionProvider, RemoteAgentExecutionTransport,
    RemoteAgentRegistration, RemoteAgentRegistry, RemoteAgentTransportMetadata,
    UnavailableRemoteAgentExecutionTransport,
};
pub use crate::application_execution_service_host::ServiceBackedApplicationHostRuntime;
pub use crate::application_execution_service_provider::{
    application_execution_service_descriptor_runtime, application_execution_service_id,
    bootstrap_application_execution_service, bootstrap_default_application_execution_service,
    bootstrap_unavailable_application_execution_service, ApplicationExecutionSystemServiceProvider,
};
pub use crate::application_hosts::{
    is_application_runtime_unavailable, ApplicationHostRuntime, UnavailableApplicationRuntimeHost,
    UnavailableWasmApplicationHost, WasmApplicationHostFactory,
};
pub use crate::application_service_provider::{
    ApplicationOrchestrationBackend, ApplicationSystemServiceProvider,
};
pub use crate::approval_service_provider::{
    bootstrap_local_approval_service, ApprovalReviewerPolicy, ApprovalSystemServiceProvider,
    LocalApprovalReviewerPolicy,
};
pub use crate::autonomy_evolution_live_executor::{
    AutonomyEvolutionLiveExecutionCommand, AutonomyEvolutionLiveExecutionResult,
    AutonomyEvolutionLiveExecutor, AutonomyEvolutionTargetExecutionOutcome,
};
pub use crate::autonomy_evolution_service_provider::AutonomyEvolutionSystemServiceProvider;
pub use crate::autonomy_runtime_config::{AutonomyProviderMode, AutonomyRuntimeConfig};
pub use crate::autonomy_service_provider::{
    bootstrap_autonomy_local_services, bootstrap_autonomy_services,
    bootstrap_autonomy_unavailable_services, AutonomyRuntimeBundle, HostHeartbeatServiceAdapter,
    ScheduledAgentTaskSystemServiceProvider, HostSchedulerServiceAdapter,
};
pub use crate::autonomy_supervisor::AutonomyLifecycleCoordinator;
pub use crate::code_intelligence_service_provider::{
    bootstrap_local_code_intelligence_service, CodeIntelligenceProvider,
    CodeIntelligenceSystemServiceProvider, LocalCodeIntelligenceProvider,
};
pub use crate::composed_agent_execution_backend::ComposedAgentExecutionBackend;
pub use crate::config_service_provider::{bootstrap_local_config_service, ConfigSystemServiceProvider};
pub use crate::context_service_provider::ContextSystemServiceProvider;
pub use crate::diagnostics_service_provider::{
    bootstrap_local_diagnostics_service, DiagnosticsSystemServiceProvider,
};
#[allow(deprecated)]
pub use crate::domain_pack_service_provider::{
    bootstrap_builtin_domain_pack_services, bootstrap_domain_pack_services,
    DomainPackProviderRegistration, DomainPackRuntimeBundle,
};
pub use crate::driver_service_provider::DriverSystemServiceProvider;
pub use crate::entitlement::{CapabilityCallContext, EntitlementOperation, EntitlementRuntimeFacade};
pub use crate::entitlement_service_provider::{
    entitlement_service_descriptor, EntitlementSystemServiceProvider,
};
#[allow(deprecated)]
pub use crate::env_bridge::{apply_mcp_env, McpEnvApplyOutcome};
pub use crate::evm_service_provider::{
    evm_service_descriptor, EvmProviderStrategy, EvmSystemServiceProvider, MockEvmProvider,
    UnavailableEvmProvider,
};
pub use crate::execution_control::ExecutionControlPolicyResolver;
pub use crate::execution_control_fork_join::{
    ExecutionControlForkJoinCoordinator, ForkJoinChildForkRequest, ForkJoinParentResumeRequest,
    ForkJoinParentWaitRequest, FORK_JOIN_EXECUTION_CONTROL_SOURCE,
};
pub use crate::execution_control_goal_lifecycle::{
    ExecutionControlGoalLifecycleCoordinator, GoalLifecycleParentResumeRequest,
    GoalLifecycleParentWaitRequest, GOAL_LIFECYCLE_EXECUTION_CONTROL_SOURCE,
};
pub use crate::execution_control_session_loop::{
    ExecutionControlSessionLoopCoordinator, SessionLoopKind, SessionLoopRegisterRequest,
    SessionLoopShutdownRequest, SessionLoopWakeRequest, SESSION_LOOP_EXECUTION_CONTROL_SOURCE,
    SESSION_LOOP_PLAN_REGISTER_EVENT, SESSION_LOOP_PLAN_WAKE_EVENT,
    SESSION_LOOP_SHUTDOWN_EVENT, SESSION_LOOP_TASK_CAPABILITY_ID,
    SESSION_LOOP_WORKER_REGISTER_EVENT, SESSION_LOOP_WORKER_WAKE_EVENT,
};
pub use crate::execution_control_runtime::{
    ExecutionControlExecutionSnapshot, ExecutionControlObserver, ExecutionControlRuntimeCapability,
    ExecutionControlRuntimeSnapshot, NoopExecutionControlObserver,
};
pub use crate::execution_control_service_provider::{
    execution_control_service_descriptor, ExecutionControlSystemServiceProvider,
};
pub use crate::factory::{McpServerFactory, RuntimeEnvBuilder};
pub use crate::file_service_local::{FileProvider, LocalFileProvider};
pub use crate::file_service_provider::{bootstrap_local_file_service, FileSystemServiceProvider};
pub use crate::genui_surface_store::ApplicationGenUiSurfaceStore;
pub use crate::git_service_provider::{
    bootstrap_local_git_service, GitProvider, GitSystemServiceProvider, LocalGitProvider,
};
pub use crate::hook_service_provider::{bootstrap_local_hook_service, HookSystemServiceProvider};
pub use crate::interaction_ledger_store::{InteractionLedgerStore, PersistInteractionLedgerStore};
pub use crate::interaction_service_bootstrap::bootstrap_interaction_service;
pub use crate::interaction_service_provider::{
    interaction_service_command, InteractionLedgerEvent, InteractionSystemServiceProvider,
};
pub use crate::lease::McpSessionLease;
pub use crate::llm_service_catalog::{LlmProviderCatalogProfile, LlmProviderProfile};
pub use crate::llm_service_provider::LlmSystemServiceProvider;
#[allow(deprecated)]
pub use crate::mcp_runtime::{
    apply_concurrency_isolation, probe_definition_statuses, ConcurrencyIsolationPolicy,
    McpDefinitionSource, McpLifecycleScope, McpRegistryConfig, McpRuntimeContext, McpRuntimeFacade,
    McpRuntimeManager, McpRuntimeStatus, McpRuntimeStatusState, McpServerDefinition, McpToolPolicy,
};
pub use crate::mcp_service_provider::{mcp_service_descriptor, McpSystemServiceProvider};
pub use crate::memory_service_provider::MemorySystemServiceProvider;
pub use crate::package::{runtime_host_mcp_package_descriptor, RuntimeHostPackageRequirement};
pub use crate::payment_adapter::{LocalSimulatedPaymentAdapter, PaymentAdapterStrategy};
pub use crate::payment_policy::{DefaultPaymentPolicyEngine, PaymentPolicyDecision, PaymentPolicyEngine};
pub use crate::payment_service_provider::{payment_service_descriptor, PaymentSystemServiceProvider};
pub use crate::plugin::{
    plugin_failure_event, DescriptorPluginHost, PluginHost, PluginHostFactory,
    PluginLifecycleController, PluginManifestValidator, PluginRuntimeFacade, PluginRuntimeGuard,
    PluginRuntimeResult,
};
pub use crate::plugin_capability::{
    built_in_service_capability, detect_slot_conflict, discover_manifest_capabilities,
    CapabilityCallAdmissionChain, CapabilityCallAdmissionCheck, CapabilityCallAdmissionContext,
    CapabilityCallAdmissionDecision, CapabilityHintAdmissionCheck,
    CapabilityOwnershipAdmissionCheck, FailClosedSlotConflictPolicy,
    PluginCapabilityConflictPolicy, PluginCapabilityRegistry, PluginCapabilityRegistryBuilder,
    PluginCapabilityService, PluginCapabilityServiceBuilder,
};
pub use crate::plugin_capability_service_provider::{
    plugin_capability_service_command, plugin_capability_service_descriptor,
    PluginCapabilitySystemServiceProvider,
};
pub use crate::plugin_control::{
    AdmissionCheck, AdmissionContext, AdmissionDecision, CompatibilityAdmissionCheck,
    InMemoryPluginRepository, ManifestShapeAdmissionCheck, PluginAdmissionChain,
    PluginControlService, PluginControlServiceBuilder, PluginRepository, PluginRepositoryMutation,
    PluginRepositorySnapshot, SourcePolicyAdmissionCheck,
};
pub use crate::plugin_control_service_provider::{
    plugin_control_service_command, plugin_control_service_descriptor,
    PluginControlSystemServiceProvider,
};
pub use crate::plugin_hook::{
    DefaultPluginHookFailureStrategy, DefaultPluginHookTimeoutStrategy,
    DescriptorOnlyPluginHookExecutor, PluginHookBus, PluginHookBusBuilder, PluginHookExecutor,
    PluginHookFailureStrategy, PluginHookRegistry, PluginHookRegistryBuilder, PluginHookRunner,
    PluginHookTimeoutStrategy,
};
pub use crate::plugin_hook_service_provider::{
    plugin_hook_service_command, plugin_hook_service_descriptor, PluginHookSystemServiceProvider,
};
pub use crate::plugin_hosts::{
    BuiltInAdapterPluginRuntimeHost, DescriptorPluginRuntimeHost, PluginHostLifecycleSupervisor,
    PluginHostRuntime, PluginHostRuntimeFactory, PluginHostRuntimeResult, ProcessPluginRuntimeHost,
    RemoteProxyPluginRuntimeHost, UnavailablePluginRuntimeHost, WasmPluginRuntimeHost,
};
pub use crate::plugin_marketplace_service_provider::{
    bootstrap_local_plugin_marketplace_service, PluginMarketplaceSystemServiceProvider,
};
pub use crate::process_service_local::{LocalProcessProvider, ProcessProvider};
pub use crate::process_service_provider::{bootstrap_local_process_service, ProcessSystemServiceProvider};
pub use crate::realtime_service_provider::{
    bootstrap_unavailable_realtime_service, RealtimeProvider, RealtimeSystemServiceProvider,
};
pub use crate::remote_environment_service_provider::{
    bootstrap_local_remote_environment_service, RemoteEnvironmentSystemServiceProvider,
};
pub use crate::review_service_provider::{
    bootstrap_local_review_service, LocalReviewProvider, ReviewProvider,
    ReviewSystemServiceProvider,
};
pub use crate::route_c_bootstrap::{
    bootstrap_route_c_optional_services, RouteCBootstrapDiagnostic, RouteCHostRuntimeBundle,
    RouteCOptionalServicesBootstrap, RouteCOptionalServicesBootstrapInputs,
};
pub use crate::sandbox_service_local::{LocalSandboxProvider, MockSandboxProvider, SandboxProvider};
pub use crate::sandbox_service_provider::{bootstrap_local_sandbox_service, SandboxSystemServiceProvider};
pub use crate::service_audit_runtime_bundle::ServiceAuditRuntimeBundle;
pub use crate::service_call_audit::{
    InMemoryServiceCallAuditSink, ServiceCallAuditEvent, ServiceCallAuditSink,
};
pub use crate::service_call_audit_service_provider::{
    service_call_audit_replay_session_command, service_call_audit_replay_trace_command,
    service_call_audit_service_descriptor, ServiceCallAuditReplayResult,
    ServiceCallAuditReplaySessionCommand, ServiceCallAuditReplayTraceCommand,
    ServiceCallAuditSystemServiceProvider, SERVICE_CALL_AUDIT_REPLAY_SESSION_COMMAND,
    SERVICE_CALL_AUDIT_REPLAY_TRACE_COMMAND, SERVICE_CALL_AUDIT_SERVICE_ID,
};
pub use crate::service_contract_registry::{
    InMemoryServiceContractRegistry, ServiceContractDescriptor, ServiceContractRegistry,
};
pub use crate::service_decorator::{
    AllowAllServiceRuntimePolicy, DenyAllServiceRuntimePolicy, EntitlementRuntimeDecorator,
    MeteringRuntimeDecorator, PolicyRuntimeDecorator, ResourceRuntimeDecorator,
    ServiceRuntimeCallContext, ServiceRuntimeDecorator, ServiceRuntimePolicy,
    ServiceRuntimePolicyDecision, TraceRequiredRuntimeDecorator,
};
pub use crate::service_policy_engine::{
    InMemoryServicePolicyEngine, ServicePolicyDecision, ServicePolicyEngine, ServicePolicyInput,
    ServicePolicyLayer,
};
pub use crate::service_provider::{
    ServiceProviderFactory, ServiceProviderFactoryContext, ServiceProviderInstance,
    StaticServiceProviderFactory,
};
pub use crate::service_provider_selector::{
    ProviderSelectionStrategy, ProviderSelector, ProviderSnapshot,
};
pub use crate::service_router::{ServiceRouteRequest, ServiceRouteResponse, ServiceRouter};
pub use crate::service_runtime::ServiceRuntime;
pub use crate::service_runtime_error::{ServiceRuntimeConfig, ServiceRuntimeError};
pub use crate::service_runtime_event::{
    InMemoryServiceRuntimeEventSink, ServiceRuntimeEvent, ServiceRuntimeEventSink,
    ServiceRuntimeServiceSnapshot, ServiceRuntimeSnapshot,
};
pub use crate::skill_service_provider::SkillSystemServiceProvider;
pub use crate::store_service_provider::{store_service_descriptor, StoreSystemServiceProvider};
pub use crate::task_service_provider::{
    bootstrap_local_task_service, TaskSystemServiceProvider, TASK_CLAIM_COMMAND,
    TASK_CREATE_GOAL_COMMAND, TASK_QUERY_COMMAND, TASK_RESUME_COORDINATOR_COMMAND,
    TASK_REVIEW_COMMAND, TASK_SNAPSHOT_COMMAND, TASK_START_COMMAND, TASK_SUBMIT_REVIEW_COMMAND,
};
pub use crate::tool_family_providers::{
    industrial_tool_family_provider_contributor, industrial_tool_family_provider_inventory,
    industrial_tool_family_toolsets, industrial_tool_planning_service,
    IndustrialToolFamilyProviderInventory, REQUIRED_INDUSTRIAL_TOOL_FAMILIES,
};
pub use crate::tool_service_availability::{AvailabilitySignalSet, ToolAvailabilityEvaluator};
pub use crate::tool_service_environment::{
    industrial_tool_runtime_environment_service, StaticToolRuntimeEnvironmentProvider,
    ToolRuntimeEnvironmentInvocation, ToolRuntimeEnvironmentProvider,
    ToolRuntimeEnvironmentService, UnavailableToolRuntimeEnvironmentProvider,
};
pub use crate::tool_service_gateway::{
    industrial_tool_managed_gateway_service, StaticToolManagedGatewayProvider,
    ToolManagedGatewayInvocation, ToolManagedGatewayProvider, ToolManagedGatewayService,
    UnavailableToolManagedGatewayProvider,
};
pub use crate::tool_service_planning::{
    CapabilityToolDescriptorContributor, StaticToolDescriptorContributor,
    ToolDescriptorContributor, ToolPlanningService, ToolPlanningToolsetResolver,
};
pub use crate::tool_service_provider::{bootstrap_tool_planning_service, ToolSystemServiceProvider};
pub use crate::transport::{ConfigBackedMcpTransport, McpTransport};
pub use crate::wasm_runtime_provider::{
    ComponentModelWasmRuntimeProvider, DefaultInProcessWasmRuntimeProvider,
    UnavailableWasmRuntimeProvider, WasmApplicationRuntimeProvider, WasmCertificationFixtureSet,
    WasmCertificationHarness, WasmCertificationProfile, WasmCertificationReport,
    WasmCertificationStatus, WasmConformanceFixtureKind, WasmExampleFixtureKind,
    WasmExecutionSession, WasmGuestHarnessFixture, WasmGuestHarnessReport, WasmGuestRuntimeHarness,
    WasmHardenedProviderEnvelope, WasmHardenedProviderMockAdapter, WasmHardenedProviderResponse,
    WasmMockHostOutcome, WasmRuntimeProviderRegistry, WasmTelemetryEvent, WasmTelemetrySink,
    WasmTelemetrySinkRef, WasmTelemetryStage, WasmToolchainFixtureReport,
};
pub use crate::web3_service_provider::{
    web3_service_descriptor, MockWeb3Provider, UnavailableWeb3Provider, Web3ProviderStrategy,
    Web3SystemServiceProvider,
};

pub use crate::executor::{
    AgentInfo, AgentRunner, ApplicationExecutor, ApplicationExecutorConfig,
    ApplicationExecutorRegistry, CallbackDispatcher, DelegateResult, DelegatedTask, EventBus,
    ExecutionQueue, ExecutorCommand, ExecutorEvent, ExecutorEventFactory, ForkContext, ForkManager,
    HookEvent, MergeResult, RoutingDecision, SystemEvent, TaskContext, TaskExecutor, TaskId,
    TaskResult, TaskRouter, TaskStatus, TokenUsage,
};

/// Embedded persistence crate alias for presentation-shell composition roots.
///
/// Presentation shells must not declare a direct `macaca-persist` dependency (Route C gate).
/// The SDK cannot re-export persist without violating
/// `application-execution-sdk-no-runtime-provider-construction`, so shells that already
/// depend on runtime-host for bootstrap wiring reach persistence types through this alias.
/// Runtime behavior must still flow through service clients at the HTTP/SSE boundary.
pub use macaca_persist as persist;
