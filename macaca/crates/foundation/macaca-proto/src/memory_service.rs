//! Provider-neutral Memory service command contracts.
//!
//! These DTOs are owned by `macaca-proto` so SDK clients, shells, and
//! runtime-host providers share one protocol surface without depending on the
//! concrete Memory service crate or host composition root.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    AgentId, ApplicationId, MacacaError, MacacaResult, MemoryEntry, MemoryId, MemoryLayer,
    TraceContext,
};

/// Stable service id used by runtime-host registration and SDK clients.
pub const MEMORY_SERVICE_ID: &str = "service.memory";

/// Command names accepted by the Memory service provider adapter.
pub const MEMORY_REMEMBER_COMMAND: &str = "memory.remember";
pub const MEMORY_RECALL_COMMAND: &str = "memory.recall";
pub const MEMORY_PREFETCH_COMMAND: &str = "memory.prefetch";
pub const MEMORY_GET_COMMAND: &str = "memory.get";
pub const MEMORY_FORGET_COMMAND: &str = "memory.forget";
pub const MEMORY_STATUS_COMMAND: &str = "memory.status";
pub const MEMORY_SNAPSHOT_COMMAND: &str = "memory.snapshot";

/// Visibility classes supported by the memory fabric.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MemoryVisibility {
    AgentPrivate,
    SessionShared,
    ApplicationShared,
    UserScoped,
    GlobalSystem,
}

/// Stable identity fields that tie a memory scope to the running Agent OS.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemoryIdentity {
    pub application_id: ApplicationId,
    pub agent_id: Option<AgentId>,
    pub agent_name: Option<String>,
    pub session_id: Option<String>,
    pub project_id: Option<String>,
}

impl MemoryIdentity {
    /// Create an identity rooted at an application.
    pub fn new(application_id: ApplicationId) -> Self {
        Self {
            application_id,
            agent_id: None,
            agent_name: None,
            session_id: None,
            project_id: None,
        }
    }
}

/// Full scoping envelope for every memory-fabric request.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemoryScope {
    pub tenant_id: Option<String>,
    pub user_id: Option<String>,
    pub namespace: Option<String>,
    pub identity: MemoryIdentity,
    pub visibility: MemoryVisibility,
}

impl MemoryScope {
    /// Create a new scope with only application identity and target visibility.
    pub fn new(application_id: ApplicationId, visibility: MemoryVisibility) -> Self {
        Self {
            tenant_id: None,
            user_id: None,
            namespace: None,
            identity: MemoryIdentity::new(application_id),
            visibility,
        }
    }

    /// Convenience constructor for agent-private memory keyed by concrete agent id.
    pub fn agent_private(application_id: ApplicationId, agent_id: AgentId) -> Self {
        Self::new(application_id, MemoryVisibility::AgentPrivate).agent_id(agent_id)
    }

    /// Convenience constructor for agent-private memory keyed by agent name.
    pub fn agent_private_named(
        application_id: ApplicationId,
        agent_name: impl Into<String>,
    ) -> Self {
        Self::new(application_id, MemoryVisibility::AgentPrivate).agent_name(agent_name)
    }

    /// Convenience constructor for session-shared memory.
    pub fn session_shared(application_id: ApplicationId, session_id: impl Into<String>) -> Self {
        Self::new(application_id, MemoryVisibility::SessionShared).session_id(session_id)
    }

    /// Convenience constructor for project-shared memory.
    pub fn project_shared(application_id: ApplicationId, project_id: impl Into<String>) -> Self {
        Self::new(application_id, MemoryVisibility::SessionShared).project_id(project_id)
    }

    pub fn agent_id(mut self, agent_id: AgentId) -> Self {
        self.identity.agent_id = Some(agent_id);
        self
    }

    pub fn agent_name(mut self, agent_name: impl Into<String>) -> Self {
        self.identity.agent_name = Some(agent_name.into());
        self
    }

    pub fn session_id(mut self, session_id: impl Into<String>) -> Self {
        self.identity.session_id = Some(session_id.into());
        self
    }

    pub fn project_id(mut self, project_id: impl Into<String>) -> Self {
        self.identity.project_id = Some(project_id.into());
        self
    }

    pub fn tenant_id(mut self, tenant_id: impl Into<String>) -> Self {
        self.tenant_id = Some(tenant_id.into());
        self
    }

    pub fn user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    pub fn namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = Some(namespace.into());
        self
    }

    /// Validate the minimum identity required by the selected visibility.
    pub fn validate(&self) -> MacacaResult<()> {
        match self.visibility {
            MemoryVisibility::AgentPrivate => self.validate_agent_private(),
            MemoryVisibility::SessionShared => self.validate_session_shared(),
            MemoryVisibility::ApplicationShared => Ok(()),
            MemoryVisibility::UserScoped => self.require_non_empty(
                self.user_id.as_deref(),
                "UserScoped memory requires user_id",
            ),
            MemoryVisibility::GlobalSystem => Ok(()),
        }
    }

    /// Return the agent id if the scope already carries one.
    pub fn agent_id_value(&self) -> Option<AgentId> {
        self.identity.agent_id
    }

    fn validate_agent_private(&self) -> MacacaResult<()> {
        if self.identity.agent_id.is_none()
            && self
                .identity
                .agent_name
                .as_deref()
                .is_none_or(str::is_empty)
        {
            return Err(MacacaError::Memory(
                "AgentPrivate memory requires agent_id or agent_name".into(),
            ));
        }
        Ok(())
    }

    fn validate_session_shared(&self) -> MacacaResult<()> {
        if self
            .identity
            .session_id
            .as_deref()
            .is_none_or(str::is_empty)
            && self
                .identity
                .project_id
                .as_deref()
                .is_none_or(str::is_empty)
        {
            return Err(MacacaError::Memory(
                "SessionShared memory requires session_id or project_id".into(),
            ));
        }
        Ok(())
    }

    fn require_non_empty(&self, value: Option<&str>, message: &str) -> MacacaResult<()> {
        if value.is_none_or(str::is_empty) {
            return Err(MacacaError::Memory(message.into()));
        }
        Ok(())
    }
}

/// Capability bitmap for a memory provider or adapter.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryCapabilitySet {
    pub store: bool,
    pub search: bool,
    pub prompt: bool,
    pub lifecycle: bool,
    pub flush: bool,
    pub artifact: bool,
    pub governance: bool,
    pub knowledge: bool,
}

impl MemoryCapabilitySet {
    /// Helper for the common builtin case: the provider can write and recall memory.
    pub fn basic_store_search() -> Self {
        Self {
            store: true,
            search: true,
            ..Self::default()
        }
    }
}

/// Health/status snapshot returned by a `MemoryFacade`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryStatusReport {
    pub provider_id: String,
    pub healthy: bool,
    pub capabilities: MemoryCapabilitySet,
    pub message: Option<String>,
}

impl MemoryStatusReport {
    /// Convenience constructor for a healthy provider status.
    pub fn healthy(provider_id: impl Into<String>, capabilities: MemoryCapabilitySet) -> Self {
        Self {
            provider_id: provider_id.into(),
            healthy: true,
            capabilities,
            message: None,
        }
    }
}

/// Provider-neutral policy hints for memory operations.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemoryPolicyHints {
    pub privacy_tier: Option<String>,
    pub max_results: Option<usize>,
    pub metadata: BTreeMap<String, String>,
}

/// Command for writing one memory item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRememberCommand {
    pub scope: MemoryScope,
    pub trace: TraceContext,
    pub content: String,
    pub layer: MemoryLayer,
    pub metadata: Value,
    pub policy: MemoryPolicyHints,
}

impl MemoryRememberCommand {
    /// Build a scoped write command and validate the isolation boundary.
    pub fn new(
        scope: MemoryScope,
        trace: TraceContext,
        content: impl Into<String>,
    ) -> MacacaResult<Self> {
        validate_scope_and_trace(&scope, &trace)?;
        let content = content.into();
        if content.trim().is_empty() {
            return Err(MacacaError::Memory(
                "Memory remember command requires non-empty content".into(),
            ));
        }
        Ok(Self {
            scope,
            trace,
            content,
            layer: MemoryLayer::Session,
            metadata: Value::Null,
            policy: MemoryPolicyHints::default(),
        })
    }
}

/// Command for scoped recall/search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRecallCommand {
    pub scope: MemoryScope,
    pub trace: TraceContext,
    pub query: String,
    pub limit: usize,
    pub policy: MemoryPolicyHints,
}

impl MemoryRecallCommand {
    /// Build a scoped recall command with bounded result size.
    pub fn new(
        scope: MemoryScope,
        trace: TraceContext,
        query: impl Into<String>,
        limit: usize,
    ) -> MacacaResult<Self> {
        validate_scope_and_trace(&scope, &trace)?;
        validate_no_global_recall(&scope)?;
        let query = query.into();
        if query.trim().is_empty() {
            return Err(MacacaError::Memory(
                "Memory recall command requires non-empty query".into(),
            ));
        }
        if limit == 0 {
            return Err(MacacaError::Memory(
                "Memory recall command requires positive limit".into(),
            ));
        }
        Ok(Self {
            scope,
            trace,
            query,
            limit,
            policy: MemoryPolicyHints::default(),
        })
    }
}

/// Command for prompt-oriented prefetch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryPrefetchCommand {
    pub scope: MemoryScope,
    pub trace: TraceContext,
    pub query: String,
    pub limit: usize,
    pub policy: MemoryPolicyHints,
}

/// Command for scoped point lookup by memory id.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryGetCommand {
    pub scope: MemoryScope,
    pub trace: TraceContext,
    pub id: MemoryId,
    pub policy: MemoryPolicyHints,
}

/// Command for scoped deletion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryForgetCommand {
    pub scope: MemoryScope,
    pub trace: TraceContext,
    pub id: MemoryId,
    pub policy: MemoryPolicyHints,
}

/// Command for lightweight service status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStatusCommand {
    pub scope: MemoryScope,
    pub trace: TraceContext,
}

/// Command for deterministic snapshots.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryServiceSnapshotCommand {
    pub scope: MemoryScope,
    pub trace: TraceContext,
    pub include_governance: bool,
}

/// Result for memory write operations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemoryRememberResult {
    pub id: MemoryId,
    pub stored_at: DateTime<Utc>,
}

/// Result for recall and prefetch operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRecallResult {
    pub entries: Vec<MemoryEntry>,
    pub total_candidates: usize,
    pub returned_at: DateTime<Utc>,
}

impl MemoryRecallResult {
    /// Wrap entries with deterministic count metadata.
    pub fn new(entries: Vec<MemoryEntry>) -> Self {
        let total_candidates = entries.len();
        Self {
            entries,
            total_candidates,
            returned_at: Utc::now(),
        }
    }
}

/// Result for scoped point lookup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryGetResult {
    pub entry: Option<MemoryEntry>,
    pub returned_at: DateTime<Utc>,
}

impl MemoryGetResult {
    /// Wrap an optional entry with deterministic response metadata.
    pub fn new(entry: Option<MemoryEntry>) -> Self {
        Self {
            entry,
            returned_at: Utc::now(),
        }
    }
}

/// Provider-neutral topology labels exposed by Memory Service snapshots.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemoryTopologyLabels {
    pub application_namespace: String,
    pub agent_collection: String,
    pub shared_collection: Option<String>,
}

/// Deterministic Memory Service snapshot.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemoryServiceSnapshot {
    pub service_id: String,
    pub provider_id: String,
    pub healthy: bool,
    pub capabilities: MemoryCapabilitySet,
    pub topology: Option<MemoryTopologyLabels>,
    pub governance_counts: BTreeMap<String, u64>,
    pub last_audit_ids: Vec<String>,
    pub captured_at: DateTime<Utc>,
}

impl MemoryServiceSnapshot {
    /// Build a snapshot from provider status and optional topology labels.
    pub fn new(
        provider_id: impl Into<String>,
        healthy: bool,
        capabilities: MemoryCapabilitySet,
        topology: Option<MemoryTopologyLabels>,
    ) -> Self {
        Self {
            service_id: MEMORY_SERVICE_ID.into(),
            provider_id: provider_id.into(),
            healthy,
            capabilities,
            topology,
            governance_counts: BTreeMap::new(),
            last_audit_ids: Vec::new(),
            captured_at: Utc::now(),
        }
    }
}

fn validate_scope_and_trace(scope: &MemoryScope, trace: &TraceContext) -> MacacaResult<()> {
    scope.validate()?;
    if trace.trace_id.trim().is_empty() {
        return Err(MacacaError::Config(
            "Memory service command requires trace_id".into(),
        ));
    }
    Ok(())
}

fn validate_no_global_recall(scope: &MemoryScope) -> MacacaResult<()> {
    if matches!(
        scope.visibility,
        MemoryVisibility::ApplicationShared | MemoryVisibility::GlobalSystem
    ) {
        return Err(MacacaError::Memory(
            "Memory recall requires AgentPrivate or SessionShared scope".into(),
        ));
    }
    Ok(())
}
