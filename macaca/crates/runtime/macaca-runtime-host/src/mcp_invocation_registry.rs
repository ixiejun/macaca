//! Service-owned MCP invocation lease registry.
//!
//! The registry is the State/Memento side of MCP serviceization.  It records
//! which scoped MCP runtime lease is active, how it is keyed, when it was last
//! used, and how cleanup completed.  It stores only bounded operational
//! metadata: no raw tool input, output, env, headers, credentials, or provider
//! payloads are retained.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use macaca_framework::mcp::McpSessionMode;
use tokio::sync::Mutex;

use crate::lease::McpSessionLease;
use crate::mcp_runtime::{
    McpRuntimeContext, McpRuntimeKey, McpRuntimeStatus, McpRuntimeStatusState, McpServerDefinition,
};

/// Bounded runtime metadata for one active MCP invocation lease.
#[derive(Debug, Clone)]
struct McpInvocationLeaseRecord {
    refs: usize,
    state: McpRuntimeStatusState,
    last_error: Option<String>,
    last_used: Instant,
    cleanup_status: Option<String>,
}

/// Runtime-host registry for scoped MCP invocation leases.
#[derive(Debug, Default)]
pub(crate) struct McpInvocationSessionRegistry {
    records: Mutex<BTreeMap<McpRuntimeKey, McpInvocationLeaseRecord>>,
}

impl McpInvocationSessionRegistry {
    /// Acquire a lease key for the lifecycle scope declared by the MCP server.
    ///
    /// The registry centralizes lifecycle keying so all production callers
    /// (framework tools, SDK callers, and WASM service calls) share the same
    /// isolation rules.  The returned lease is explicit ownership evidence used
    /// by dispatch and cleanup paths.
    pub async fn acquire(
        &self,
        definition: &McpServerDefinition,
        context: &McpRuntimeContext,
    ) -> McpSessionLease {
        let key = McpRuntimeKey::from_definition(definition, context);
        let mut records = self.records.lock().await;
        let record = records
            .entry(key.clone())
            .or_insert_with(|| McpInvocationLeaseRecord {
                refs: 0,
                state: McpRuntimeStatusState::Ready,
                last_error: None,
                last_used: Instant::now(),
                cleanup_status: None,
            });
        record.refs += 1;
        record.last_used = Instant::now();
        record.cleanup_status = None;
        tracing::debug!(
            server_id = %key.server_id,
            lifecycle = ?key.scope,
            refs = record.refs,
            "mcp invocation lease acquired"
        );
        McpSessionLease::new(key)
    }

    /// Release a lease, returning a sanitized status only when the record
    /// actually closes.
    pub async fn release(
        &self,
        lease: &McpSessionLease,
        force_remove: bool,
        reason: &str,
    ) -> Option<McpRuntimeStatus> {
        self.release_key(lease.key(), force_remove, reason).await
    }

    /// Force-release all leases matching a cleanup predicate.
    pub async fn cleanup_matching(
        &self,
        matches: impl Fn(&McpRuntimeKey) -> bool,
        reason: &str,
    ) -> Vec<McpRuntimeStatus> {
        let records = self.records.lock().await;
        let keys = records
            .keys()
            .filter(|key| matches(key))
            .cloned()
            .collect::<Vec<_>>();
        drop(records);
        let mut statuses = Vec::new();
        for key in keys {
            if let Some(status) = self.release_key(&key, true, reason).await {
                statuses.push(status);
            }
        }
        statuses
    }

    /// Release idle, unreferenced leases after the supplied TTL.
    pub async fn cleanup_idle(&self, ttl: Duration) -> Vec<McpRuntimeStatus> {
        let now = Instant::now();
        let records = self.records.lock().await;
        let keys = records
            .iter()
            .filter_map(|(key, record)| {
                (record.refs == 0 && now.duration_since(record.last_used) >= ttl)
                    .then_some(key.clone())
            })
            .collect::<Vec<_>>();
        drop(records);
        let mut statuses = Vec::new();
        for key in keys {
            if let Some(status) = self.release_key(&key, true, "idle_expired").await {
                statuses.push(status);
            }
        }
        statuses
    }

    /// Return whether a key is currently retained by the registry.
    #[cfg(test)]
    pub async fn contains(&self, key: &McpRuntimeKey) -> bool {
        self.records.lock().await.contains_key(key)
    }

    async fn release_key(
        &self,
        key: &McpRuntimeKey,
        force_remove: bool,
        reason: &str,
    ) -> Option<McpRuntimeStatus> {
        let mut records = self.records.lock().await;
        let record = records.get_mut(key)?;
        if !force_remove && record.refs > 1 {
            record.refs -= 1;
            record.last_used = Instant::now();
            tracing::debug!(
                server_id = %key.server_id,
                lifecycle = ?key.scope,
                refs = record.refs,
                "mcp invocation lease reference released"
            );
            return None;
        }
        record.cleanup_status = Some(reason.to_string());
        let record = records.remove(key)?;
        tracing::info!(
            server_id = %key.server_id,
            lifecycle = ?key.scope,
            cleanup_status = reason,
            "mcp invocation lease closed"
        );
        Some(status_from_record(key, record))
    }
}

fn status_from_record(key: &McpRuntimeKey, record: McpInvocationLeaseRecord) -> McpRuntimeStatus {
    McpRuntimeStatus {
        server_id: key.server_id.clone(),
        transport: "runtime".to_string(),
        lifecycle: key.scope.clone(),
        session_mode: McpSessionMode::Stateful,
        state: record.state,
        exposed_tools: Vec::new(),
        failure_reason: record.cleanup_status.or(record.last_error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    use macaca_framework::mcp::{McpSessionMode, McpTransportConfig};
    use macaca_proto::ApplicationId;

    use crate::mcp_runtime::{McpDefinitionSource, McpLifecycleScope};

    fn definition(id: &str, lifecycle: McpLifecycleScope) -> McpServerDefinition {
        McpServerDefinition {
            id: id.to_string(),
            transport: McpTransportConfig::Stdio {
                command: "server-bin".into(),
                args: Vec::new(),
                env: BTreeMap::new(),
                cwd: None,
            },
            lifecycle,
            session_mode: McpSessionMode::Stateful,
            tool_prefix: None,
            required_bins: Vec::new(),
            enabled: true,
            source: McpDefinitionSource::Compatibility,
            concurrency_isolation: None,
        }
    }

    fn context(session: &str, agent: &str) -> McpRuntimeContext {
        McpRuntimeContext {
            app_id: Some(ApplicationId(uuid::Uuid::nil())),
            session_id: Some(session.into()),
            agent_name: Some(agent.into()),
        }
    }

    #[tokio::test]
    async fn session_scope_reuses_one_key_with_reference_counting() {
        let registry = McpInvocationSessionRegistry::default();
        let definition = definition("server-a", McpLifecycleScope::Session);
        let lease_a = registry.acquire(&definition, &context("s1", "a1")).await;
        let lease_b = registry.acquire(&definition, &context("s1", "a2")).await;

        assert_eq!(lease_a.key(), lease_b.key());
        assert!(registry
            .release(&lease_a, false, "released")
            .await
            .is_none());
        assert!(registry.contains(lease_b.key()).await);
        assert!(registry
            .release(&lease_b, false, "released")
            .await
            .is_some());
    }

    #[tokio::test]
    async fn agent_session_scope_isolates_agents() {
        let registry = McpInvocationSessionRegistry::default();
        let definition = definition("server-a", McpLifecycleScope::AgentSession);
        let lease_a = registry.acquire(&definition, &context("s1", "a1")).await;
        let lease_b = registry.acquire(&definition, &context("s1", "a2")).await;

        assert_ne!(lease_a.key(), lease_b.key());
        let statuses = registry
            .cleanup_matching(
                |key| key.agent_name.as_deref() == Some("a1"),
                "agent_cleanup",
            )
            .await;
        assert_eq!(statuses.len(), 1);
        assert!(!registry.contains(lease_a.key()).await);
        assert!(registry.contains(lease_b.key()).await);
    }

    #[tokio::test]
    async fn call_scope_closes_after_single_release() {
        let registry = McpInvocationSessionRegistry::default();
        let definition = definition("server-a", McpLifecycleScope::Call);
        let lease = registry.acquire(&definition, &context("s1", "a1")).await;

        let status = registry
            .release(&lease, true, "call_completed")
            .await
            .unwrap();

        assert_eq!(status.server_id, "server-a");
        assert_eq!(status.failure_reason.as_deref(), Some("call_completed"));
        assert!(!registry.contains(lease.key()).await);
    }
}
