//! Replaceable Store/EventLog Strategy for autonomy evolution governance.
//!
//! The local JSONL implementation in this module is a development provider. It
//! proves replay, migration, compaction, and sanitization semantics while all
//! callers depend on the `EvolutionGovernanceLedger` trait. A production
//! Store/EventLog backend can replace this provider without changing Skill,
//! Autonomy, Web, CLI, or SDK semantics.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use chrono::{DateTime, Utc};
use macaca_proto::{MacacaError, MacacaResult, TraceContext};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::{EvolutionScope, EvolutionTargetType, AUTONOMY_EVOLUTION_SERVICE_ID};

const CURRENT_SCHEMA_VERSION: u32 = 1;
const MAX_REF_LEN: usize = 160;
const MAX_REFS: usize = 8;
const MAX_REPLAY_LIMIT: usize = 10_000;

/// Provider-neutral record categories written by the governance ledger.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvolutionGovernanceLedgerRecordKind {
    Transition,
    Admission,
    Benchmark,
    Release,
    Snapshot,
    Other(String),
}

/// Sanitized append command accepted by ledger providers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvolutionGovernanceAppendCommand {
    pub record_id: String,
    pub run_id: String,
    pub node_id: String,
    pub kind: EvolutionGovernanceLedgerRecordKind,
    pub target_type: Option<EvolutionTargetType>,
    pub trace: TraceContext,
    pub scope: EvolutionScope,
    pub record_ref: String,
    pub evidence_refs: Vec<String>,
    pub policy_decision_refs: Vec<String>,
    pub audit_refs: Vec<String>,
    pub rollback_refs: Vec<String>,
}

/// Versioned, body-free ledger record safe for replay and snapshots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvolutionGovernanceLedgerRecord {
    pub schema_version: u32,
    pub sequence: u64,
    pub record_id: String,
    pub run_id: String,
    pub node_id: String,
    pub kind: EvolutionGovernanceLedgerRecordKind,
    pub target_type: Option<EvolutionTargetType>,
    pub trace_id: String,
    pub scope: EvolutionScope,
    pub record_ref: String,
    pub evidence_refs: Vec<String>,
    pub policy_decision_refs: Vec<String>,
    pub audit_refs: Vec<String>,
    pub rollback_refs: Vec<String>,
    pub captured_at: DateTime<Utc>,
}

/// Replay query with scope and cursor filters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvolutionGovernanceReplayCommand {
    pub scope: EvolutionScope,
    pub run_id: Option<String>,
    pub after_sequence: Option<u64>,
    pub limit: usize,
}

/// Replay result plus bounded diagnostics for skipped or migrated rows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvolutionGovernanceReplayResult {
    pub records: Vec<EvolutionGovernanceLedgerRecord>,
    pub next_after_sequence: Option<u64>,
    pub diagnostics: Vec<String>,
}

/// Bounded snapshot that summarizes ledger health without raw payloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvolutionGovernanceLedgerSnapshot {
    pub provider: String,
    pub record_count: usize,
    pub last_sequence: Option<u64>,
    pub schema_version: u32,
    pub diagnostics: Vec<String>,
    pub captured_at: DateTime<Utc>,
}

/// Result returned after a compaction pass rewrites retained records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvolutionGovernanceCompactionResult {
    pub retained_records: usize,
    pub removed_records: usize,
    pub last_sequence: Option<u64>,
    pub diagnostics: Vec<String>,
    pub captured_at: DateTime<Utc>,
}

/// Replaceable governance ledger Strategy.
pub trait EvolutionGovernanceLedger: Send + Sync {
    fn append(
        &self,
        command: EvolutionGovernanceAppendCommand,
    ) -> MacacaResult<EvolutionGovernanceLedgerRecord>;

    fn replay(
        &self,
        command: EvolutionGovernanceReplayCommand,
    ) -> MacacaResult<EvolutionGovernanceReplayResult>;

    fn compact(&self, retain_latest: usize) -> MacacaResult<EvolutionGovernanceCompactionResult>;

    fn snapshot(&self) -> MacacaResult<EvolutionGovernanceLedgerSnapshot>;
}

/// Local JSONL development provider for the governance ledger Strategy.
#[derive(Debug)]
pub struct LocalJsonlEvolutionGovernanceLedger {
    path: PathBuf,
    lock: Mutex<()>,
}

impl LocalJsonlEvolutionGovernanceLedger {
    /// Open or create a local development ledger file.
    pub fn open(path: impl Into<PathBuf>) -> MacacaResult<Self> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(io_error)?;
        }
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(io_error)?;
        info!(
            service_id = AUTONOMY_EVOLUTION_SERVICE_ID,
            ledger_path = %path.display(),
            "opened local evolution governance ledger development provider"
        );
        Ok(Self {
            path,
            lock: Mutex::new(()),
        })
    }

    fn read_all_locked(&self) -> MacacaResult<ReadOutcome> {
        read_records(&self.path)
    }
}

impl EvolutionGovernanceLedger for LocalJsonlEvolutionGovernanceLedger {
    fn append(
        &self,
        command: EvolutionGovernanceAppendCommand,
    ) -> MacacaResult<EvolutionGovernanceLedgerRecord> {
        validate_append(&command)?;
        let _guard = self.lock.lock().expect("governance ledger mutex poisoned");
        let outcome = self.read_all_locked()?;
        let next_sequence = outcome
            .records
            .iter()
            .map(|record| record.sequence)
            .max()
            .unwrap_or(0)
            + 1;
        let record = sanitized_record(command, next_sequence);
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(io_error)?;
        let line = serde_json::to_string(&record).map_err(json_error)?;
        writeln!(file, "{line}").map_err(io_error)?;
        info!(
            service_id = AUTONOMY_EVOLUTION_SERVICE_ID,
            sequence = record.sequence,
            run_id = record.run_id.as_str(),
            trace_id = record.trace_id.as_str(),
            "appended sanitized evolution governance ledger record"
        );
        Ok(record)
    }

    fn replay(
        &self,
        command: EvolutionGovernanceReplayCommand,
    ) -> MacacaResult<EvolutionGovernanceReplayResult> {
        let _guard = self.lock.lock().expect("governance ledger mutex poisoned");
        let outcome = self.read_all_locked()?;
        let limit = command.limit.min(MAX_REPLAY_LIMIT);
        let mut records = outcome
            .records
            .into_iter()
            .filter(|record| {
                command
                    .run_id
                    .as_ref()
                    .map(|run_id| &record.run_id == run_id)
                    .unwrap_or(true)
                    && command
                        .after_sequence
                        .map(|sequence| record.sequence > sequence)
                        .unwrap_or(true)
                    && crate::scope_matches_filter(&record.scope, &command.scope)
            })
            .collect::<Vec<_>>();
        records.sort_by_key(|record| record.sequence);
        records.truncate(limit);
        let next_after_sequence = records.last().map(|record| record.sequence);
        info!(
            service_id = AUTONOMY_EVOLUTION_SERVICE_ID,
            replayed_records = records.len(),
            "replayed sanitized evolution governance ledger records"
        );
        Ok(EvolutionGovernanceReplayResult {
            records,
            next_after_sequence,
            diagnostics: outcome.diagnostics,
        })
    }

    fn compact(&self, retain_latest: usize) -> MacacaResult<EvolutionGovernanceCompactionResult> {
        let _guard = self.lock.lock().expect("governance ledger mutex poisoned");
        let outcome = self.read_all_locked()?;
        let original_count = outcome.records.len();
        let start = original_count.saturating_sub(retain_latest);
        let retained = outcome.records[start..].to_vec();
        let mut file = File::create(&self.path).map_err(io_error)?;
        for record in &retained {
            let line = serde_json::to_string(record).map_err(json_error)?;
            writeln!(file, "{line}").map_err(io_error)?;
        }
        info!(
            service_id = AUTONOMY_EVOLUTION_SERVICE_ID,
            retained_records = retained.len(),
            removed_records = original_count.saturating_sub(retained.len()),
            "compacted local evolution governance ledger development provider"
        );
        Ok(EvolutionGovernanceCompactionResult {
            retained_records: retained.len(),
            removed_records: original_count.saturating_sub(retained.len()),
            last_sequence: retained.last().map(|record| record.sequence),
            diagnostics: outcome.diagnostics,
            captured_at: Utc::now(),
        })
    }

    fn snapshot(&self) -> MacacaResult<EvolutionGovernanceLedgerSnapshot> {
        let _guard = self.lock.lock().expect("governance ledger mutex poisoned");
        let outcome = self.read_all_locked()?;
        Ok(EvolutionGovernanceLedgerSnapshot {
            provider: "local-jsonl-development".into(),
            record_count: outcome.records.len(),
            last_sequence: outcome.records.last().map(|record| record.sequence),
            schema_version: CURRENT_SCHEMA_VERSION,
            diagnostics: outcome.diagnostics,
            captured_at: Utc::now(),
        })
    }
}

#[derive(Debug, Default)]
struct ReadOutcome {
    records: Vec<EvolutionGovernanceLedgerRecord>,
    diagnostics: Vec<String>,
}

fn validate_append(command: &EvolutionGovernanceAppendCommand) -> MacacaResult<()> {
    if command.record_id.trim().is_empty() {
        return Err(MacacaError::Config(
            "evolution governance ledger append requires record_id".into(),
        ));
    }
    if command.run_id.trim().is_empty() {
        return Err(MacacaError::Config(
            "evolution governance ledger append requires run_id".into(),
        ));
    }
    if command.node_id.trim().is_empty() {
        return Err(MacacaError::Config(
            "evolution governance ledger append requires node_id".into(),
        ));
    }
    if command.trace.trace_id.trim().is_empty() {
        return Err(MacacaError::Config(
            "evolution governance ledger append requires trace_id".into(),
        ));
    }
    Ok(())
}

fn sanitized_record(
    command: EvolutionGovernanceAppendCommand,
    sequence: u64,
) -> EvolutionGovernanceLedgerRecord {
    EvolutionGovernanceLedgerRecord {
        schema_version: CURRENT_SCHEMA_VERSION,
        sequence,
        record_id: sanitize_ref(&command.record_id),
        run_id: sanitize_ref(&command.run_id),
        node_id: sanitize_ref(&command.node_id),
        kind: command.kind,
        target_type: command.target_type,
        trace_id: sanitize_ref(&command.trace.trace_id),
        scope: command.scope,
        record_ref: sanitize_ref(&command.record_ref),
        evidence_refs: sanitize_refs(&command.evidence_refs),
        policy_decision_refs: sanitize_refs(&command.policy_decision_refs),
        audit_refs: sanitize_refs(&command.audit_refs),
        rollback_refs: sanitize_refs(&command.rollback_refs),
        captured_at: Utc::now(),
    }
}

fn read_records(path: &Path) -> MacacaResult<ReadOutcome> {
    let file = File::open(path).map_err(io_error)?;
    let mut outcome = ReadOutcome::default();
    for line in BufReader::new(file).lines() {
        let line = line.map_err(io_error)?;
        if line.trim().is_empty() {
            continue;
        }
        match parse_record_line(&line) {
            Ok((record, diagnostics)) => {
                outcome.diagnostics.extend(diagnostics);
                outcome.records.push(record);
            }
            Err(diagnostic) => {
                warn!(
                    service_id = AUTONOMY_EVOLUTION_SERVICE_ID,
                    diagnostic, "skipped malformed evolution governance ledger record"
                );
                outcome.diagnostics.push(diagnostic.into());
            }
        }
    }
    outcome.records.sort_by_key(|record| record.sequence);
    Ok(outcome)
}

fn parse_record_line(
    line: &str,
) -> Result<(EvolutionGovernanceLedgerRecord, Vec<String>), &'static str> {
    let value: serde_json::Value =
        serde_json::from_str(line).map_err(|_| "malformed_record_skipped")?;
    let schema_version = value
        .get("schema_version")
        .and_then(serde_json::Value::as_u64)
        .ok_or("malformed_record_skipped")? as u32;
    match schema_version {
        CURRENT_SCHEMA_VERSION => {
            let mut record: EvolutionGovernanceLedgerRecord =
                serde_json::from_value(value).map_err(|_| "malformed_record_skipped")?;
            sanitize_existing_record(&mut record);
            Ok((record, Vec::new()))
        }
        0 => migrate_v0(value),
        _ => Err("unsupported_schema_version_skipped"),
    }
}

fn migrate_v0(
    mut value: serde_json::Value,
) -> Result<(EvolutionGovernanceLedgerRecord, Vec<String>), &'static str> {
    if let Some(kind) = value.get("kind").and_then(serde_json::Value::as_str) {
        let migrated = match kind {
            "transition" => "Transition",
            "admission" => "Admission",
            "benchmark" => "Benchmark",
            "release" => "Release",
            "snapshot" => "Snapshot",
            _ => kind,
        };
        value["kind"] = serde_json::Value::String(migrated.to_owned());
    }
    let mut record: EvolutionGovernanceLedgerRecord =
        serde_json::from_value(value).map_err(|_| "malformed_record_skipped")?;
    record.schema_version = CURRENT_SCHEMA_VERSION;
    sanitize_existing_record(&mut record);
    Ok((record, vec!["schema_v0_record_migrated".into()]))
}

fn sanitize_existing_record(record: &mut EvolutionGovernanceLedgerRecord) {
    record.record_id = sanitize_ref(&record.record_id);
    record.run_id = sanitize_ref(&record.run_id);
    record.node_id = sanitize_ref(&record.node_id);
    record.trace_id = sanitize_ref(&record.trace_id);
    record.record_ref = sanitize_ref(&record.record_ref);
    record.evidence_refs = sanitize_refs(&record.evidence_refs);
    record.policy_decision_refs = sanitize_refs(&record.policy_decision_refs);
    record.audit_refs = sanitize_refs(&record.audit_refs);
    record.rollback_refs = sanitize_refs(&record.rollback_refs);
}

fn sanitize_refs(values: &[String]) -> Vec<String> {
    values
        .iter()
        .take(MAX_REFS)
        .map(|value| sanitize_ref(value))
        .collect()
}

/// Sanitize a single ledger reference before it enters an audit snapshot.
///
/// Hardened per the 2026-07-08 audit (P0-5). The previous implementation relied
/// solely on a keyword deny-list, which let a real secret *value* (e.g. an
/// `sk-…` key or a bearer token) pass unchanged whenever it did not literally
/// contain the words "secret"/"credential". It also used `String::truncate`,
/// which panics when the byte length lands inside a multi-byte character.
///
/// This now (1) applies the structural, value-shape secret masking from the
/// foundation `text_sanitize` module so credential-shaped refs are fully
/// redacted, (2) keeps the keyword replacements as defense-in-depth for embedded
/// markers, and (3) truncates on a UTF-8 character boundary.
fn sanitize_ref(value: &str) -> String {
    // Structural masking first: a ref that *is* a credential is redacted whole,
    // regardless of surrounding keywords.
    let masked = macaca_proto::text_sanitize::mask_secret(value);
    let keyword_scrubbed = masked
        .replace("raw_prompt", "sanitized")
        .replace("raw_provider_payload", "sanitized")
        .replace("credential", "redacted")
        .replace("secret", "redacted")
        .replace("private_key", "redacted")
        .replace("raw_signature", "redacted");
    // Character-boundary-safe truncation (never panics on multi-byte input).
    macaca_proto::text_sanitize::safe_char_prefix(&keyword_scrubbed, MAX_REF_LEN).to_string()
}

fn io_error(error: std::io::Error) -> MacacaError {
    MacacaError::Config(error.to_string())
}

fn json_error(error: serde_json::Error) -> MacacaError {
    MacacaError::Config(error.to_string())
}
