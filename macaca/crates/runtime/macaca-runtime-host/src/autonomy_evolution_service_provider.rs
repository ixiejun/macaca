//! Runtime-host adapter for the Autonomy Evolution Control Plane service.
//!
//! This module is an Adapter/Bridge between the kernel-visible `SystemService`
//! command envelope and the typed `macaca-autonomy-evolution` service contract.
//! It deliberately performs no candidate classification, benchmark scoring,
//! rollout decision, rollback interpretation, or target mutation. Those
//! semantics remain in the service provider and future target adapter
//! Strategies.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use macaca_autonomy_evolution::{
    AutonomyEvolutionService, EvolutionAdmissionCommand, EvolutionBenchmarkCommand,
    EvolutionLiveAuditCommand, EvolutionLiveTickCommand, EvolutionReleaseCommand,
    EvolutionSnapshotCommand, EvolutionTransitionCommand, InMemoryAutonomyEvolutionProvider,
    OsCodeEvolutionProposalCommand, UnavailableAutonomyEvolutionProvider,
    AUTONOMY_EVOLUTION_ADMISSION_COMMAND, AUTONOMY_EVOLUTION_BENCHMARK_COMMAND,
    AUTONOMY_EVOLUTION_HEALTH_COMMAND, AUTONOMY_EVOLUTION_LIVE_AUDIT_COMMAND,
    AUTONOMY_EVOLUTION_LIVE_TICK_COMMAND, AUTONOMY_EVOLUTION_OS_CODE_PROPOSAL_COMMAND,
    AUTONOMY_EVOLUTION_RELEASE_COMMAND, AUTONOMY_EVOLUTION_SERVICE_ID,
    AUTONOMY_EVOLUTION_SNAPSHOT_COMMAND, AUTONOMY_EVOLUTION_TRANSITION_COMMAND,
};
use macaca_kernel::SystemService;
use macaca_proto::{
    CleanupPolicy, MacacaError, ServiceCallResult, ServiceCommand, ServiceDescriptor, ServiceError,
    ServiceHealth, ServiceResult, TraceContext,
};
use tracing::info;

/// Runtime-host wrapper for a replaceable autonomy evolution provider.
pub struct AutonomyEvolutionSystemServiceProvider {
    provider: Arc<dyn AutonomyEvolutionService>,
}

impl AutonomyEvolutionSystemServiceProvider {
    /// Wrap a concrete, remote, plugin, mock, or unavailable provider.
    pub fn new(provider: Arc<dyn AutonomyEvolutionService>) -> Self {
        Self { provider }
    }

    /// Build the first-slice in-memory provider for local runtime-host tests.
    pub fn in_memory() -> Self {
        Self::new(Arc::new(InMemoryAutonomyEvolutionProvider::default()))
    }

    /// Build the fail-closed Null Object provider used when absent.
    pub fn unavailable() -> Self {
        Self::new(Arc::new(UnavailableAutonomyEvolutionProvider::default()))
    }
}

#[async_trait]
impl SystemService for AutonomyEvolutionSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.provider.descriptor()
    }

    async fn start(&self) -> ServiceResult<()> {
        let descriptor = self.provider.descriptor();
        info!(
            service_id = %descriptor.id,
            "autonomy evolution system service provider started"
        );
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = command_trace(&command)?;
        info!(
            service_id = AUTONOMY_EVOLUTION_SERVICE_ID,
            command = %command.name,
            trace_id = %trace.trace_id,
            "autonomy evolution system service command accepted"
        );
        match command.name.as_str() {
            AUTONOMY_EVOLUTION_TRANSITION_COMMAND => {
                let typed: EvolutionTransitionCommand = decode(command.payload)?;
                service_result(
                    self.provider
                        .transition(typed)
                        .await
                        .map_err(service_adapter_error)?,
                    trace,
                )
            }
            AUTONOMY_EVOLUTION_ADMISSION_COMMAND => {
                let typed: EvolutionAdmissionCommand = decode(command.payload)?;
                service_result(
                    self.provider
                        .admit_candidate(typed)
                        .await
                        .map_err(service_adapter_error)?,
                    trace,
                )
            }
            AUTONOMY_EVOLUTION_BENCHMARK_COMMAND => {
                let typed: EvolutionBenchmarkCommand = decode(command.payload)?;
                service_result(
                    self.provider
                        .run_paired_benchmark(typed)
                        .await
                        .map_err(service_adapter_error)?,
                    trace,
                )
            }
            AUTONOMY_EVOLUTION_RELEASE_COMMAND => {
                let typed: EvolutionReleaseCommand = decode(command.payload)?;
                service_result(
                    self.provider
                        .evaluate_release(typed)
                        .await
                        .map_err(service_adapter_error)?,
                    trace,
                )
            }
            AUTONOMY_EVOLUTION_OS_CODE_PROPOSAL_COMMAND => {
                let typed: OsCodeEvolutionProposalCommand = decode(command.payload)?;
                service_result(
                    self.provider
                        .evaluate_os_code_proposal(typed)
                        .await
                        .map_err(service_adapter_error)?,
                    trace,
                )
            }
            AUTONOMY_EVOLUTION_LIVE_TICK_COMMAND => {
                let typed: EvolutionLiveTickCommand = decode(command.payload)?;
                service_result(
                    self.provider
                        .run_live_tick(typed)
                        .await
                        .map_err(service_adapter_error)?,
                    trace,
                )
            }
            AUTONOMY_EVOLUTION_LIVE_AUDIT_COMMAND => {
                let typed: EvolutionLiveAuditCommand = decode(command.payload)?;
                service_result(
                    self.provider
                        .audit_live_run(typed)
                        .await
                        .map_err(service_adapter_error)?,
                    trace,
                )
            }
            AUTONOMY_EVOLUTION_SNAPSHOT_COMMAND => {
                let typed: EvolutionSnapshotCommand = decode(command.payload)?;
                service_result(
                    self.provider
                        .snapshot(typed)
                        .await
                        .map_err(service_adapter_error)?,
                    trace,
                )
            }
            AUTONOMY_EVOLUTION_HEALTH_COMMAND => service_result(
                self.provider
                    .health(trace.clone())
                    .await
                    .map_err(service_adapter_error)?,
                trace,
            ),
            other => Err(ServiceError::UnsupportedCommand(format!(
                "unsupported Autonomy Evolution service command '{other}'"
            ))),
        }
    }

    async fn stop(&self) -> ServiceResult<()> {
        info!(
            service_id = AUTONOMY_EVOLUTION_SERVICE_ID,
            "autonomy evolution system service provider stopped"
        );
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        info!(
            service_id = AUTONOMY_EVOLUTION_SERVICE_ID,
            "autonomy evolution system service provider cleanup completed"
        );
        Ok(())
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        Ok(self.provider.descriptor().health)
    }
}

fn command_trace(command: &ServiceCommand) -> ServiceResult<TraceContext> {
    command
        .trace
        .clone()
        .ok_or(ServiceError::MissingTraceContext)
}

fn decode<T: serde::de::DeserializeOwned>(value: serde_json::Value) -> ServiceResult<T> {
    serde_json::from_value(value)
        .map_err(|error| ServiceError::UnsupportedCommand(error.to_string()))
}

fn service_result<T: serde::Serialize>(
    output: T,
    trace: TraceContext,
) -> ServiceResult<ServiceCallResult> {
    Ok(ServiceCallResult {
        output: serde_json::to_value(output)
            .map_err(|error| ServiceError::AdapterFailure(error.to_string()))?,
        trace,
        status: "ok".into(),
        metadata: BTreeMap::new(),
        cleanup_hint: Some(CleanupPolicy::None),
    })
}

fn service_adapter_error(error: MacacaError) -> ServiceError {
    ServiceError::AdapterFailure(error.to_string())
}
