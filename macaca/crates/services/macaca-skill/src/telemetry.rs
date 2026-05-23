//! Sanitized Skill usage telemetry contracts.
//!
//! Telemetry is intentionally metadata-only.  Providers record bounded counters
//! and timestamps so curation can reason about effectiveness without storing
//! raw prompts, task outputs, provider payloads, or full skill instruction
//! bodies in governance snapshots.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::governance::SkillAuthorKind;

/// Event types accepted by the usage telemetry command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillUsageEventKind {
    Created,
    Viewed,
    Activated,
    Used,
    ResourceRead,
    Patched,
    SuccessfulTask,
    FailedTask,
    Archived,
    Restored,
    Quarantined,
    QuarantineReleased,
    Superseded,
    Rejected,
    Pinned,
    Unpinned,
}

/// Sanitized observation emitted by framework tools or service providers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillUsageObservation {
    pub skill_id: String,
    pub name: String,
    pub source: String,
    pub source_scope: String,
    pub event: SkillUsageEventKind,
    pub author_kind: SkillAuthorKind,
    pub created_by: Option<String>,
    pub pinned: Option<bool>,
    pub evidence_id: Option<String>,
    pub metadata: BTreeMap<String, String>,
}

impl SkillUsageObservation {
    /// Stable key used by providers that have not introduced a separate id.
    pub fn key(&self) -> String {
        if self.skill_id.trim().is_empty() {
            self.name.trim().to_string()
        } else {
            self.skill_id.trim().to_string()
        }
    }
}

/// Aggregated, prompt-safe telemetry for one governed skill.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillUsageTelemetry {
    pub view_count: u64,
    pub activation_count: u64,
    pub resource_read_count: u64,
    pub use_count: u64,
    pub patch_count: u64,
    pub successful_task_count: u64,
    pub failed_task_count: u64,
    pub last_viewed_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub last_resource_read_at: Option<DateTime<Utc>>,
    pub last_patched_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub last_successful_task_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub last_failed_task_at: Option<DateTime<Utc>>,
    pub last_lifecycle_event_at: Option<DateTime<Utc>>,
    pub last_observed_at: Option<DateTime<Utc>>,
}

impl Default for SkillUsageTelemetry {
    fn default() -> Self {
        Self {
            view_count: 0,
            activation_count: 0,
            resource_read_count: 0,
            use_count: 0,
            patch_count: 0,
            successful_task_count: 0,
            failed_task_count: 0,
            last_viewed_at: None,
            last_used_at: None,
            last_resource_read_at: None,
            last_patched_at: None,
            last_successful_task_at: None,
            last_failed_task_at: None,
            last_lifecycle_event_at: None,
            last_observed_at: None,
        }
    }
}

impl SkillUsageTelemetry {
    /// Apply one sanitized event to aggregate counters.
    pub fn record(&mut self, event: &SkillUsageEventKind, observed_at: DateTime<Utc>) {
        self.last_observed_at = Some(observed_at);
        match event {
            SkillUsageEventKind::Viewed => {
                self.view_count = self.view_count.saturating_add(1);
                self.last_viewed_at = Some(observed_at);
            }
            SkillUsageEventKind::Activated | SkillUsageEventKind::Used => {
                self.activation_count = self.activation_count.saturating_add(1);
                self.use_count = self.use_count.saturating_add(1);
                self.last_used_at = Some(observed_at);
            }
            SkillUsageEventKind::ResourceRead => {
                self.resource_read_count = self.resource_read_count.saturating_add(1);
                self.last_resource_read_at = Some(observed_at);
            }
            SkillUsageEventKind::Patched => {
                self.patch_count = self.patch_count.saturating_add(1);
                self.last_patched_at = Some(observed_at);
            }
            SkillUsageEventKind::SuccessfulTask => {
                self.successful_task_count = self.successful_task_count.saturating_add(1);
                self.last_successful_task_at = Some(observed_at);
            }
            SkillUsageEventKind::FailedTask => {
                self.failed_task_count = self.failed_task_count.saturating_add(1);
                self.last_failed_task_at = Some(observed_at);
            }
            SkillUsageEventKind::Created
            | SkillUsageEventKind::Archived
            | SkillUsageEventKind::Restored
            | SkillUsageEventKind::Quarantined
            | SkillUsageEventKind::QuarantineReleased
            | SkillUsageEventKind::Superseded
            | SkillUsageEventKind::Rejected
            | SkillUsageEventKind::Pinned
            | SkillUsageEventKind::Unpinned => {
                self.last_lifecycle_event_at = Some(observed_at);
            }
        }
    }

    /// Return the latest usage-like timestamp for deterministic curation.
    pub fn last_activity_at(&self) -> Option<DateTime<Utc>> {
        [
            self.last_used_at,
            self.last_viewed_at,
            self.last_resource_read_at,
            self.last_patched_at,
            self.last_successful_task_at,
            self.last_failed_task_at,
        ]
        .into_iter()
        .flatten()
        .max()
    }
}
