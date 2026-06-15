//! Scheduler metadata shaping and redaction (**Specification** pattern).
//!
//! Strips sensitive caller metadata keys and projects only scheduler-safe fields
//! into job/target envelopes so prompt material never leaks across service boundaries.

use std::collections::BTreeMap;

use macaca_proto::{ScheduledAgentTaskId, SCHEDULED_AGENT_TASK_SERVICE_ID, SCHEDULER_SERVICE_ID};

pub(super) fn scheduler_target_metadata(
    task_id: &ScheduledAgentTaskId,
    payload_digest: &Option<String>,
    caller_metadata: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let mut metadata = BTreeMap::new();
    metadata.insert("source".into(), SCHEDULED_AGENT_TASK_SERVICE_ID.into());
    metadata.insert("scheduled_agent_task_id".into(), task_id.as_str().into());
    metadata.insert("scheduler_run_source".into(), SCHEDULER_SERVICE_ID.into());
    for (key, value) in sanitize_metadata(caller_metadata.clone()) {
        if key.starts_with("skill.alias.") && !value.trim().is_empty() {
            metadata.insert(key, value);
        }
    }
    if let Some(digest) = payload_digest {
        metadata.insert("payload_digest".into(), digest.clone());
    }
    metadata
}

pub(super) fn scheduler_job_metadata(
    task_id: &ScheduledAgentTaskId,
    payload_digest: &Option<String>,
    caller_metadata: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let mut metadata = sanitize_metadata(caller_metadata.clone());
    metadata.insert("source".into(), SCHEDULED_AGENT_TASK_SERVICE_ID.into());
    metadata.insert("scheduled_agent_task_id".into(), task_id.as_str().into());
    if let Some(digest) = payload_digest {
        metadata.insert("payload_digest".into(), digest.clone());
    }
    metadata
}

pub(super) fn sanitize_metadata(metadata: BTreeMap<String, String>) -> BTreeMap<String, String> {
    metadata
        .into_iter()
        .filter(|(key, _)| {
            let lowered = key.to_ascii_lowercase();
            !lowered.contains("prompt")
                && !lowered.contains("secret")
                && !lowered.contains("token")
                && !lowered.contains("credential")
                && !lowered.contains("private_key")
        })
        .collect()
}
