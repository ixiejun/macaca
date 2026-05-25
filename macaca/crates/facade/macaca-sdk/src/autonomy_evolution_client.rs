//! SDK client for the Autonomy Evolution Control Plane.
//!
//! This module is a thin Facade over `SystemServiceClient`. It deliberately
//! does not classify candidates, score benchmarks, apply rollouts, or mutate
//! target packages. Those semantics remain service-owned and provider-owned;
//! SDK callers only submit typed commands and receive body-free results.

use std::sync::Arc;

use async_trait::async_trait;
use macaca_autonomy_evolution::{
    EvolutionRunState, EvolutionServiceSnapshot, EvolutionSnapshotCommand,
    EvolutionTransitionCommand, EvolutionTransitionResult, AUTONOMY_EVOLUTION_HEALTH_COMMAND,
    AUTONOMY_EVOLUTION_SERVICE_ID, AUTONOMY_EVOLUTION_SNAPSHOT_COMMAND,
    AUTONOMY_EVOLUTION_TRANSITION_COMMAND,
};
use macaca_proto::{MacacaError, MacacaResult, TraceContext};
use tracing::{info, warn};

use crate::service_client::{ServiceCallCommand, SystemServiceClient};

/// Focused SDK boundary for autonomy evolution operations.
#[async_trait]
pub trait SystemAutonomyEvolutionClient: Send + Sync {
    async fn transition(
        &self,
        command: EvolutionTransitionCommand,
    ) -> MacacaResult<EvolutionTransitionResult>;

    async fn snapshot(
        &self,
        command: EvolutionSnapshotCommand,
    ) -> MacacaResult<EvolutionServiceSnapshot>;

    async fn health(&self, trace: TraceContext) -> MacacaResult<EvolutionServiceSnapshot>;
}

/// Null Object client used when no control-plane service is installed.
#[derive(Debug, Clone, Default)]
pub struct UnavailableSystemAutonomyEvolutionClient;

#[async_trait]
impl SystemAutonomyEvolutionClient for UnavailableSystemAutonomyEvolutionClient {
    async fn transition(
        &self,
        command: EvolutionTransitionCommand,
    ) -> MacacaResult<EvolutionTransitionResult> {
        warn!(
            trace_id = command.trace.trace_id.as_str(),
            run_id = command.run_id.as_str(),
            "sdk autonomy evolution client unavailable for transition"
        );
        Ok(EvolutionTransitionResult::denied(
            &command,
            command
                .from_state
                .clone()
                .unwrap_or(EvolutionRunState::Observed),
            "autonomy evolution control plane service is unavailable",
        ))
    }

    async fn snapshot(
        &self,
        command: EvolutionSnapshotCommand,
    ) -> MacacaResult<EvolutionServiceSnapshot> {
        info!(
            trace_id = command.trace.trace_id.as_str(),
            "sdk autonomy evolution client returning unavailable snapshot"
        );
        Ok(EvolutionServiceSnapshot::unavailable(
            "autonomy evolution control plane service is unavailable",
        ))
    }

    async fn health(&self, trace: TraceContext) -> MacacaResult<EvolutionServiceSnapshot> {
        info!(
            trace_id = trace.trace_id.as_str(),
            "sdk autonomy evolution client returning unavailable health"
        );
        Ok(EvolutionServiceSnapshot::unavailable(
            "autonomy evolution control plane service is unavailable",
        ))
    }
}

/// Runtime-backed client implemented over generic service calls.
#[derive(Clone)]
pub struct ServiceBackedAutonomyEvolutionClient {
    service: Arc<dyn SystemServiceClient>,
}

impl ServiceBackedAutonomyEvolutionClient {
    /// Create a service-backed client without constructing any provider.
    pub fn new(service: Arc<dyn SystemServiceClient>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl SystemAutonomyEvolutionClient for ServiceBackedAutonomyEvolutionClient {
    async fn transition(
        &self,
        command: EvolutionTransitionCommand,
    ) -> MacacaResult<EvolutionTransitionResult> {
        call(
            &self.service,
            AUTONOMY_EVOLUTION_TRANSITION_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn snapshot(
        &self,
        command: EvolutionSnapshotCommand,
    ) -> MacacaResult<EvolutionServiceSnapshot> {
        call(
            &self.service,
            AUTONOMY_EVOLUTION_SNAPSHOT_COMMAND,
            command.trace.clone(),
            command,
        )
        .await
    }

    async fn health(&self, trace: TraceContext) -> MacacaResult<EvolutionServiceSnapshot> {
        call(
            &self.service,
            AUTONOMY_EVOLUTION_HEALTH_COMMAND,
            trace.clone(),
            serde_json::json!({}),
        )
        .await
    }
}

async fn call<T, R>(
    service: &Arc<dyn SystemServiceClient>,
    command_name: &str,
    trace: TraceContext,
    payload: T,
) -> MacacaResult<R>
where
    T: serde::Serialize,
    R: serde::de::DeserializeOwned,
{
    let command = ServiceCallCommand::new(
        AUTONOMY_EVOLUTION_SERVICE_ID,
        command_name,
        serde_json::to_value(payload).map_err(|error| MacacaError::Config(error.to_string()))?,
    )?
    .with_trace(trace);
    let result = service.call_service(&command).await?;
    serde_json::from_value(result.output).map_err(|error| MacacaError::Config(error.to_string()))
}
