//! Bounded state for media-audio idempotency and freshness checks.
//!
//! This Memento stores only opaque artifact references; raw media and prompts
//! never enter the ledger.

use std::collections::BTreeMap;

use macaca_proto::{ServiceCommand, ServiceError, ServiceResult};
use tokio::sync::RwLock;

const SIDE_EFFECTING_COMMANDS: &[&str] = &[
    "audio.import_audio_request",
    "audio.transcode_request",
    "audio.segment_request",
    "audio.filter_request",
    "audio.mix_request",
    "audio.synthesis_request",
    "audio.export_request",
];

#[derive(Default)]
pub struct AudioSideEffectLedger {
    completions: RwLock<BTreeMap<String, String>>,
}

impl AudioSideEffectLedger {
    pub fn validate_audio_version(command: &ServiceCommand) -> ServiceResult<()> {
        let expected = command.metadata.get("audio_version");
        let current = command.metadata.get("current_audio_version");
        if expected.is_some() && current.is_some() && expected != current {
            return Err(ServiceError::AdapterFailure("audio_version_stale".into()));
        }
        Ok(())
    }

    pub async fn outcome_ref(&self, command: &ServiceCommand, fallback_ref: String) -> String {
        if !SIDE_EFFECTING_COMMANDS.contains(&command.name.as_str()) {
            return fallback_ref;
        }
        let key = command
            .metadata
            .get("idempotency_key")
            .cloned()
            .unwrap_or_else(|| fallback_ref.clone());
        self.completions
            .write()
            .await
            .entry(key)
            .or_insert(fallback_ref)
            .clone()
    }

    pub async fn clear(&self) {
        self.completions.write().await.clear();
    }
    pub async fn completion_count(&self) -> usize {
        self.completions.read().await.len()
    }
}
