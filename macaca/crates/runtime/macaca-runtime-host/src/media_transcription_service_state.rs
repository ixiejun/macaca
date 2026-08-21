//! Bounded runtime state for metadata-only transcription side-effect safety.
//!
//! The ledger holds opaque outcome references only. It never receives source,
//! chunk, transcript, subtitle, or provider payload data.

use std::collections::BTreeMap;

use macaca_proto::{ServiceCommand, ServiceError, ServiceResult};
use tokio::sync::RwLock;

/// Commands that can create, mutate, publish, or cancel provider-owned work.
const SIDE_EFFECTING_COMMANDS: &[&str] = &[
    "transcription.import_source_request",
    "transcription.batch_request",
    "transcription.start_stream",
    "transcription.append_stream_chunk",
    "transcription.finish_stream",
    "transcription.cancel_stream",
    "transcription.diarization_request",
    "transcription.redaction_request",
    "transcription.subtitle_export_request",
    "transcription.translation_handoff_request",
];

/// Memento-friendly ledger that deduplicates opaque references by request key.
#[derive(Default)]
pub struct TranscriptionSideEffectLedger {
    completions: RwLock<BTreeMap<String, String>>,
}

impl TranscriptionSideEffectLedger {
    /// Reject an explicitly stale host-issued source version before work starts.
    pub fn validate_source_version(command: &ServiceCommand) -> ServiceResult<()> {
        let expected = command.metadata.get("source_version");
        let current = command.metadata.get("current_source_version");
        if expected.is_some() && current.is_some() && expected != current {
            return Err(ServiceError::AdapterFailure(
                "transcription_source_version_stale".into(),
            ));
        }
        Ok(())
    }

    /// Return a stable opaque outcome reference for side effects with the same key.
    pub async fn outcome_ref(&self, command: &ServiceCommand, fallback_ref: String) -> String {
        if !SIDE_EFFECTING_COMMANDS.contains(&command.name.as_str()) {
            return fallback_ref;
        }
        let key = command
            .metadata
            .get("idempotency_key")
            .cloned()
            .unwrap_or_else(|| fallback_ref.clone());
        let mut completions = self.completions.write().await;
        completions.entry(key).or_insert(fallback_ref).clone()
    }

    /// Discard ephemeral ledger state when the provider is cleaned up or restarted.
    pub async fn clear(&self) {
        self.completions.write().await.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use macaca_proto::{ServiceCommandName, TraceContext};

    fn command(name: &str) -> ServiceCommand {
        ServiceCommand::with_trace(
            ServiceCommandName::new(name),
            serde_json::json!({}),
            TraceContext::new("transcription-ledger"),
        )
    }

    #[tokio::test]
    async fn ledger_reuses_side_effect_outcome_references() {
        let ledger = TranscriptionSideEffectLedger::default();
        let mut first = command("transcription.batch_request");
        first
            .metadata
            .insert("idempotency_key".into(), "once".into());
        let mut replay = command("transcription.batch_request");
        replay
            .metadata
            .insert("idempotency_key".into(), "once".into());
        assert_eq!(
            ledger.outcome_ref(&first, "artifact:first".into()).await,
            ledger.outcome_ref(&replay, "artifact:second".into()).await
        );
    }

    #[test]
    fn stale_source_versions_fail_before_provider_work() {
        let mut command = command("transcription.batch_request");
        command
            .metadata
            .insert("source_version".into(), "v1".into());
        command
            .metadata
            .insert("current_source_version".into(), "v2".into());
        assert!(TranscriptionSideEffectLedger::validate_source_version(&command).is_err());
    }
}
