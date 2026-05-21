//! Shell-facing system facade for Web and CLI thin-shell migration.
//!
//! `SystemFacade` is the SDK-owned Facade that upper presentation shells call
//! instead of reaching into provider crates or Web internals. The facade is
//! deliberately composed from small Strategy clients so each capability can be
//! migrated to `ServiceRuntime` independently in later Route C phases.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::info;

use macaca_proto::MacacaResult;

pub use crate::application_client::{SystemApplicationClient, UnavailableSystemApplicationClient};
pub use crate::context_client::{SystemContextClient, UnavailableSystemContextClient};
pub use crate::driver_client::{SystemDriverClient, UnavailableSystemDriverClient};
pub use crate::entitlement_client::{SystemEntitlementClient, UnavailableSystemEntitlementClient};
pub use crate::evm_client::{SystemEvmClient, UnavailableSystemEvmClient};
pub use crate::heartbeat_client::{SystemHeartbeatClient, UnavailableSystemHeartbeatClient};
pub use crate::llm_client::{SystemLlmClient, UnavailableSystemLlmClient};
pub use crate::mcp_client::{SystemMcpClient, UnavailableSystemMcpClient};
pub use crate::memory_client::{SystemMemoryClient, UnavailableSystemMemoryClient};
pub use crate::package_client::{
    EmptySystemPackageClient, PackageInspectionCommand, PackageInspectionResult,
    SystemPackageClient,
};
pub use crate::payment_client::{SystemPaymentClient, UnavailableSystemPaymentClient};
pub use crate::scheduler_client::{SystemSchedulerClient, UnavailableSystemSchedulerClient};
pub use crate::service_client::{
    ServiceCallCommand, ServiceCallResult, ServiceInspectionCommand, ServiceInspectionResult,
    SystemServiceClient, UnavailableSystemServiceClient,
};
pub use crate::skill_client::{SystemSkillClient, UnavailableSystemSkillClient};
pub use crate::status_client::{
    kernel_status_snapshot, StaticSystemStatusDataSource, SystemStatusClient,
    SystemStatusDataSource, SystemStatusSnapshot,
};
pub use crate::store_client::{SystemStoreClient, UnavailableSystemStoreClient};
pub use crate::task_client::{
    SystemTaskClient, TaskBoardDataSource, TaskBoardQueryCommand, TaskBoardQueryResult,
    TodoStoreTaskBoardDataSource,
};
pub use crate::trace_client::{
    EmptySystemTraceClient, SessionEventQueryCommand, SystemTraceClient, TraceQueryResult,
    TraceTailCommand,
};
pub use crate::web3_client::{SystemWeb3Client, UnavailableSystemWeb3Client};

/// Policy-ready approval command produced by Web/CLI confirmation surfaces.
///
/// Approval processing still belongs to policy/runtime services. S3 keeps this
/// command in the SDK so presentation shells can produce an auditable decision
/// object without becoming the policy engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalDecisionCommand {
    pub decision_id: String,
    pub approved: bool,
    pub decided_at: DateTime<Utc>,
}

/// SDK system facade consumed by Web/CLI thin shells.
///
/// The type parameters are intentionally capability-scoped Strategy clients.
/// Existing two-client call sites keep working through default service, trace,
/// and package clients that return structured empty/unavailable responses until
/// later Route C phases attach concrete runtime-backed implementations.
pub struct SystemFacade<
    T,
    S,
    SV = UnavailableSystemServiceClient,
    TR = EmptySystemTraceClient,
    P = EmptySystemPackageClient,
    L = UnavailableSystemLlmClient,
    M = UnavailableSystemMemoryClient,
    C = UnavailableSystemContextClient,
    D = UnavailableSystemDriverClient,
    SK = UnavailableSystemSkillClient,
    MCP = UnavailableSystemMcpClient,
    A = UnavailableSystemApplicationClient,
    ST = UnavailableSystemStoreClient,
    E = UnavailableSystemEntitlementClient,
    PMT = UnavailableSystemPaymentClient,
    W3 = UnavailableSystemWeb3Client,
    EVM = UnavailableSystemEvmClient,
    SCH = UnavailableSystemSchedulerClient,
    HB = UnavailableSystemHeartbeatClient,
> {
    task_board: T,
    status: S,
    service: SV,
    trace: TR,
    package: P,
    llm: L,
    memory: M,
    context: C,
    driver: D,
    skill: SK,
    mcp: MCP,
    application: A,
    store: ST,
    entitlement: E,
    payment: PMT,
    web3: W3,
    evm: EVM,
    scheduler: SCH,
    heartbeat: HB,
}

impl<T, S> SystemFacade<T, S>
where
    T: SystemTaskClient,
    S: SystemStatusClient,
{
    /// Create a facade from the current compatibility task and status clients.
    ///
    /// This constructor preserves the pre-S3 Web/CLI call shape. It installs
    /// explicit empty/unavailable clients for capabilities whose service-backed
    /// implementations are intentionally deferred to later Route C phases.
    pub fn new(task_board: T, status: S) -> Self {
        Self {
            task_board,
            status,
            service: UnavailableSystemServiceClient,
            trace: EmptySystemTraceClient,
            package: EmptySystemPackageClient,
            llm: UnavailableSystemLlmClient,
            memory: UnavailableSystemMemoryClient,
            context: UnavailableSystemContextClient,
            driver: UnavailableSystemDriverClient,
            skill: UnavailableSystemSkillClient,
            mcp: UnavailableSystemMcpClient,
            application: UnavailableSystemApplicationClient,
            store: UnavailableSystemStoreClient,
            entitlement: UnavailableSystemEntitlementClient,
            payment: UnavailableSystemPaymentClient,
            web3: UnavailableSystemWeb3Client,
            evm: UnavailableSystemEvmClient,
            scheduler: UnavailableSystemSchedulerClient,
            heartbeat: UnavailableSystemHeartbeatClient,
        }
    }
}

impl<T, S, SV, TR, P>
    SystemFacade<
        T,
        S,
        SV,
        TR,
        P,
        UnavailableSystemLlmClient,
        UnavailableSystemMemoryClient,
        UnavailableSystemContextClient,
        UnavailableSystemDriverClient,
        UnavailableSystemSkillClient,
        UnavailableSystemMcpClient,
        UnavailableSystemApplicationClient,
        UnavailableSystemStoreClient,
        UnavailableSystemEntitlementClient,
        UnavailableSystemPaymentClient,
        UnavailableSystemWeb3Client,
        UnavailableSystemEvmClient,
        UnavailableSystemSchedulerClient,
        UnavailableSystemHeartbeatClient,
    >
where
    T: SystemTaskClient,
    S: SystemStatusClient,
    SV: SystemServiceClient,
    TR: SystemTraceClient,
    P: SystemPackageClient,
{
    /// Create a facade from explicit pre-S5 capability clients.
    ///
    /// This constructor keeps existing callers source-compatible and installs
    /// explicit Null Object clients for S5 capabilities until a runtime-backed
    /// composition path is provided.
    pub fn with_clients(task_board: T, status: S, service: SV, trace: TR, package: P) -> Self {
        Self {
            task_board,
            status,
            service,
            trace,
            package,
            llm: UnavailableSystemLlmClient,
            memory: UnavailableSystemMemoryClient,
            context: UnavailableSystemContextClient,
            driver: UnavailableSystemDriverClient,
            skill: UnavailableSystemSkillClient,
            mcp: UnavailableSystemMcpClient,
            application: UnavailableSystemApplicationClient,
            store: UnavailableSystemStoreClient,
            entitlement: UnavailableSystemEntitlementClient,
            payment: UnavailableSystemPaymentClient,
            web3: UnavailableSystemWeb3Client,
            evm: UnavailableSystemEvmClient,
            scheduler: UnavailableSystemSchedulerClient,
            heartbeat: UnavailableSystemHeartbeatClient,
        }
    }
}

impl<T, S, SV, TR, P, L, M, C, D, SK, MCP, A, ST, E, PMT, W3, EVM, SCH, HB>
    SystemFacade<T, S, SV, TR, P, L, M, C, D, SK, MCP, A, ST, E, PMT, W3, EVM, SCH, HB>
where
    T: SystemTaskClient,
    S: SystemStatusClient,
    SV: SystemServiceClient,
    TR: SystemTraceClient,
    P: SystemPackageClient,
    L: SystemLlmClient,
    M: SystemMemoryClient,
    C: SystemContextClient,
    D: SystemDriverClient,
    SK: SystemSkillClient,
    MCP: SystemMcpClient,
    A: SystemApplicationClient,
    ST: SystemStoreClient,
    E: SystemEntitlementClient,
    PMT: SystemPaymentClient,
    W3: SystemWeb3Client,
    EVM: SystemEvmClient,
    SCH: SystemSchedulerClient,
    HB: SystemHeartbeatClient,
{
    /// Create a facade with all current Route C capability clients installed.
    ///
    /// This constructor is the explicit composition point for serviceized
    /// runtimes.  It keeps SDK dependency injection visible and prevents the
    /// facade from constructing provider/backends internally.
    pub fn with_route_c_clients(
        task_board: T,
        status: S,
        service: SV,
        trace: TR,
        package: P,
        llm: L,
        memory: M,
        context: C,
        driver: D,
        skill: SK,
        mcp: MCP,
        application: A,
        store: ST,
        entitlement: E,
        payment: PMT,
        web3: W3,
        evm: EVM,
    ) -> SystemFacade<
        T,
        S,
        SV,
        TR,
        P,
        L,
        M,
        C,
        D,
        SK,
        MCP,
        A,
        ST,
        E,
        PMT,
        W3,
        EVM,
        UnavailableSystemSchedulerClient,
        UnavailableSystemHeartbeatClient,
    > {
        SystemFacade::<
            T,
            S,
            SV,
            TR,
            P,
            L,
            M,
            C,
            D,
            SK,
            MCP,
            A,
            ST,
            E,
            PMT,
            W3,
            EVM,
            UnavailableSystemSchedulerClient,
            UnavailableSystemHeartbeatClient,
        > {
            task_board,
            status,
            service,
            trace,
            package,
            llm,
            memory,
            context,
            driver,
            skill,
            mcp,
            application,
            store,
            entitlement,
            payment,
            web3,
            evm,
            scheduler: UnavailableSystemSchedulerClient,
            heartbeat: UnavailableSystemHeartbeatClient,
        }
    }

    /// Create a facade with Route C capability clients plus autonomy clients.
    ///
    /// This constructor keeps Scheduler and Heartbeat composition explicit.
    /// The facade receives clients as Strategy objects and never constructs
    /// providers, timers, stores, queues, or application-specific workflows.
    pub fn with_route_c_and_autonomy_clients(
        task_board: T,
        status: S,
        service: SV,
        trace: TR,
        package: P,
        llm: L,
        memory: M,
        context: C,
        driver: D,
        skill: SK,
        mcp: MCP,
        application: A,
        store: ST,
        entitlement: E,
        payment: PMT,
        web3: W3,
        evm: EVM,
        scheduler: SCH,
        heartbeat: HB,
    ) -> Self {
        Self {
            task_board,
            status,
            service,
            trace,
            package,
            llm,
            memory,
            context,
            driver,
            skill,
            mcp,
            application,
            store,
            entitlement,
            payment,
            web3,
            evm,
            scheduler,
            heartbeat,
        }
    }

    /// Borrow the focused Web3 Service client.
    pub fn web3_client(&self) -> &W3 {
        &self.web3
    }

    /// Borrow the focused EVM Service client.
    pub fn evm_client(&self) -> &EVM {
        &self.evm
    }

    /// Borrow the focused Scheduler Service client.
    pub fn scheduler_client(&self) -> &SCH {
        &self.scheduler
    }

    /// Borrow the focused Heartbeat Service client.
    pub fn heartbeat_client(&self) -> &HB {
        &self.heartbeat
    }

    /// Borrow the focused Payment Service client.
    pub fn payment_client(&self) -> &PMT {
        &self.payment
    }

    /// Borrow the focused Store Service client.
    pub fn store_client(&self) -> &ST {
        &self.store
    }

    /// Borrow the focused Entitlement Service client.
    pub fn entitlement_client(&self) -> &E {
        &self.entitlement
    }

    /// Borrow the focused Application Service client.
    pub fn application_client(&self) -> &A {
        &self.application
    }

    /// Borrow the focused Driver Service client.
    pub fn driver_client(&self) -> &D {
        &self.driver
    }

    /// Borrow the focused Skill Service client.
    pub fn skill_client(&self) -> &SK {
        &self.skill
    }

    /// Borrow the focused MCP Service client.
    pub fn mcp_client(&self) -> &MCP {
        &self.mcp
    }

    /// Query a session-scoped task board through the facade boundary.
    pub async fn query_task_board(
        &self,
        command: TaskBoardQueryCommand,
    ) -> MacacaResult<TaskBoardQueryResult> {
        info!(
            app_id = %command.app_id.0,
            session_id = %command.session_id,
            "system facade task board query started"
        );
        let result = self.task_board.query_task_board(&command).await?;
        info!(
            app_id = %command.app_id.0,
            session_id = %command.session_id,
            count = result.count,
            "system facade task board query completed"
        );
        Ok(result)
    }

    /// Return a shell-facing status snapshot without exposing presentation internals.
    pub async fn status_snapshot(&self) -> MacacaResult<SystemStatusSnapshot> {
        info!("system facade status snapshot requested");
        self.status.status_snapshot().await
    }

    /// Inspect service availability through the SDK service client boundary.
    pub async fn inspect_services(
        &self,
        command: ServiceInspectionCommand,
    ) -> MacacaResult<ServiceInspectionResult> {
        info!(
            scope = %command.scope,
            "system facade service inspection started"
        );
        self.service.inspect_services(&command).await
    }

    /// Dispatch a service command through the replaceable SDK service client.
    pub async fn call_service(
        &self,
        command: ServiceCallCommand,
    ) -> MacacaResult<ServiceCallResult> {
        info!(
            service_id = %command.service_id,
            command = %command.command_name,
            "system facade service call started"
        );
        self.service.call_service(&command).await
    }

    /// Replay session events through the replaceable trace client.
    pub async fn replay_events(
        &self,
        command: SessionEventQueryCommand,
    ) -> MacacaResult<TraceQueryResult> {
        info!(
            session_id = %command.session_id,
            since = ?command.since,
            limit = ?command.limit,
            "system facade trace replay started"
        );
        self.trace.replay_events(&command).await
    }

    /// Tail session trace events through the replaceable trace client.
    pub async fn tail_trace(&self, command: TraceTailCommand) -> MacacaResult<TraceQueryResult> {
        info!(
            session_id = %command.session_id,
            since = ?command.since,
            "system facade trace tail started"
        );
        self.trace.tail_trace(&command).await
    }

    /// Inspect packages through the replaceable package client.
    pub async fn inspect_packages(
        &self,
        command: PackageInspectionCommand,
    ) -> MacacaResult<PackageInspectionResult> {
        info!(
            package_ref = ?command.package_ref,
            "system facade package inspection started"
        );
        self.package.inspect_packages(&command).await
    }

    /// Dispatch a typed LLM chat command through the focused LLM client.
    pub async fn llm_chat(
        &self,
        command: macaca_llm::LlmChatCommand,
    ) -> MacacaResult<macaca_llm::LlmChatResult> {
        info!(
            trace_id = %command.trace.trace_id,
            "system facade llm chat started"
        );
        self.llm.chat(command).await
    }

    /// Recall scoped memory through the focused Memory client.
    pub async fn memory_recall(
        &self,
        command: macaca_memory::MemoryRecallCommand,
    ) -> MacacaResult<macaca_memory::MemoryRecallResult> {
        info!(
            trace_id = %command.trace.trace_id,
            "system facade memory recall started"
        );
        self.memory.recall(command).await
    }

    /// Assemble model context through the focused Context client.
    pub async fn assemble_context(
        &self,
        command: macaca_context::ContextAssembleCommand,
    ) -> MacacaResult<macaca_context::ContextAssembleServiceResult> {
        info!(
            trace_id = %command.trace.trace_id,
            "system facade context assembly started"
        );
        self.context.assemble(command).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use chrono::Utc;
    use macaca_proto::{ApplicationId, TodoItem, TodoStatus};

    struct MockTaskBoardClient {
        todos: Vec<TodoItem>,
    }

    #[async_trait]
    impl SystemTaskClient for MockTaskBoardClient {
        async fn query_task_board(
            &self,
            _command: &TaskBoardQueryCommand,
        ) -> MacacaResult<TaskBoardQueryResult> {
            let mut todos = self.todos.clone();
            todos.sort_by_key(|todo| todo.sequence_number);
            let count = todos.len();
            Ok(TaskBoardQueryResult { todos, count })
        }
    }

    fn todo(sequence_number: u32) -> TodoItem {
        let mut item = TodoItem::new(
            ApplicationId(uuid::Uuid::new_v4()),
            Some("session-a".into()),
            "agent",
            "planner",
            "title",
            "description",
            1,
        );
        item.session_id = Some("session-a".into());
        item.status = TodoStatus::Pending;
        item.sequence_number = sequence_number;
        item.created_at = Utc::now();
        item.updated_at = Utc::now();
        item
    }

    #[tokio::test]
    async fn system_facade_returns_sorted_task_board_without_web_dependency() {
        let facade = SystemFacade::new(
            MockTaskBoardClient {
                todos: vec![todo(2), todo(1)],
            },
            StaticSystemStatusDataSource::new(SystemStatusSnapshot {
                version: "test".into(),
                agent_count: 0,
                loaded_apps: 0,
                max_agents: 8,
                llm_provider: "stub".into(),
                app_runtime: "macaca-app/AppRuntime".into(),
                gateway_enabled: false,
            }),
        );
        let result = facade
            .query_task_board(
                TaskBoardQueryCommand::new(ApplicationId(uuid::Uuid::new_v4()), "session-a")
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(result.count, 2);
        assert_eq!(result.todos[0].sequence_number, 1);
    }

    #[tokio::test]
    async fn default_service_client_returns_structured_unavailable() {
        let facade = SystemFacade::new(
            MockTaskBoardClient { todos: Vec::new() },
            StaticSystemStatusDataSource::new(SystemStatusSnapshot {
                version: "test".into(),
                agent_count: 0,
                loaded_apps: 0,
                max_agents: 8,
                llm_provider: "stub".into(),
                app_runtime: "macaca-app/AppRuntime".into(),
                gateway_enabled: false,
            }),
        );
        let command =
            ServiceCallCommand::new("service-a", "command-a", serde_json::json!({})).unwrap();
        let error = facade.call_service(command).await.unwrap_err();
        assert!(error.to_string().contains("unavailable"));
    }

    #[test]
    fn task_board_command_rejects_blank_session_scope() {
        let error = TaskBoardQueryCommand::new(ApplicationId(uuid::Uuid::new_v4()), "  ");
        assert!(error.is_err());
    }
}
