//! Provider-local payload memento store.
//!
//! Scheduler jobs must never contain raw prompts, but Runtime Host needs the
//! prompt when a due run becomes an `AgentExecutionCommand`.  This module is the
//! local Memento implementation for that split.  It stores prompt material only
//! inside the Scheduled Agent Task provider and exposes safe references,
//! digests, and redacted summaries to other services.

use std::collections::hash_map::DefaultHasher;
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

use chrono::{DateTime, Utc};
use macaca_proto::{AutonomyPayloadRef, MacacaResult};

/// Owned payload material. This type is deliberately crate-local and must not
/// be embedded in Scheduler DTOs, summaries, logs, or shell responses.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StoredScheduledAgentPayload {
    pub(crate) prompt: String,
    pub(crate) delegated_context: serde_json::Value,
    pub(crate) digest: String,
    pub(crate) redacted_summary: String,
    pub(crate) created_at: DateTime<Utc>,
}

/// In-memory payload repository for the local provider.
#[derive(Debug, Default)]
pub(crate) struct InMemoryPayloadStore {
    payloads: BTreeMap<String, StoredScheduledAgentPayload>,
}

impl InMemoryPayloadStore {
    /// Persist prompt material and return a safe payload reference.
    pub(crate) fn insert(
        &mut self,
        task_id: &str,
        prompt: String,
        delegated_context: serde_json::Value,
        safe_name: Option<&str>,
    ) -> MacacaResult<AutonomyPayloadRef> {
        let digest = payload_digest(&prompt, &delegated_context);
        let redacted_summary = safe_name
            .filter(|name| !name.trim().is_empty())
            .map(|name| name.trim().to_string())
            .unwrap_or_else(|| "scheduled agent task prompt redacted".into());
        let reference = format!("scheduled-agent-task://payload/{task_id}");
        let payload = StoredScheduledAgentPayload {
            prompt,
            delegated_context,
            digest: digest.clone(),
            redacted_summary: redacted_summary.clone(),
            created_at: Utc::now(),
        };
        self.payloads.insert(reference.clone(), payload);
        let mut payload_ref = AutonomyPayloadRef::new(reference, redacted_summary)?;
        payload_ref.content_digest = Some(digest);
        payload_ref.metadata.insert(
            "owner_service".into(),
            macaca_proto::SCHEDULED_AGENT_TASK_SERVICE_ID.into(),
        );
        Ok(payload_ref)
    }

    /// Resolve a payload reference for Runtime Host dispatch.
    pub(crate) fn get(&self, reference: &str) -> Option<&StoredScheduledAgentPayload> {
        self.payloads.get(reference)
    }

    /// Remove a payload, returning it if present.
    ///
    /// Used to roll back a payload that was inserted while preparing a task whose
    /// downstream scheduler registration then failed (2026-07-08 audit S15), so a
    /// sensitive prompt does not linger for a task that was never registered.
    pub(crate) fn remove(&mut self, reference: &str) -> Option<StoredScheduledAgentPayload> {
        self.payloads.remove(reference)
    }

    /// Return current payload count for health/snapshot diagnostics.
    pub(crate) fn len(&self) -> usize {
        self.payloads.len()
    }
}

/// Build a deterministic local digest without adding a new dependency.
///
/// This is not a cryptographic commitment.  It is a bounded correlation token
/// for local audit mementos and tests.  A future persistent provider can replace
/// it with a stronger hash while preserving the DTO field contract.
fn payload_digest(prompt: &str, delegated_context: &serde_json::Value) -> String {
    let mut hasher = DefaultHasher::new();
    prompt.hash(&mut hasher);
    delegated_context.to_string().hash(&mut hasher);
    format!("digest.scheduled_agent_task.{:016x}", hasher.finish())
}
