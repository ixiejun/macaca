//! Runtime-host adapter for the Heartbeat autonomy service.
//!
//! **Pattern:** Adapter/Bridge — decodes generic service-runtime commands into
//! typed Heartbeat DTOs.  Wake coalescing and gate evaluation remain provider
//! behavior; the adapter preserves trace and structured reply envelopes only.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_heartbeat::{HeartbeatService, UnavailableHeartbeatProvider};
use macaca_kernel::SystemService;
use macaca_proto::{
    HeartbeatCancelWakeCommand, HeartbeatCompleteRunCommand, HeartbeatQueryCommand,
    HeartbeatUpdateProfileCommand, HeartbeatWakeCommand, ServiceCallResult, ServiceCommand,
    ServiceDescriptor, ServiceError, ServiceHealth, ServiceResult, HEARTBEAT_CANCEL_WAKE_COMMAND,
    HEARTBEAT_COMPLETE_RUN_COMMAND, HEARTBEAT_GET_RUN_COMMAND, HEARTBEAT_HEALTH_COMMAND,
    HEARTBEAT_LIST_RUNS_COMMAND, HEARTBEAT_SERVICE_ID, HEARTBEAT_SNAPSHOT_COMMAND,
    HEARTBEAT_UPDATE_PROFILE_COMMAND, HEARTBEAT_WAKE_COMMAND,
};
use tracing::info;

use super::support::{command_trace, decode, service_adapter_error, service_result};

/// Runtime-host adapter for a Heartbeat service provider.
///
/// This adapter decodes generic service-runtime commands into typed Heartbeat
/// commands.  Wake coalescing and gate evaluation remain provider behavior; the
/// adapter only preserves trace, structured errors, and sanitized result
/// envelopes.
pub struct HostHeartbeatServiceAdapter {
    provider: Arc<dyn HeartbeatService>,
}

impl HostHeartbeatServiceAdapter {
    /// Wrap a concrete or remote Heartbeat provider.
    pub fn new(provider: Arc<dyn HeartbeatService>) -> Self {
        Self { provider }
    }

    /// Build the fail-closed Null Object provider used by default.
    pub fn unavailable() -> Self {
        Self::new(Arc::new(UnavailableHeartbeatProvider::default()))
    }
}

#[async_trait]
impl SystemService for HostHeartbeatServiceAdapter {
    fn descriptor(&self) -> ServiceDescriptor {
        self.provider.descriptor()
    }

    async fn start(&self) -> ServiceResult<()> {
        let descriptor = self.provider.descriptor();
        info!(
            service_id = %descriptor.id,
            "heartbeat system service provider started"
        );
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = command_trace(&command)?;
        info!(
            service_id = HEARTBEAT_SERVICE_ID,
            command = %command.name,
            trace_id = %trace.trace_id,
            "heartbeat system service command accepted"
        );
        match command.name.as_str() {
            HEARTBEAT_WAKE_COMMAND => {
                let typed: HeartbeatWakeCommand = decode(command.payload)?;
                service_result(
                    self.provider
                        .wake(typed)
                        .await
                        .map_err(service_adapter_error)?,
                    trace,
                )
            }
            HEARTBEAT_CANCEL_WAKE_COMMAND => {
                let typed: HeartbeatCancelWakeCommand = decode(command.payload)?;
                service_result(
                    self.provider
                        .cancel_wake(typed)
                        .await
                        .map_err(service_adapter_error)?,
                    trace,
                )
            }
            HEARTBEAT_COMPLETE_RUN_COMMAND => {
                let typed: HeartbeatCompleteRunCommand = decode(command.payload)?;
                service_result(
                    self.provider
                        .complete_run(typed)
                        .await
                        .map_err(service_adapter_error)?,
                    trace,
                )
            }
            HEARTBEAT_GET_RUN_COMMAND => {
                let typed: HeartbeatQueryCommand = decode(command.payload)?;
                service_result(
                    self.provider
                        .get_run(typed)
                        .await
                        .map_err(service_adapter_error)?,
                    trace,
                )
            }
            HEARTBEAT_LIST_RUNS_COMMAND => {
                let typed: HeartbeatQueryCommand = decode(command.payload)?;
                service_result(
                    self.provider
                        .list_runs(typed)
                        .await
                        .map_err(service_adapter_error)?,
                    trace,
                )
            }
            HEARTBEAT_UPDATE_PROFILE_COMMAND => {
                let typed: HeartbeatUpdateProfileCommand = decode(command.payload)?;
                service_result(
                    self.provider
                        .update_profile(typed)
                        .await
                        .map_err(service_adapter_error)?,
                    trace,
                )
            }
            HEARTBEAT_HEALTH_COMMAND => service_result(
                self.provider
                    .health(trace.clone())
                    .await
                    .map_err(service_adapter_error)?,
                trace,
            ),
            HEARTBEAT_SNAPSHOT_COMMAND => {
                let typed: HeartbeatQueryCommand = decode(command.payload)?;
                service_result(
                    self.provider
                        .snapshot(typed)
                        .await
                        .map_err(service_adapter_error)?,
                    trace,
                )
            }
            other => Err(ServiceError::UnsupportedCommand(format!(
                "unsupported Heartbeat service command '{other}'"
            ))),
        }
    }

    async fn stop(&self) -> ServiceResult<()> {
        info!(
            service_id = HEARTBEAT_SERVICE_ID,
            "heartbeat system service provider stopped"
        );
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        info!(
            service_id = HEARTBEAT_SERVICE_ID,
            "heartbeat system service provider cleanup completed"
        );
        Ok(())
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        Ok(self.provider.descriptor().health)
    }
}
