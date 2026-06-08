//! Mutable state helpers for the built-in Skill system service provider.
//!
//! The public provider remains a thin service adapter: it decodes commands,
//! emits trace-friendly logs, and delegates state transitions to focused child
//! modules.  Keeping this Facade module tree small prevents the provider adapter
//! from becoming a hidden curation engine while still allowing future
//! Store/EventLog-backed providers to replace the in-memory implementation.
//!
//! Module layout (Facade + State + Specification vocabulary):
//! - `governance_read_model.rs` — usage recording, snapshots, telemetry, operator mementos
//! - `lifecycle_mutations.rs` — lifecycle State machine transitions and supersede orchestration
//! - `alias_governance.rs` — alias upsert, resolve, and diagnostic snapshots
//! - `proposal_store.rs` — draft experience proposals and append-only event bridge
//! - `event_vocabulary.rs` — deterministic event id generation and snapshot filters

mod alias_governance;
mod event_vocabulary;
mod governance_read_model;
mod lifecycle_mutations;
mod proposal_store;

pub(crate) use event_vocabulary::event_id;

use std::collections::BTreeMap;
use std::path::PathBuf;

use macaca_skill::{
    SkillAutonomousMaterializationRunResult, SkillExperienceProposalRecord,
    SkillGovernanceEventRecord, SkillGovernanceRecord, SkillProposalProcessingRecord,
};
use tokio::sync::Mutex;

/// In-memory governance state for the built-in provider.
///
/// This type is intentionally small and deterministic.  It is not the final
/// persistence layer; it is the local provider's Strategy implementation for
/// tests, development hosts, and future migration to a durable governance store.
#[derive(Default)]
pub(crate) struct SkillProviderGovernanceState {
    pub(crate) records: Mutex<BTreeMap<String, SkillGovernanceRecord>>,
    pub(crate) aliases: Mutex<BTreeMap<String, macaca_skill::SkillAliasRecord>>,
    pub(crate) proposals: Mutex<BTreeMap<String, SkillExperienceProposalRecord>>,
    pub(crate) proposal_processing: Mutex<BTreeMap<String, SkillProposalProcessingRecord>>,
    pub(crate) materialization_operator_runs: Mutex<Vec<SkillAutonomousMaterializationRunResult>>,
    pub(crate) event_log: Mutex<Vec<SkillGovernanceEventRecord>>,
    pub(crate) event_journal_path: Mutex<Option<PathBuf>>,
    pub(crate) event_journal_write_lock: Mutex<()>,
    pub(crate) curation_mementos: Mutex<BTreeMap<String, SkillCurationRollbackMemento>>,
}

/// Local rollback memento captured before an approved curation apply run.
///
/// A future Store/EventLog strategy can persist this same shape as durable
/// artifacts.  The built-in provider keeps it in memory so rollback semantics
/// can be proven without leaking skill bodies or package bytes into reports.
#[derive(Debug, Clone)]
pub(crate) struct SkillCurationRollbackMemento {
    pub(crate) rollback_ref: String,
    pub(crate) before_snapshot_ref: String,
    pub(crate) after_snapshot_ref: Option<String>,
    pub(crate) records: BTreeMap<String, SkillGovernanceRecord>,
    pub(crate) aliases: BTreeMap<String, macaca_skill::SkillAliasRecord>,
    pub(crate) proposals: BTreeMap<String, SkillExperienceProposalRecord>,
    pub(crate) proposal_processing: BTreeMap<String, SkillProposalProcessingRecord>,
    pub(crate) event_log: Vec<SkillGovernanceEventRecord>,
    pub(crate) report_refs: Vec<String>,
    pub(crate) package_memento_refs: Vec<String>,
}
