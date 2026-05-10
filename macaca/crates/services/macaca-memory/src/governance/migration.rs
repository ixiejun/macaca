use async_trait::async_trait;
use macaca_proto::{MacacaResult, MemoryEntry};

use crate::core::scope::MemoryScope;

use super::audit::{MemoryAuditEvent, MemoryAuditEventKind};
use super::runtime::MemoryGovernanceJournal;

/// Checkpointed memory provider migration plan.
#[derive(Debug, Clone)]
pub struct MemoryProviderMigrationPlan {
    pub source_provider_id: String,
    pub target_provider_id: String,
    pub scope: MemoryScope,
    pub batch_size: usize,
}

/// Provider migration lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryProviderMigrationStatus {
    Planned,
    Copying,
    Verifying,
    Completed,
    Failed,
    RolledBack,
}

/// Result of one migration execution.
#[derive(Debug, Clone)]
pub struct MemoryProviderMigrationResult {
    pub status: MemoryProviderMigrationStatus,
    pub source_authoritative: bool,
    pub copied_entries: usize,
    pub verified_entries: usize,
    pub diagnostics: Vec<String>,
}

/// Minimal source/target port used by the migration runtime.
#[async_trait]
pub trait MemoryProviderMigrationPort: Send + Sync {
    async fn list_entries(
        &self,
        scope: &MemoryScope,
        limit: usize,
    ) -> MacacaResult<Vec<MemoryEntry>>;
    async fn write_entry(&self, entry: MemoryEntry) -> MacacaResult<()>;
}

/// Copy/verify migration executor.
pub struct MemoryProviderMigrationRuntime<J> {
    journal: J,
}

impl<J> MemoryProviderMigrationRuntime<J> {
    pub fn new(journal: J) -> Self {
        Self { journal }
    }
}

impl<J> MemoryProviderMigrationRuntime<J>
where
    J: MemoryGovernanceJournal,
{
    pub async fn migrate<S, T>(
        &self,
        plan: MemoryProviderMigrationPlan,
        source: &S,
        target: &T,
    ) -> MacacaResult<MemoryProviderMigrationResult>
    where
        S: MemoryProviderMigrationPort,
        T: MemoryProviderMigrationPort,
    {
        let limit = plan.batch_size.max(1);
        let source_entries = source.list_entries(&plan.scope, limit).await?;
        for entry in &source_entries {
            target.write_entry(entry.clone()).await?;
        }
        let target_entries = target.list_entries(&plan.scope, limit).await?;
        let source_ids: std::collections::BTreeSet<_> = source_entries
            .iter()
            .map(|entry| entry.id.0.to_string())
            .collect();
        let target_ids: std::collections::BTreeSet<_> = target_entries
            .iter()
            .map(|entry| entry.id.0.to_string())
            .collect();

        if source_ids != target_ids {
            let reason = format!(
                "migration verification failed: copied={} verified={}",
                source_ids.len(),
                target_ids.len()
            );
            self.journal
                .append_audit(
                    MemoryAuditEvent::new(
                        MemoryAuditEventKind::PropagatedDelete,
                        plan.scope,
                        reason.clone(),
                    )
                    .provider_id(format!(
                        "{}->{}",
                        plan.source_provider_id, plan.target_provider_id
                    )),
                )
                .await?;
            return Ok(MemoryProviderMigrationResult {
                status: MemoryProviderMigrationStatus::Failed,
                source_authoritative: true,
                copied_entries: source_ids.len(),
                verified_entries: target_ids.len(),
                diagnostics: vec![reason],
            });
        }

        let scope = plan.scope;
        self.journal
            .append_audit(
                MemoryAuditEvent::new(
                    MemoryAuditEventKind::ArtifactGenerated,
                    scope,
                    "migration completed after copy and verification",
                )
                .provider_id(format!(
                    "{}->{}",
                    plan.source_provider_id, plan.target_provider_id
                )),
            )
            .await?;
        Ok(MemoryProviderMigrationResult {
            status: MemoryProviderMigrationStatus::Completed,
            source_authoritative: false,
            copied_entries: source_ids.len(),
            verified_entries: target_ids.len(),
            diagnostics: Vec::new(),
        })
    }
}
