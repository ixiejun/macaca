//! Factory constructors for [`super::types::SystemFacade`].
//!
//! Constructors install Null Object defaults for capabilities that are not yet
//! wired by the composition root.  Each factory method is an explicit Builder
//! entry point so dependency injection remains visible and auditable.

use crate::application_client::UnavailableSystemApplicationClient;
use crate::context_client::UnavailableSystemContextClient;
use crate::driver_client::UnavailableSystemDriverClient;
use crate::entitlement_client::UnavailableSystemEntitlementClient;
use crate::evm_client::UnavailableSystemEvmClient;
use crate::heartbeat_client::UnavailableSystemHeartbeatClient;
use crate::llm_client::UnavailableSystemLlmClient;
use crate::mcp_client::UnavailableSystemMcpClient;
use crate::memory_client::UnavailableSystemMemoryClient;
use crate::package_client::{EmptySystemPackageClient, SystemPackageClient};
use crate::scheduler_client::UnavailableSystemSchedulerClient;
use crate::service_client::{SystemServiceClient, UnavailableSystemServiceClient};
use crate::skill_client::UnavailableSystemSkillClient;
use crate::status_client::SystemStatusClient;
use crate::store_client::UnavailableSystemStoreClient;
use crate::task_client::SystemTaskClient;
use crate::trace_client::{EmptySystemTraceClient, SystemTraceClient};
use crate::application_client::SystemApplicationClient;
use crate::context_client::SystemContextClient;
use crate::driver_client::SystemDriverClient;
use crate::entitlement_client::SystemEntitlementClient;
use crate::evm_client::SystemEvmClient;
use crate::heartbeat_client::SystemHeartbeatClient;
use crate::llm_client::SystemLlmClient;
use crate::mcp_client::SystemMcpClient;
use crate::memory_client::SystemMemoryClient;
use crate::payment_client::{SystemPaymentClient, UnavailableSystemPaymentClient};
use crate::scheduler_client::SystemSchedulerClient;
use crate::skill_client::SystemSkillClient;
use crate::store_client::SystemStoreClient;
use crate::web3_client::{SystemWeb3Client, UnavailableSystemWeb3Client};

impl<T, S> super::types::SystemFacade<T, S>
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
    super::types::SystemFacade<
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
    super::types::SystemFacade<T, S, SV, TR, P, L, M, C, D, SK, MCP, A, ST, E, PMT, W3, EVM, SCH, HB>
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
    ) -> super::types::SystemFacade<
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
        super::types::SystemFacade::<
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
}
