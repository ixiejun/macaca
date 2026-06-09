//! Shared post-assembly **Template Method** step for context reporting.
//!
//! Both the Context Service path and the legacy local assembler converge here:
//! scoped memory recall injection, framework message conversion, and report
//! persistence to the session event log.

use macaca_framework::model::ChatOptions;

use crate::context_memory_injection::{apply_active_recall, apply_preflight_memory};
use crate::context_message_codec::{llm_messages_to_framework, llm_options_to_framework};
use crate::context_report_events::persist_context_report_events;
use crate::context_reporting_memory::{digest_scope, recall_scope};
use crate::source_artifact::persist_pruned_source_artifacts;

use super::ContextReportingChatModel;

impl ContextReportingChatModel {
    /// Apply scoped recall injectors, persist reports, and return framework DTOs.
    pub(super) async fn finalize_assembled_context(
        &self,
        session_id: &str,
        model: &str,
        incoming_messages: &[serde_json::Value],
        options: &ChatOptions,
        assembled: &mut macaca_sdk::context::ContextAssembleResult,
        lineage_count: u32,
    ) -> (Vec<serde_json::Value>, ChatOptions) {
        assembled.report.lineage_compaction_count = lineage_count;
        let message_count_before_recall = assembled.messages.len();
        let composer_active_recall_proven = self.composer_handles_active_vector_recall()
            && !assembled.report.active_recall.is_empty();
        let preflight_cfg = self.preflight_config();
        let active_recall_cfg = Self::active_recall_fallback_config(
            &self.active_vector_memory,
            &preflight_cfg,
            composer_active_recall_proven,
        );
        apply_active_recall(
            &self.recall_runtime,
            &self.memory_client,
            recall_scope(self.app_id, self.session_id.as_deref(), None),
            &active_recall_cfg,
            composer_active_recall_proven,
            assembled,
            incoming_messages,
        )
        .await;
        apply_preflight_memory(
            &self.recall_runtime,
            &self.memory_client,
            digest_scope(self.app_id, self.session_id.as_deref()),
            &preflight_cfg,
            assembled,
            incoming_messages,
        )
        .await;
        let recall_injected_messages = assembled.messages.len() != message_count_before_recall;
        let output_messages =
            if assembled.report.engine_id == "legacy" && !recall_injected_messages {
                incoming_messages.to_vec()
            } else {
                llm_messages_to_framework(&assembled.messages)
            };
        let output_options = llm_options_to_framework(&assembled.options, options);
        let mut report = assembled.report.clone();
        persist_pruned_source_artifacts(
            &self.event_log,
            session_id,
            &self.agent_name,
            Some(self.app_id.to_string()),
            &mut report,
            incoming_messages,
        )
        .await;
        persist_context_report_events(
            &self.event_log,
            session_id,
            &self.agent_name,
            self.app_id,
            model,
            &report,
        )
        .await;
        tracing::info!(
            app_id = %self.app_id,
            session_id,
            agent = %self.agent_name,
            command = "context.report.persist",
            engine_id = %assembled.report.engine_id,
            recall_injected = recall_injected_messages,
            "context assembly finalized with scoped recall and report persistence"
        );
        (output_messages, output_options)
    }
}
