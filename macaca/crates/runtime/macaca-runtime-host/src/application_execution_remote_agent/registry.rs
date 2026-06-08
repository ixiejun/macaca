//! Health and lease Registry for remote application-execution agents.
//!
//! The Registry owns in-memory admission state only.  It performs capability,
//! tenant, health, and lease-budget matching during selection but never executes
//! application logic or inspects business payloads.

use std::collections::BTreeMap;

use chrono::Utc;
use macaca_proto::{
    ApplicationExecutionProviderHealth, ApplicationExecutionProviderLease,
    ApplicationExecutionScope, ServiceError, StartApplicationExecutionCommand,
};
use tokio::sync::RwLock;
use tracing::info;

use super::registration::RemoteAgentRegistration;

/// Health and lease registry for remote agents.
#[derive(Default)]
pub struct RemoteAgentRegistry {
    agents: RwLock<BTreeMap<String, RemoteAgentRegistration>>,
    leases: RwLock<BTreeMap<String, ApplicationExecutionProviderLease>>,
}

impl RemoteAgentRegistry {
    /// Build an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register one remote agent after descriptor-style validation.
    pub async fn register(&self, agent: RemoteAgentRegistration) -> Result<(), ServiceError> {
        agent.validate()?;
        info!(
            agent_id = %agent.agent_id,
            transport_kind = %agent.transport.transport_kind,
            capabilities = agent.capability_declarations.len(),
            "remote agent registration admitted"
        );
        self.agents
            .write()
            .await
            .insert(agent.agent_id.clone(), agent);
        Ok(())
    }

    /// Select the first healthy agent that satisfies capability and lease constraints.
    pub(super) async fn select(
        &self,
        command: &StartApplicationExecutionCommand,
    ) -> Result<RemoteAgentRegistration, ServiceError> {
        let agents = self.agents.read().await;
        let leases = self.leases.read().await;
        for agent in agents.values() {
            if let Some(tenant_id) = &agent.tenant_id {
                if command.tenant_id.as_ref() != Some(tenant_id) {
                    continue;
                }
            }
            if matches!(
                agent.health_state,
                ApplicationExecutionProviderHealth::Unavailable { .. }
            ) {
                continue;
            }
            let active_lease_count = leases
                .values()
                .filter(|lease| {
                    lease.provider_id == agent.agent_id && lease.expires_at > Utc::now()
                })
                .count();
            if active_lease_count >= agent.max_leases {
                continue;
            }
            let capability_match = command.requested_capabilities.iter().all(|required| {
                agent
                    .capability_declarations
                    .iter()
                    .any(|declared| declared == required)
            });
            if capability_match {
                return Ok(agent.clone());
            }
        }
        Err(ServiceError::ServiceUnavailable(
            "no healthy remote agent matched capability, tenant, and lease constraints".into(),
        ))
    }

    /// Persist one newly issued lease for later control and gateway validation.
    pub(super) async fn store_lease(&self, lease: ApplicationExecutionProviderLease) {
        self.leases
            .write()
            .await
            .insert(lease.lease_id.clone(), lease);
    }

    /// Resolve the active lease for one execution scope.
    pub(super) async fn lease_for_scope(
        &self,
        scope: &ApplicationExecutionScope,
    ) -> Option<ApplicationExecutionProviderLease> {
        self.leases
            .read()
            .await
            .values()
            .find(|lease| {
                lease.scope.application_id == scope.application_id
                    && lease.scope.session_id == scope.session_id
                    && lease.scope.run_id == scope.run_id
            })
            .cloned()
    }

    /// Look up the registration associated with one stored lease.
    pub(super) async fn agent_for_lease(
        &self,
        lease: &ApplicationExecutionProviderLease,
    ) -> Option<RemoteAgentRegistration> {
        self.agents.read().await.get(&lease.provider_id).cloned()
    }

    /// Resolve one lease by id for gateway ingress validation.
    pub(super) async fn lease_by_id(
        &self,
        lease_id: &str,
    ) -> Option<ApplicationExecutionProviderLease> {
        self.leases.read().await.get(lease_id).cloned()
    }

    /// Return the most recently stored lease for lightweight snapshot probes.
    pub(super) async fn latest_lease(&self) -> Option<ApplicationExecutionProviderLease> {
        self.leases.read().await.values().last().cloned()
    }

    /// Report whether any remote agents have been admitted.
    pub(super) async fn has_agents(&self) -> bool {
        !self.agents.read().await.is_empty()
    }
}
