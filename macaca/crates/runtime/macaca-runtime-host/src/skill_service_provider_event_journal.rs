//! Local durable event journal for the built-in Skill governance provider.
//!
//! The Skill service already expresses governance mutations as sanitized
//! `SkillGovernanceEventRecord` values.  This module gives the local provider a
//! small Memento implementation: append those sanitized events to JSONL and
//! replay them on startup.  The journal is deliberately provider-local and
//! replaceable, so a future Store/EventLog Strategy can swap the persistence
//! backend without changing SDK, Web, CLI, or application contracts.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use macaca_skill::{
    SkillAliasRecord, SkillExperienceProposalRecord, SkillGovernanceEventRecord,
    SkillGovernanceReadModel, SkillGovernanceRecord,
};
use tokio::fs::{self, OpenOptions};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::skill_service_provider_state::SkillProviderGovernanceState;

const MAX_REPLAY_EVENTS: usize = 50_000;
const MAX_JOURNAL_BYTES: u64 = 32 * 1024 * 1024;

impl SkillProviderGovernanceState {
    /// Configure the optional local governance event journal.
    ///
    /// Composition roots call this before service startup.  The path is generic
    /// workspace/service infrastructure, not application semantics.  A missing
    /// path disables durable local replay and leaves the provider's existing
    /// in-memory behavior unchanged.
    pub(crate) async fn configure_governance_event_journal_path(&self, path: Option<PathBuf>) {
        let mut slot = self.event_journal_path.lock().await;
        *slot = path;
    }

    /// Replay the configured local governance event journal into live state.
    ///
    /// Replay runs before package recovery.  That ordering lets durable usage
    /// counters win when they exist, while materialized package recovery still
    /// fills identity gaps for packages that predate the journal.
    pub(crate) async fn replay_governance_event_journal(&self) -> usize {
        let Some(path) = self.event_journal_path.lock().await.clone() else {
            tracing::info!(
                "skill governance event journal replay skipped because no journal path is configured"
            );
            return 0;
        };
        let events = load_journal_events(&path).await;
        let replayed = events.len();
        if replayed == 0 {
            tracing::info!(
                path = %path.display(),
                "skill governance event journal replay found no durable events"
            );
            return 0;
        }

        let read_model = SkillGovernanceReadModel::from_events(events.clone());
        self.replace_live_state_from_read_model(read_model, events)
            .await;
        tracing::info!(
            path = %path.display(),
            replayed_events = replayed,
            "skill governance event journal replay restored local provider state"
        );
        replayed
    }

    /// Append one sanitized governance event to the configured local journal.
    ///
    /// The caller has already passed through the `SkillGovernanceEventRecord`
    /// constructor, which strips unsafe metadata.  This method persists exactly
    /// that bounded DTO as JSONL and never serializes raw prompts, provider
    /// payloads, package bytes, or full Skill bodies.
    pub(crate) async fn persist_governance_event_journal(
        &self,
        event: &SkillGovernanceEventRecord,
    ) -> Result<(), String> {
        let Some(path) = self.event_journal_path.lock().await.clone() else {
            return Ok(());
        };
        let _guard = self.event_journal_write_lock.lock().await;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|error| format!("create journal parent failed: {error}"))?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .await
            .map_err(|error| format!("open journal failed: {error}"))?;
        let line = serde_json::to_string(event)
            .map_err(|error| format!("serialize governance event failed: {error}"))?;
        file.write_all(line.as_bytes())
            .await
            .map_err(|error| format!("write governance event failed: {error}"))?;
        file.write_all(b"\n")
            .await
            .map_err(|error| format!("write governance event newline failed: {error}"))?;
        file.flush()
            .await
            .map_err(|error| format!("flush governance event failed: {error}"))?;
        tracing::info!(
            event_id = %event.event_id,
            trace_id = %event.trace_id,
            path = %path.display(),
            "skill governance event persisted to local durable journal"
        );
        Ok(())
    }

    async fn replace_live_state_from_read_model(
        &self,
        read_model: SkillGovernanceReadModel,
        events: Vec<SkillGovernanceEventRecord>,
    ) {
        *self.records.lock().await = keyed_records(read_model.records);
        *self.aliases.lock().await = keyed_aliases(read_model.aliases);
        *self.proposals.lock().await = keyed_proposals(read_model.proposals);
        *self.event_log.lock().await = events;
    }
}

async fn load_journal_events(path: &Path) -> Vec<SkillGovernanceEventRecord> {
    let Ok(metadata) = fs::metadata(path).await else {
        return Vec::new();
    };
    if !metadata.is_file() {
        tracing::warn!(
            path = %path.display(),
            "skill governance event journal replay skipped non-file path"
        );
        return Vec::new();
    }
    if metadata.len() > MAX_JOURNAL_BYTES {
        tracing::warn!(
            path = %path.display(),
            bytes = metadata.len(),
            max_bytes = MAX_JOURNAL_BYTES,
            "skill governance event journal replay skipped oversized journal"
        );
        return Vec::new();
    }

    let Ok(file) = fs::File::open(path).await else {
        tracing::warn!(
            path = %path.display(),
            "skill governance event journal replay could not open journal"
        );
        return Vec::new();
    };
    let mut reader = BufReader::new(file).lines();
    let mut events = Vec::new();
    let mut skipped = 0usize;
    while let Ok(Some(line)) = reader.next_line().await {
        if events.len() >= MAX_REPLAY_EVENTS {
            tracing::warn!(
                path = %path.display(),
                max_events = MAX_REPLAY_EVENTS,
                "skill governance event journal replay stopped at bounded event limit"
            );
            break;
        }
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<SkillGovernanceEventRecord>(&line) {
            Ok(event) => events.push(event),
            Err(error) => {
                skipped = skipped.saturating_add(1);
                tracing::warn!(
                    path = %path.display(),
                    error = %error,
                    "skill governance event journal replay skipped malformed line"
                );
            }
        }
    }
    tracing::info!(
        path = %path.display(),
        loaded_events = events.len(),
        skipped_events = skipped,
        "skill governance event journal replay loaded durable events"
    );
    events
}

fn keyed_records(records: Vec<SkillGovernanceRecord>) -> BTreeMap<String, SkillGovernanceRecord> {
    records
        .into_iter()
        .map(|record| (record.provenance.skill_id.clone(), record))
        .collect()
}

fn keyed_aliases(aliases: Vec<SkillAliasRecord>) -> BTreeMap<String, SkillAliasRecord> {
    aliases
        .into_iter()
        .map(|alias| (alias.key(), alias))
        .collect()
}

fn keyed_proposals(
    proposals: Vec<SkillExperienceProposalRecord>,
) -> BTreeMap<String, SkillExperienceProposalRecord> {
    proposals
        .into_iter()
        .map(|proposal| (proposal.proposal_id.clone(), proposal))
        .collect()
}
