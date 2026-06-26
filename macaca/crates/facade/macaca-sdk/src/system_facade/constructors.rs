//! Factory constructors for [`super::types::SystemFacade`].
//!
//! Constructors install Null Object defaults for capabilities that are not yet
//! wired by the composition root.  Each factory method is an explicit Builder
//! entry point so dependency injection remains visible and auditable.

use crate::application_client::UnavailableSystemApplicationClient;
use crate::domain_pack_client::{EmptySystemDomainPackClient, SystemDomainPackClient};
use crate::driver_client::UnavailableSystemDriverClient;
use crate::entitlement_client::UnavailableSystemEntitlementClient;
use crate::evm_client::UnavailableSystemEvmClient;
use crate::heartbeat_client::UnavailableSystemHeartbeatClient;
use crate::llm_client::UnavailableSystemLlmClient;
use crate::mcp_client::UnavailableSystemMcpClient;
use crate::package_client::{EmptySystemPackageClient, SystemPackageClient};
use crate::payment_client::UnavailableSystemPaymentClient;
use crate::scheduler_client::UnavailableSystemSchedulerClient;
use crate::service_client::{SystemServiceClient, UnavailableSystemServiceClient};
use crate::status_client::SystemStatusClient;
use crate::store_client::UnavailableSystemStoreClient;
use crate::task_client::SystemTaskClient;
use crate::trace_client::{EmptySystemTraceClient, SystemTraceClient};
use crate::web3_client::UnavailableSystemWeb3Client;

use crate::application_client::SystemApplicationClient;
use crate::driver_client::SystemDriverClient;
use crate::entitlement_client::SystemEntitlementClient;
use crate::evm_client::SystemEvmClient;
use crate::heartbeat_client::SystemHeartbeatClient;
use crate::llm_client::SystemLlmClient;
use crate::mcp_client::SystemMcpClient;
use crate::memory_client::{SystemMemoryClient, UnavailableSystemMemoryClient};
use crate::payment_client::SystemPaymentClient;
use crate::scheduler_client::SystemSchedulerClient;
use crate::store_client::SystemStoreClient;
use crate::web3_client::SystemWeb3Client;

use super::types::{UnavailableSystemContextClient, UnavailableSystemSkillClient};

impl<T, S> super::types::SystemFacade<T, S>
where
    T: SystemTaskClient,
    S: SystemStatusClient,
{
    /// Create a facade from task and status clients.
    ///
    /// This constructor preserves the stable Web/CLI facade shape. It installs
    /// explicit empty/unavailable clients for capabilities whose service-backed
    /// implementations are provided by the runtime composition root.
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
            domain_pack: EmptySystemDomainPackClient,
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
    /// Create a facade from explicit base capability clients.
    ///
    /// This constructor installs explicit Null Object clients for capabilities
    /// whose runtime-backed composition path has not been injected.
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
            domain_pack: EmptySystemDomainPackClient,
        }
    }
}

impl<T, S, SV, TR, P, L, M, C, D, SK, MCP, A, ST, E, PMT, W3, EVM, SCH, HB>
    super::types::SystemFacade<
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
        SCH,
        HB,
    >
where
    T: SystemTaskClient,
    S: SystemStatusClient,
    SV: SystemServiceClient,
    TR: SystemTraceClient,
    P: SystemPackageClient,
    L: SystemLlmClient,
    M: SystemMemoryClient,
    C: Send + Sync,
    D: SystemDriverClient,
    SK: Send + Sync,
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
    /// Create a facade with all current service capability clients installed.
    ///
    /// This constructor is the explicit composition point for serviceized
    /// runtimes.  It keeps SDK dependency injection visible and prevents the
    /// facade from constructing provider/backends internally.
    pub fn with_service_clients(
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
            domain_pack: EmptySystemDomainPackClient,
        }
    }

    /// Create a facade with service capability clients plus autonomy clients.
    ///
    /// This constructor keeps Scheduler and Heartbeat composition explicit.
    /// The facade receives clients as Strategy objects and never constructs
    /// providers, timers, stores, queues, or application-specific workflows.
    pub fn with_service_and_autonomy_clients(
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
            domain_pack: EmptySystemDomainPackClient,
        }
    }
}

impl<T, S, SV, TR, P, L, M, C, D, SK, MCP, A, ST, E, PMT, W3, EVM, SCH, HB, DP>
    super::types::SystemFacade<
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
        SCH,
        HB,
        DP,
    >
{
    /// Replace the current domain-pack discovery client.
    ///
    /// This Builder-style method preserves existing facade construction sites while allowing a
    /// host composition root to inject a catalog-backed discovery Strategy.  The facade only
    /// stores the client; it never constructs package providers or reads runtime-host internals.
    pub fn with_domain_pack_client<DP2>(
        self,
        domain_pack: DP2,
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
        SCH,
        HB,
        DP2,
    >
    where
        DP2: SystemDomainPackClient,
    {
        super::types::SystemFacade {
            task_board: self.task_board,
            status: self.status,
            service: self.service,
            trace: self.trace,
            package: self.package,
            llm: self.llm,
            memory: self.memory,
            context: self.context,
            driver: self.driver,
            skill: self.skill,
            mcp: self.mcp,
            application: self.application,
            store: self.store,
            entitlement: self.entitlement,
            payment: self.payment,
            web3: self.web3,
            evm: self.evm,
            scheduler: self.scheduler,
            heartbeat: self.heartbeat,
            domain_pack,
        }
    }
}
