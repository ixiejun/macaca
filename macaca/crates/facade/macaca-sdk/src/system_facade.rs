//! Shell-facing system facade (**Facade** module root) for Web and CLI thin-shell migration.
//!
//! `SystemFacade` is the SDK-owned Facade that upper presentation shells call
//! instead of reaching into provider crates or Web internals. The facade is
//! deliberately composed from small Strategy clients so each capability can be
//! connected to `ServiceRuntime` independently as capability clients mature.
//!
//! # Module tree (P5 iteration 96)
//! - `types` — generic aggregate struct over Strategy client type parameters
//! - `constructors` — Builder factories with Null Object defaults
//! - `accessors` — focused client borrows for capability-specific shells
//! - `operations` — audited command forwarding with `tracing` audit nodes
//! - `tests` — contract tests for facade composition boundaries

mod accessors;
mod constructors;
mod operations;
mod types;

#[cfg(test)]
mod tests;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub use crate::application_client::{SystemApplicationClient, UnavailableSystemApplicationClient};
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
pub use crate::status_client::{
    StaticSystemStatusClient, SystemStatusClient, SystemStatusSnapshot,
};
pub use crate::store_client::{SystemStoreClient, UnavailableSystemStoreClient};
pub use crate::task_client::{
    ServiceBackedTaskBoardDataSource, SystemTaskClient, TaskBoardDataSource, TaskBoardQueryCommand,
    TaskBoardQueryResult,
};
pub use crate::tool_client::{
    ServiceBackedToolClient, SystemToolClient, UnavailableSystemToolClient,
};
pub use crate::trace_client::{
    EmptySystemTraceClient, SessionEventQueryCommand, SystemTraceClient, TraceQueryResult,
    TraceTailCommand,
};
pub use crate::web3_client::{SystemWeb3Client, UnavailableSystemWeb3Client};

pub use types::SystemFacade;

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
