//! `SystemService` lifecycle implementation for the built-in Skill provider.
//!
//! This module owns provider start/stop/cleanup/health hooks.  Command bodies are
//! delegated to `command_dispatch` so the adapter stays a thin syscall surface.

use async_trait::async_trait;
use macaca_kernel::SystemService;
use macaca_proto::{
    ServiceCallResult, ServiceCommand, ServiceDescriptor, ServiceHealth, ServiceResult,
};

use super::SkillSystemServiceProvider;

#[async_trait]
impl SystemService for SkillSystemServiceProvider {
    fn descriptor(&self) -> ServiceDescriptor {
        self.descriptor.clone()
    }

    async fn start(&self) -> ServiceResult<()> {
        self.governance_state
            .configure_governance_event_journal_path(self.governance_event_journal_path.clone())
            .await;
        let replayed = self
            .governance_state
            .replay_governance_event_journal()
            .await;
        let recovered = self
            .governance_state
            .recover_materialized_skill_packages(&self.materialized_skill_roots)
            .await;
        tracing::info!(
            service_id = %self.descriptor.id,
            configured = self.snapshot_facade.is_some(),
            materialized_skill_roots = self.materialized_skill_roots.len(),
            governance_event_journal_configured = self.governance_event_journal_path.is_some(),
            replayed_governance_events = replayed,
            recovered_governance_records = recovered,
            "skill service provider started"
        );
        Ok(())
    }

    async fn call(&self, command: ServiceCommand) -> ServiceResult<ServiceCallResult> {
        let trace = Self::trace(&command)?;
        tracing::info!(
            service_id = %self.descriptor.id,
            command = %command.name,
            trace_id = %trace.trace_id,
            "skill service command accepted"
        );
        self.dispatch_command(command, trace).await
    }

    async fn stop(&self) -> ServiceResult<()> {
        tracing::info!(service_id = %self.descriptor.id, "skill service provider stopped");
        Ok(())
    }

    async fn cleanup(&self) -> ServiceResult<()> {
        tracing::info!(
            service_id = %self.descriptor.id,
            "skill service provider cleanup completed"
        );
        Ok(())
    }

    async fn health(&self) -> ServiceResult<ServiceHealth> {
        if self.snapshot_facade.is_some() {
            Ok(ServiceHealth::Healthy)
        } else {
            Ok(ServiceHealth::Unavailable {
                reason: "skill runtime is not configured".into(),
            })
        }
    }
}
