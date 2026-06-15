//! Core `SystemFacade` aggregate type for shell-facing SDK composition.
//!
//! The struct is a generic Facade over Strategy clients.  Each type parameter
//! names one capability boundary so shells can compose runtime-backed or Null
//! Object implementations without importing provider crates.

use crate::application_client::UnavailableSystemApplicationClient;
use crate::driver_client::UnavailableSystemDriverClient;
use crate::entitlement_client::UnavailableSystemEntitlementClient;
use crate::evm_client::UnavailableSystemEvmClient;
use crate::heartbeat_client::UnavailableSystemHeartbeatClient;
use crate::llm_client::UnavailableSystemLlmClient;
use crate::mcp_client::UnavailableSystemMcpClient;
use crate::package_client::EmptySystemPackageClient;
use crate::payment_client::UnavailableSystemPaymentClient;
use crate::scheduler_client::UnavailableSystemSchedulerClient;
use crate::service_client::UnavailableSystemServiceClient;
use crate::store_client::UnavailableSystemStoreClient;
use crate::trace_client::EmptySystemTraceClient;
use crate::web3_client::UnavailableSystemWeb3Client;

use crate::memory_client::UnavailableSystemMemoryClient;

#[derive(Debug, Clone, Default)]
pub struct UnavailableSystemContextClient;

#[derive(Debug, Clone, Default)]
pub struct UnavailableSystemSkillClient;

/// SDK system facade consumed by Web/CLI thin shells.
///
/// The type parameters are intentionally capability-scoped Strategy clients.
/// Existing two-client call sites keep working through default service, trace,
/// and package clients that return structured empty/unavailable responses until
/// later service providers attach concrete runtime-backed implementations.
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
    pub(super) task_board: T,
    pub(super) status: S,
    pub(super) service: SV,
    pub(super) trace: TR,
    pub(super) package: P,
    pub(super) llm: L,
    pub(super) memory: M,
    pub(super) context: C,
    pub(super) driver: D,
    pub(super) skill: SK,
    pub(super) mcp: MCP,
    pub(super) application: A,
    pub(super) store: ST,
    pub(super) entitlement: E,
    pub(super) payment: PMT,
    pub(super) web3: W3,
    pub(super) evm: EVM,
    pub(super) scheduler: SCH,
    pub(super) heartbeat: HB,
}
