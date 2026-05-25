//! Service contract and Null Object provider for `service.autonomy_evolution`.
//!
//! The trait is command-oriented so runtime decorators can attach trace, policy,
//! resource, entitlement, package guard, metering, and sanitized audit behavior
//! before a provider advances a run. Provider absence is explicit and
//! fail-closed; callers never need to infer optional-service availability from
//! panics or empty shell output.

use async_trait::async_trait;
use macaca_proto::{
    CapabilityId, CleanupPolicy, KernelServiceId, MacacaResult, ServiceCapability,
    ServiceDescriptor, ServiceHealth, ServiceLifecycleState, ServiceScope, ServiceType,
    TraceContext, TraceSchemaRef,
};
use tracing::{info, warn};

use crate::{
    EvolutionAdmissionCommand, EvolutionAdmissionResult, EvolutionBenchmarkCommand,
    EvolutionBenchmarkResult, EvolutionScope, EvolutionServiceSnapshot, EvolutionSnapshotCommand,
    EvolutionTransitionCommand, EvolutionTransitionResult, AUTONOMY_EVOLUTION_ADMISSION_COMMAND,
    AUTONOMY_EVOLUTION_BENCHMARK_COMMAND, AUTONOMY_EVOLUTION_HEALTH_COMMAND,
    AUTONOMY_EVOLUTION_SERVICE_ID, AUTONOMY_EVOLUTION_SNAPSHOT_COMMAND,
    AUTONOMY_EVOLUTION_TRANSITION_COMMAND,
};

#[async_trait]
pub trait AutonomyEvolutionService: Send + Sync {
    fn descriptor(&self) -> ServiceDescriptor;

    async fn health(&self, trace: TraceContext) -> MacacaResult<EvolutionServiceSnapshot>;

    async fn snapshot(
        &self,
        command: EvolutionSnapshotCommand,
    ) -> MacacaResult<EvolutionServiceSnapshot>;

    async fn transition(
        &self,
        command: EvolutionTransitionCommand,
    ) -> MacacaResult<EvolutionTransitionResult>;

    async fn admit_candidate(
        &self,
        command: EvolutionAdmissionCommand,
    ) -> MacacaResult<EvolutionAdmissionResult>;

    async fn run_paired_benchmark(
        &self,
        command: EvolutionBenchmarkCommand,
    ) -> MacacaResult<EvolutionBenchmarkResult>;
}

#[derive(Debug, Clone)]
pub struct UnavailableAutonomyEvolutionProvider {
    provider_id: String,
    reason: String,
}

impl UnavailableAutonomyEvolutionProvider {
    /// Create the Null Object provider with a safe, bounded reason string.
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            provider_id: "unavailable".into(),
            reason: reason.into(),
        }
    }

    fn unavailable_result(
        &self,
        command: &EvolutionTransitionCommand,
    ) -> EvolutionTransitionResult {
        warn!(
            service_id = AUTONOMY_EVOLUTION_SERVICE_ID,
            provider_id = self.provider_id.as_str(),
            trace_id = command.trace.trace_id.as_str(),
            "autonomy evolution transition rejected because provider is unavailable"
        );
        EvolutionTransitionResult::denied(
            command,
            command
                .from_state
                .clone()
                .unwrap_or(crate::EvolutionRunState::Observed),
            format!("autonomy evolution provider unavailable: {}", self.reason),
        )
    }
}

impl Default for UnavailableAutonomyEvolutionProvider {
    fn default() -> Self {
        Self::new("autonomy evolution control plane provider is not installed")
    }
}

#[async_trait]
impl AutonomyEvolutionService for UnavailableAutonomyEvolutionProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        autonomy_evolution_service_descriptor(ServiceHealth::Unavailable {
            reason: self.reason.clone(),
        })
    }

    async fn health(&self, trace: TraceContext) -> MacacaResult<EvolutionServiceSnapshot> {
        info!(
            service_id = AUTONOMY_EVOLUTION_SERVICE_ID,
            provider_id = self.provider_id.as_str(),
            trace_id = trace.trace_id.as_str(),
            "autonomy evolution health requested from unavailable provider"
        );
        Ok(EvolutionServiceSnapshot::unavailable(self.reason.clone()))
    }

    async fn snapshot(
        &self,
        command: EvolutionSnapshotCommand,
    ) -> MacacaResult<EvolutionServiceSnapshot> {
        info!(
            service_id = AUTONOMY_EVOLUTION_SERVICE_ID,
            provider_id = self.provider_id.as_str(),
            trace_id = command.trace.trace_id.as_str(),
            "autonomy evolution snapshot requested from unavailable provider"
        );
        Ok(EvolutionServiceSnapshot::unavailable(self.reason.clone()))
    }

    async fn transition(
        &self,
        command: EvolutionTransitionCommand,
    ) -> MacacaResult<EvolutionTransitionResult> {
        Ok(self.unavailable_result(&command))
    }

    async fn admit_candidate(
        &self,
        command: EvolutionAdmissionCommand,
    ) -> MacacaResult<EvolutionAdmissionResult> {
        warn!(
            service_id = AUTONOMY_EVOLUTION_SERVICE_ID,
            provider_id = self.provider_id.as_str(),
            trace_id = command.trace.trace_id.as_str(),
            candidate_id = command.candidate.candidate_id.as_str(),
            "autonomy evolution admission rejected because provider is unavailable"
        );
        Ok(EvolutionAdmissionResult::unavailable(
            &command,
            format!("autonomy evolution provider unavailable: {}", self.reason),
        ))
    }

    async fn run_paired_benchmark(
        &self,
        command: EvolutionBenchmarkCommand,
    ) -> MacacaResult<EvolutionBenchmarkResult> {
        warn!(
            service_id = AUTONOMY_EVOLUTION_SERVICE_ID,
            provider_id = self.provider_id.as_str(),
            trace_id = command.trace.trace_id.as_str(),
            benchmark_id = command.benchmark_id.as_str(),
            "autonomy evolution benchmark rejected because provider is unavailable"
        );
        Ok(EvolutionBenchmarkResult::unavailable(
            &command,
            format!("autonomy evolution provider unavailable: {}", self.reason),
        ))
    }
}

/// Descriptor helper shared by concrete and unavailable providers.
pub fn autonomy_evolution_service_descriptor(health: ServiceHealth) -> ServiceDescriptor {
    let mut descriptor = ServiceDescriptor::new(
        KernelServiceId::new(AUTONOMY_EVOLUTION_SERVICE_ID),
        ServiceType::new("autonomy.evolution_control_plane"),
        TraceSchemaRef::new("macaca.trace.autonomy_evolution.v1"),
    );
    descriptor.lifecycle_state = ServiceLifecycleState::Registered;
    descriptor.health = health;
    descriptor.supported_scopes = vec![
        ServiceScope::Global,
        ServiceScope::Application(String::new()),
        ServiceScope::Session(String::new()),
    ];
    descriptor.cleanup_policy = CleanupPolicy::None;
    descriptor.capabilities = vec![
        ServiceCapability::new(
            CapabilityId::new(AUTONOMY_EVOLUTION_TRANSITION_COMMAND),
            "Advance a provider-neutral evolution run lifecycle",
        ),
        ServiceCapability::new(
            CapabilityId::new(AUTONOMY_EVOLUTION_ADMISSION_COMMAND),
            "Evaluate an evolution candidate through admission quality gates",
        ),
        ServiceCapability::new(
            CapabilityId::new(AUTONOMY_EVOLUTION_BENCHMARK_COMMAND),
            "Score a normalized paired evolution benchmark",
        ),
        ServiceCapability::new(
            CapabilityId::new(AUTONOMY_EVOLUTION_SNAPSHOT_COMMAND),
            "Read bounded autonomy evolution run snapshots",
        ),
        ServiceCapability::new(
            CapabilityId::new(AUTONOMY_EVOLUTION_HEALTH_COMMAND),
            "Read autonomy evolution control plane health",
        ),
    ];
    descriptor
}

impl EvolutionScope {
    pub(crate) fn matches_filter(&self, filter: &EvolutionScope) -> bool {
        filter
            .application_id
            .as_ref()
            .map(|value| self.application_id.as_ref() == Some(value))
            .unwrap_or(true)
            && filter
                .tenant_id
                .as_ref()
                .map(|value| self.tenant_id.as_ref() == Some(value))
                .unwrap_or(true)
            && filter
                .session_id
                .as_ref()
                .map(|value| self.session_id.as_ref() == Some(value))
                .unwrap_or(true)
            && filter
                .task_id
                .as_ref()
                .map(|value| self.task_id.as_ref() == Some(value))
                .unwrap_or(true)
    }
}
