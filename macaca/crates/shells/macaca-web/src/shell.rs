//! Web Shell command adapter boundary for Route C Phase 12.
//!
//! The Web layer owns HTTP parsing, response mapping, and presentation logging.
//! It must not own task/session/trace/service/package semantics. This adapter
//! converts validated HTTP scope into SDK system-facade commands, then maps the
//! facade result back to the existing JSON shape expected by the frontend.

use std::sync::Arc;

use serde_json::json;
use tracing::info;

use macaca_proto::{
    ApplicationId, EvmAvailability, EvmAvailabilityCommand, MacacaError, MacacaResult,
    TraceContext, Web3Availability, Web3AvailabilityCommand,
};
use macaca_sdk::{
    StaticSystemStatusDataSource, SystemApplicationExecutionClient, SystemEvmClient, SystemFacade,
    SystemPluginCapabilityClient, SystemPluginControlClient, SystemPluginHookClient,
    SystemServiceClient, SystemStatusSnapshot, SystemWeb3Client, TaskBoardQueryCommand,
    TodoStoreTaskBoardDataSource,
};
use macaca_sdk::task::TodoStore;

/// Route-safe facade bundle for Web system surfaces.
///
/// This Adapter gives routes a single primary seam for serviceized system
/// capabilities without forcing `AppState` to expose provider construction or
/// concrete runtime-host types.  The bundle deliberately stores focused SDK
/// clients, not lower-layer providers, so Web remains responsible only for HTTP
/// validation, response projection, and audit logging.
#[derive(Clone)]
pub struct WebSystemFacadeBundle {
    service: Arc<dyn SystemServiceClient>,
    web3: Arc<dyn SystemWeb3Client>,
    evm: Arc<dyn SystemEvmClient>,
    plugin_control: Arc<dyn SystemPluginControlClient>,
    plugin_capability: Arc<dyn SystemPluginCapabilityClient>,
    plugin_hook: Arc<dyn SystemPluginHookClient>,
    application_execution: Arc<dyn SystemApplicationExecutionClient>,
}

impl WebSystemFacadeBundle {
    /// Build a route-safe bundle from runtime-backed SDK clients.
    ///
    /// The constructor is the Web-local Facade composition point required by
    /// S12.  It does not register providers or start services; those lifecycle
    /// responsibilities belong to `macaca-runtime-host` bootstrap code.
    pub fn new(
        service: Arc<dyn SystemServiceClient>,
        web3: Arc<dyn SystemWeb3Client>,
        evm: Arc<dyn SystemEvmClient>,
        plugin_control: Arc<dyn SystemPluginControlClient>,
        plugin_capability: Arc<dyn SystemPluginCapabilityClient>,
        plugin_hook: Arc<dyn SystemPluginHookClient>,
        application_execution: Arc<dyn SystemApplicationExecutionClient>,
    ) -> Self {
        Self {
            service,
            web3,
            evm,
            plugin_control,
            plugin_capability,
            plugin_hook,
            application_execution,
        }
    }

    /// Borrow the generic service-inspection client for low-risk status routes.
    pub fn service_client(&self) -> Arc<dyn SystemServiceClient> {
        Arc::clone(&self.service)
    }

    /// Borrow the focused Plugin Control client for Web plugin routes.
    pub fn plugin_control_client(&self) -> Arc<dyn SystemPluginControlClient> {
        Arc::clone(&self.plugin_control)
    }

    /// Borrow the focused Plugin Capability client for capability routes.
    pub fn plugin_capability_client(&self) -> Arc<dyn SystemPluginCapabilityClient> {
        Arc::clone(&self.plugin_capability)
    }

    /// Borrow the focused Plugin Hook client for future hook routes/adapters.
    pub fn plugin_hook_client(&self) -> Arc<dyn SystemPluginHookClient> {
        Arc::clone(&self.plugin_hook)
    }

    /// Borrow the focused Application Execution client for route adapters.
    ///
    /// This keeps Web in the Facade role: HTTP handlers receive and validate
    /// transport scope, then call the SDK. Provider strategy registration,
    /// EventLog ownership, lease validation, and control delivery stay in the
    /// runtime-host service implementation.
    pub fn application_execution_client(&self) -> Arc<dyn SystemApplicationExecutionClient> {
        Arc::clone(&self.application_execution)
    }

    /// Query optional Web3 availability through the focused SDK client.
    ///
    /// Optional-module unavailability is returned as data by the service layer.
    /// If the service call itself fails, Web still fails closed with a regular
    /// `MacacaError` instead of inventing provider semantics in the route.
    pub async fn web3_availability(
        &self,
        trace_id: impl Into<String>,
    ) -> MacacaResult<Web3Availability> {
        let trace = TraceContext::new(trace_id);
        info!(
            trace_id = %trace.trace_id,
            "web system facade querying web3 availability"
        );
        self.web3
            .availability(Web3AvailabilityCommand { trace })
            .await
    }

    /// Query optional EVM availability through the focused SDK client.
    ///
    /// This mirrors the Web3 path and keeps EVM provider, RPC, wallet, ABI, and
    /// transaction semantics outside the Web presentation shell.
    pub async fn evm_availability(
        &self,
        trace_id: impl Into<String>,
    ) -> MacacaResult<EvmAvailability> {
        let trace = TraceContext::new(trace_id);
        info!(
            trace_id = %trace.trace_id,
            "web system facade querying evm availability"
        );
        self.evm
            .availability(EvmAvailabilityCommand { trace })
            .await
    }
}

/// Thin Web Shell facade specialized for current Web runtime dependencies.
pub struct WebShellFacade {
    system: SystemFacade<TodoStoreTaskBoardDataSource, StaticSystemStatusDataSource>,
}

impl WebShellFacade {
    /// Build a shell facade from current Web state dependencies.
    ///
    /// The status adapter is intentionally inert for the task-board route. It
    /// keeps the facade type complete without making Web own status semantics.
    pub fn for_task_board(todo_store: Arc<TodoStore>) -> Self {
        Self {
            system: SystemFacade::new(
                TodoStoreTaskBoardDataSource::new(todo_store),
                StaticSystemStatusDataSource::new(SystemStatusSnapshot {
                    version: env!("CARGO_PKG_VERSION").into(),
                    agent_count: 0,
                    loaded_apps: 0,
                    max_agents: 0,
                    llm_provider: "shell-unavailable".into(),
                    app_runtime: "macaca-app/AppRuntime".into(),
                    gateway_enabled: false,
                }),
            ),
        }
    }

    /// Query the task board through the SDK facade and preserve legacy JSON shape.
    pub async fn list_todos_json(
        &self,
        app_id: ApplicationId,
        session_id: &str,
    ) -> MacacaResult<serde_json::Value> {
        info!(
            app_id = %app_id.0,
            session_id,
            "web shell task board command received"
        );
        let command = TaskBoardQueryCommand::new(app_id, session_id)
            .map_err(|error| MacacaError::Config(format!("invalid task board command: {error}")))?;
        let result = self.system.query_task_board(command).await?;
        info!(
            count = result.count,
            "web shell task board command completed"
        );
        Ok(json!({ "todos": result.todos, "count": result.count }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use macaca_sdk::runtime_host::persist::RedbStore;
    use macaca_proto::{
        EvmAvailability, EvmAvailabilityCommand, EvmCallAdmission, EvmContractCallCommand,
        EvmContractDeployCommand, EvmContractReadCommand, EvmDeployAdmission,
        EvmEventSubscribeCommand, EvmEventSubscriptionAdmission, EvmGasEstimateCommand,
        EvmReadAdmission, EvmReceiptGetCommand, EvmServiceSnapshot, EvmSnapshotCommand,
        EvmTransactionReceipt, GasEstimate, Web3Availability, Web3AvailabilityCommand,
        Web3ChainQueryCommand, Web3ServiceSnapshot, Web3SigningAdmission,
        Web3SigningRequestCommand, Web3SnapshotCommand, Web3TransactionPreparation,
        Web3TransactionPrepareCommand, Web3UnavailableReason, Web3WalletListCommand,
        Web3WalletView,
    };
    use macaca_sdk::{
        ServiceCallCommand, ServiceCallResult, ServiceInspectionCommand, ServiceInspectionResult,
        SystemEvmClient, SystemServiceClient, SystemWeb3Client,
    };

    #[derive(Debug, Default)]
    struct EmptyServiceClient;

    #[async_trait]
    impl SystemServiceClient for EmptyServiceClient {
        async fn inspect_services(
            &self,
            command: &ServiceInspectionCommand,
        ) -> MacacaResult<ServiceInspectionResult> {
            Ok(ServiceInspectionResult {
                scope: command.scope.clone(),
                services: Vec::new(),
            })
        }

        async fn call_service(
            &self,
            command: &ServiceCallCommand,
        ) -> MacacaResult<ServiceCallResult> {
            Err(MacacaError::Config(format!(
                "test service client has no dispatcher for {}",
                command.service_id
            )))
        }
    }

    #[derive(Debug, Default)]
    struct UnavailableWeb3Client;

    #[async_trait]
    impl SystemWeb3Client for UnavailableWeb3Client {
        async fn availability(
            &self,
            _command: Web3AvailabilityCommand,
        ) -> MacacaResult<Web3Availability> {
            Ok(Web3Availability::unavailable(Web3UnavailableReason::new(
                "missing",
                "not installed",
            )))
        }

        async fn wallets(
            &self,
            _command: Web3WalletListCommand,
        ) -> MacacaResult<Vec<Web3WalletView>> {
            Ok(Vec::new())
        }

        async fn request_signing(
            &self,
            _command: Web3SigningRequestCommand,
        ) -> MacacaResult<Web3SigningAdmission> {
            Err(MacacaError::Config("web3 unavailable".into()))
        }

        async fn prepare_transaction(
            &self,
            _command: Web3TransactionPrepareCommand,
        ) -> MacacaResult<Web3TransactionPreparation> {
            Err(MacacaError::Config("web3 unavailable".into()))
        }

        async fn query_chain(
            &self,
            _command: Web3ChainQueryCommand,
        ) -> MacacaResult<macaca_proto::ChainQueryResponse> {
            Err(MacacaError::Config("web3 unavailable".into()))
        }

        async fn snapshot(
            &self,
            _command: Web3SnapshotCommand,
        ) -> MacacaResult<Web3ServiceSnapshot> {
            Ok(Web3ServiceSnapshot::unavailable("not installed"))
        }
    }

    #[derive(Debug, Default)]
    struct UnavailableEvmClient;

    #[async_trait]
    impl SystemEvmClient for UnavailableEvmClient {
        async fn availability(
            &self,
            _command: EvmAvailabilityCommand,
        ) -> MacacaResult<EvmAvailability> {
            Ok(EvmAvailability::unavailable("missing", "not installed"))
        }

        async fn deploy(
            &self,
            _command: EvmContractDeployCommand,
        ) -> MacacaResult<EvmDeployAdmission> {
            Err(MacacaError::Config("evm unavailable".into()))
        }

        async fn call(&self, _command: EvmContractCallCommand) -> MacacaResult<EvmCallAdmission> {
            Err(MacacaError::Config("evm unavailable".into()))
        }

        async fn read(&self, _command: EvmContractReadCommand) -> MacacaResult<EvmReadAdmission> {
            Err(MacacaError::Config("evm unavailable".into()))
        }

        async fn estimate_gas(&self, _command: EvmGasEstimateCommand) -> MacacaResult<GasEstimate> {
            Err(MacacaError::Config("evm unavailable".into()))
        }

        async fn receipt(
            &self,
            _command: EvmReceiptGetCommand,
        ) -> MacacaResult<EvmTransactionReceipt> {
            Err(MacacaError::Config("evm unavailable".into()))
        }

        async fn subscribe(
            &self,
            _command: EvmEventSubscribeCommand,
        ) -> MacacaResult<EvmEventSubscriptionAdmission> {
            Err(MacacaError::Config("evm unavailable".into()))
        }

        async fn snapshot(&self, _command: EvmSnapshotCommand) -> MacacaResult<EvmServiceSnapshot> {
            Ok(EvmServiceSnapshot::unavailable("not installed"))
        }
    }

    #[tokio::test]
    async fn web_shell_task_board_preserves_legacy_json_shape() {
        let tempdir = tempfile::tempdir().unwrap();
        let store = Arc::new(RedbStore::open(tempdir.path().join("shell.redb")).unwrap());
        let todo_store = Arc::new(TodoStore::new(store));
        let shell = WebShellFacade::for_task_board(todo_store);
        let response = shell
            .list_todos_json(ApplicationId(uuid::Uuid::new_v4()), "session-a")
            .await
            .unwrap();
        assert!(response.get("todos").unwrap().is_array());
        assert_eq!(response.get("count").unwrap().as_u64(), Some(0));
    }

    #[tokio::test]
    async fn web_system_facade_preserves_optional_module_unavailable_as_data() {
        let facade = WebSystemFacadeBundle::new(
            Arc::new(EmptyServiceClient),
            Arc::new(UnavailableWeb3Client),
            Arc::new(UnavailableEvmClient),
            Arc::new(macaca_sdk::UnavailableSystemPluginControlClient),
            Arc::new(macaca_sdk::UnavailableSystemPluginCapabilityClient),
            Arc::new(macaca_sdk::UnavailableSystemPluginHookClient),
            Arc::new(macaca_sdk::UnavailableSystemApplicationExecutionClient::new()),
        );

        let web3 = facade.web3_availability("test-web3").await.unwrap();
        let evm = facade.evm_availability("test-evm").await.unwrap();

        assert_eq!(web3.state, "unavailable");
        assert_eq!(
            web3.reason.as_ref().map(|reason| reason.code.as_str()),
            Some("missing")
        );
        assert_eq!(evm.state, "unavailable");
        assert_eq!(evm.reason_code.as_deref(), Some("missing"));
    }
}
