//! Provider-neutral Memory service command contracts.
//!
//! `macaca-proto` owns these DTOs so SDK clients, shells, and runtime-host
//! providers share one protocol surface without depending on the concrete
//! Memory service crate or host composition root. The dense DTO families are
//! split by role while re-exporting the same public type names from this module.

mod commands;
mod results;
mod scope;

pub use commands::{
    MemoryForgetCommand, MemoryGetCommand, MemoryPrefetchCommand, MemoryRecallCommand,
    MemoryRememberCommand, MemoryServiceSnapshotCommand, MemoryStatusCommand,
};
pub use results::{
    MemoryGetResult, MemoryRecallResult, MemoryRememberResult, MemoryServiceSnapshot,
    MemoryTopologyLabels,
};
pub use scope::{
    MemoryCapabilitySet, MemoryIdentity, MemoryPolicyHints, MemoryScope, MemoryStatusReport,
    MemoryVisibility,
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

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use serde_json::json;
    use uuid::Uuid;

    use super::*;
    use crate::{AgentId, ApplicationId, TraceContext};

    #[test]
    fn memory_remember_command_preserves_serde_shape_after_split() {
        let payload = json!({
            "scope": {
                "tenant_id": null,
                "user_id": null,
                "namespace": "workspace",
                "identity": {
                    "application_id": Uuid::nil(),
                    "agent_id": Uuid::nil(),
                    "agent_name": null,
                    "session_id": null,
                    "project_id": null
                },
                "visibility": "AgentPrivate"
            },
            "trace": {
                "trace_id": "trace-memory",
                "session_id": null,
                "task_id": null,
                "agent": null,
                "emitted_at": Utc::now()
            },
            "content": "remember this",
            "layer": "Session",
            "metadata": null,
            "policy": {
                "privacy_tier": null,
                "max_results": null,
                "metadata": {}
            }
        });

        let command: MemoryRememberCommand =
            serde_json::from_value(payload).expect("legacy memory command shape should decode");
        assert_eq!(command.content, "remember this");
        assert_eq!(
            command.scope.identity.application_id,
            ApplicationId(Uuid::nil())
        );
        assert_eq!(command.scope.identity.agent_id, Some(AgentId(Uuid::nil())));

        let encoded = serde_json::to_value(&command).expect("memory command should encode");
        assert_eq!(encoded["scope"]["visibility"], "AgentPrivate");
        assert_eq!(encoded["policy"]["metadata"], json!({}));
    }

    #[test]
    fn memory_scope_validation_still_blocks_global_recall() {
        let scope = MemoryScope::new(ApplicationId(Uuid::nil()), MemoryVisibility::GlobalSystem);
        let trace = TraceContext {
            trace_id: "trace-memory".into(),
            session_id: None,
            task_id: None,
            agent: None,
            emitted_at: Utc::now(),
        };

        let error = MemoryRecallCommand::new(scope, trace, "query", 1)
            .expect_err("global recall must remain blocked");
        assert!(error.to_string().contains("Memory recall requires"));
    }
}
