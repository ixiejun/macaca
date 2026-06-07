//! Unit tests for memory recall injection policies.

use async_trait::async_trait;
use chrono::Utc;
use macaca_context::{ContextOptionsPatch, ContextPreflightRecallConfig, ContextReportBuilder};
use macaca_proto::{ApplicationId, LlmOptions, LlmRole, MemoryEntry, MemoryId, MemoryLayer, TraceContext};
use macaca_sdk::memory::{
    MemoryForgetCommand, MemoryGetCommand, MemoryGetResult, MemoryPrefetchCommand,
    MemoryRecallCommand, MemoryRecallResult, MemoryRememberCommand, MemoryRememberResult,
    MemoryScope, MemoryServiceSnapshot, MemoryServiceSnapshotCommand, MemoryStatusCommand,
    MemoryStatusReport,
};
use std::sync::Arc;

use super::active_recall::apply_active_recall;
use super::preflight_recall::apply_preflight_memory;

struct StaticMemoryClient {
    entries: Vec<MemoryEntry>,
    fail_search: bool,
}

#[async_trait]
impl macaca_sdk::SystemMemoryClient for StaticMemoryClient {
    async fn remember(
        &self,
        _command: MemoryRememberCommand,
    ) -> macaca_proto::MacacaResult<MemoryRememberResult> {
        Ok(MemoryRememberResult {
            id: MemoryId::new(),
            stored_at: Utc::now(),
        })
    }

    async fn recall(
        &self,
        _command: MemoryRecallCommand,
    ) -> macaca_proto::MacacaResult<MemoryRecallResult> {
        if self.fail_search {
            return Err(macaca_proto::MacacaError::Agent("search failed".into()));
        }
        Ok(MemoryRecallResult::new(self.entries.clone()))
    }

    async fn prefetch(
        &self,
        _command: MemoryPrefetchCommand,
    ) -> macaca_proto::MacacaResult<MemoryRecallResult> {
        self.recall(MemoryRecallCommand {
            scope: MemoryScope::project_shared(ApplicationId::new(), "workspace"),
            trace: TraceContext::new("test"),
            query: "test".into(),
            limit: self.entries.len().max(1),
            policy: macaca_sdk::memory::MemoryPolicyHints::default(),
        })
        .await
    }

    async fn get(
        &self,
        command: MemoryGetCommand,
    ) -> macaca_proto::MacacaResult<MemoryGetResult> {
        Ok(MemoryGetResult::new(
            self.entries
                .iter()
                .find(|entry| entry.id == command.id)
                .cloned(),
        ))
    }

    async fn forget(&self, _command: MemoryForgetCommand) -> macaca_proto::MacacaResult<()> {
        Ok(())
    }

    async fn status(
        &self,
        _command: MemoryStatusCommand,
    ) -> macaca_proto::MacacaResult<MemoryStatusReport> {
        Ok(MemoryStatusReport::healthy(
            "test-memory-client",
            macaca_sdk::memory::MemoryCapabilitySet::basic_store_search(),
        ))
    }

    async fn snapshot(
        &self,
        _command: MemoryServiceSnapshotCommand,
    ) -> macaca_proto::MacacaResult<MemoryServiceSnapshot> {
        Ok(MemoryServiceSnapshot::new(
            "test-memory-client",
            true,
            macaca_sdk::memory::MemoryCapabilitySet::basic_store_search(),
            None,
        ))
    }
}

fn entry(content: &str) -> MemoryEntry {
    MemoryEntry {
        id: MemoryId::new(),
        layer: MemoryLayer::Vector,
        content: content.into(),
        metadata: serde_json::Value::Null,
        agent_id: None,
        created_at: Utc::now(),
        expires_at: None,
    }
}

fn assembled() -> macaca_context::ContextAssembleResult {
    macaca_context::ContextAssembleResult {
        messages: vec![
            macaca_proto::LlmMessage::system("sys"),
            macaca_proto::LlmMessage::user("find memory"),
        ],
        options: LlmOptions::default(),
        options_patch: ContextOptionsPatch::default(),
        report: ContextReportBuilder::new("windowed").build(),
    }
}

fn framework_messages() -> Vec<serde_json::Value> {
    vec![serde_json::json!({
        "role": "user",
        "content": [{ "type": "text", "text": "find memory" }]
    })]
}

fn preflight_cfg(enabled: bool) -> ContextPreflightRecallConfig {
    ContextPreflightRecallConfig {
        enabled,
        allowed_tool_names: vec!["memory_search".into()],
        timeout_ms: 50,
        max_chars: 10_000,
        max_tokens: 4_000,
        fatal_on_failure: false,
    }
}

#[tokio::test]
async fn preflight_memory_is_invisible_by_default() {
    let memory_client: Arc<dyn macaca_sdk::SystemMemoryClient> = Arc::new(StaticMemoryClient {
        entries: vec![entry("remembered fact")],
        fail_search: false,
    });
    let mut result = assembled();
    let recall_runtime = macaca_proto::config::ContextRecallRuntimeConfig::default();

    apply_preflight_memory(
        &recall_runtime,
        &memory_client,
        MemoryScope::project_shared(ApplicationId::new(), "workspace"),
        &preflight_cfg(false),
        &mut result,
        &framework_messages(),
    )
    .await;

    assert!(result.report.sources.is_empty());
    assert_eq!(result.messages.len(), 2);
}

#[tokio::test]
async fn preflight_memory_fails_open_with_warning() {
    let memory_client: Arc<dyn macaca_sdk::SystemMemoryClient> = Arc::new(StaticMemoryClient {
        entries: vec![entry("remembered fact")],
        fail_search: true,
    });
    let mut result = assembled();
    let recall_runtime = macaca_proto::config::ContextRecallRuntimeConfig::default();

    apply_preflight_memory(
        &recall_runtime,
        &memory_client,
        MemoryScope::project_shared(ApplicationId::new(), "workspace"),
        &preflight_cfg(true),
        &mut result,
        &framework_messages(),
    )
    .await;

    assert!(result.report.sources.is_empty());
    assert!(result
        .report
        .decisions
        .iter()
        .any(|d| d.code == "preflight_recall_degraded"));
}

#[tokio::test]
async fn legacy_active_recall_reports_request_only_metadata() {
    let memory_client: Arc<dyn macaca_sdk::SystemMemoryClient> = Arc::new(StaticMemoryClient {
        entries: vec![entry("remembered fact")],
        fail_search: false,
    });
    let mut result = assembled();
    let recall_runtime = macaca_proto::config::ContextRecallRuntimeConfig::default();

    apply_active_recall(
        &recall_runtime,
        &memory_client,
        MemoryScope::project_shared(ApplicationId::new(), "workspace"),
        &preflight_cfg(true),
        false,
        &mut result,
        &framework_messages(),
    )
    .await;

    let row = result.report.active_recall[0]
        .source_breakdown
        .first()
        .unwrap();
    assert_eq!(
        row.provenance_provider_id.as_deref(),
        Some("workspace-memory")
    );
    assert_eq!(row.privacy_tier.as_deref(), Some("workspace"));
    assert_eq!(row.request_only, Some(true));
    assert_eq!(row.trust_level.as_deref(), Some("untrusted"));
    assert!(result
        .messages
        .iter()
        .any(|m| m.role == LlmRole::System && m.content.contains("reference only")));
}
