use std::sync::Arc;

use macaca_memory::{MemoryPolicyHints, MemoryRememberCommand, MemoryScope};
use macaca_proto::{ApplicationId, MemoryLayer, TraceContext};
use tracing::{info, warn};

/// Persist sanitized completion evidence into the Memory Service.
///
/// This module is a Web-shell Observer over the generic chat lifecycle. It does
/// not decide what an application means, does not parse model output, and does
/// not write directly to any vector database. Instead it creates one bounded,
/// provider-neutral `memory.remember` command and sends it through
/// `SystemMemoryClient`, so the Memory service remains the owner of routing,
/// policy, provider selection, and audit.
pub(crate) async fn capture_successful_session_completion(
    client: Arc<dyn macaca_sdk::SystemMemoryClient>,
    app_id: ApplicationId,
    session_id: impl Into<String>,
    agent_name: impl Into<String>,
    final_content: impl AsRef<str>,
    trace: TraceContext,
) {
    let session_id = session_id.into();
    let agent_name = agent_name.into();
    let output_chars = final_content.as_ref().chars().count();
    let scope = MemoryScope::session_shared(app_id, session_id.clone()).agent_name(agent_name);
    let content = format!(
        "Chat session completed successfully. output_chars={output_chars}; source=chat_session_completion"
    );
    let metadata = serde_json::json!({
        "source": "chat_session_completion",
        "session_id": session_id,
        "output_chars": output_chars,
        "capture_version": 1,
    });

    let mut command = match MemoryRememberCommand::new(scope, trace, content) {
        Ok(command) => command,
        Err(error) => {
            warn!(
                error = %error,
                "session memory capture skipped because command construction failed"
            );
            return;
        }
    };
    command.layer = MemoryLayer::Vector;
    command.metadata = metadata;
    command.policy = MemoryPolicyHints {
        privacy_tier: Some("derived_operational_summary".into()),
        max_results: None,
        metadata: std::collections::BTreeMap::from([(
            "capture_source".into(),
            "chat_session_completion".into(),
        )]),
    };

    let trace_id = command.trace.trace_id.clone();
    match client.remember(command).await {
        Ok(result) => {
            info!(
                trace_id = %trace_id,
                memory_id = %result.id.0,
                "session memory capture stored completion summary"
            );
        }
        Err(error) => {
            warn!(
                trace_id = %trace_id,
                error = %error,
                "session memory capture failed without failing chat completion"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::capture_successful_session_completion;

    use std::sync::Arc;

    use async_trait::async_trait;
    use macaca_memory::{
        MemoryForgetCommand, MemoryGetCommand, MemoryGetResult, MemoryPrefetchCommand,
        MemoryRecallCommand, MemoryRecallResult, MemoryRememberCommand, MemoryRememberResult,
        MemoryServiceSnapshot, MemoryServiceSnapshotCommand, MemoryStatusCommand,
        MemoryStatusReport,
    };
    use macaca_proto::{ApplicationId, MacacaResult, MemoryId, TraceContext};
    use tokio::sync::Mutex;

    #[derive(Default)]
    struct RecordingMemoryClient {
        commands: Mutex<Vec<MemoryRememberCommand>>,
    }

    #[async_trait]
    impl macaca_sdk::SystemMemoryClient for RecordingMemoryClient {
        async fn remember(
            &self,
            command: MemoryRememberCommand,
        ) -> MacacaResult<MemoryRememberResult> {
            self.commands.lock().await.push(command);
            Ok(MemoryRememberResult {
                id: MemoryId::new(),
                stored_at: chrono::Utc::now(),
            })
        }

        async fn recall(&self, _command: MemoryRecallCommand) -> MacacaResult<MemoryRecallResult> {
            unreachable!("session capture should not recall")
        }

        async fn prefetch(
            &self,
            _command: MemoryPrefetchCommand,
        ) -> MacacaResult<MemoryRecallResult> {
            unreachable!("session capture should not prefetch")
        }

        async fn get(&self, _command: MemoryGetCommand) -> MacacaResult<MemoryGetResult> {
            unreachable!("session capture should not get")
        }

        async fn forget(&self, _command: MemoryForgetCommand) -> MacacaResult<()> {
            unreachable!("session capture should not forget")
        }

        async fn status(&self, _command: MemoryStatusCommand) -> MacacaResult<MemoryStatusReport> {
            unreachable!("session capture should not request status")
        }

        async fn snapshot(
            &self,
            _command: MemoryServiceSnapshotCommand,
        ) -> MacacaResult<MemoryServiceSnapshot> {
            unreachable!("session capture should not request snapshots")
        }
    }

    #[tokio::test]
    async fn successful_session_completion_writes_one_scoped_long_term_memory() {
        let client = Arc::new(RecordingMemoryClient::default());
        let service_client: Arc<dyn macaca_sdk::SystemMemoryClient> = client.clone();
        let app_id = ApplicationId::new();

        capture_successful_session_completion(
            service_client,
            app_id,
            "session-1",
            "coordinator",
            "Summarize the durable behavior",
            TraceContext::new("capture-test"),
        )
        .await;

        let commands = client.commands.lock().await;
        assert_eq!(commands.len(), 1);
        let command = &commands[0];
        assert_eq!(command.scope.identity.application_id, app_id);
        assert_eq!(
            command.scope.identity.session_id.as_deref(),
            Some("session-1")
        );
        assert_eq!(
            command.scope.identity.agent_name.as_deref(),
            Some("coordinator")
        );
        assert_eq!(command.layer, macaca_proto::MemoryLayer::Vector);
        assert_eq!(command.metadata["source"], "chat_session_completion");
        assert!(!command.content.contains("Summarize the durable behavior"));
    }
}
