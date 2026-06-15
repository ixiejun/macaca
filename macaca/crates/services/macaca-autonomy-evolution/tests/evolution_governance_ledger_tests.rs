use std::sync::Arc;
use std::thread;

use macaca_autonomy_evolution::{
    EvolutionGovernanceAppendCommand, EvolutionGovernanceLedger,
    EvolutionGovernanceLedgerRecordKind, EvolutionGovernanceReplayCommand, EvolutionScope,
    EvolutionTargetType, LocalJsonlEvolutionGovernanceLedger,
};
use macaca_proto::TraceContext;

fn ledger_path(label: &str) -> std::path::PathBuf {
    let unique = format!(
        "macaca-evolution-ledger-{label}-{}-{}.jsonl",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    );
    std::env::temp_dir().join(unique)
}

fn scope() -> EvolutionScope {
    EvolutionScope {
        application_id: Some("app.generic".into()),
        tenant_id: Some("tenant.generic".into()),
        session_id: Some("session.generic".into()),
        task_id: Some("task.generic".into()),
    }
}

fn append_command(run_id: &str, record_ref: &str) -> EvolutionGovernanceAppendCommand {
    EvolutionGovernanceAppendCommand {
        record_id: format!("record-{run_id}-{record_ref}"),
        run_id: run_id.into(),
        node_id: "node.generic".into(),
        kind: EvolutionGovernanceLedgerRecordKind::Transition,
        target_type: Some(EvolutionTargetType::SkillPackage),
        trace: TraceContext::new(format!("trace-{run_id}")),
        scope: scope(),
        record_ref: record_ref.into(),
        evidence_refs: vec![format!("eventlog://evolution/{run_id}/{record_ref}")],
        policy_decision_refs: vec!["policy://evolution/allow".into()],
        audit_refs: vec!["audit://evolution/record".into()],
        rollback_refs: vec!["memento://evolution/rollback".into()],
    }
}

#[test]
fn local_jsonl_ledger_replays_after_restart() {
    let path = ledger_path("restart");
    let first = LocalJsonlEvolutionGovernanceLedger::open(path.clone()).unwrap();
    let appended = first
        .append(append_command("run-restart", "transition"))
        .unwrap();
    assert_eq!(appended.sequence, 1);

    let second = LocalJsonlEvolutionGovernanceLedger::open(path.clone()).unwrap();
    let replay = second
        .replay(EvolutionGovernanceReplayCommand {
            scope: EvolutionScope::default(),
            run_id: Some("run-restart".into()),
            after_sequence: None,
            limit: 10,
        })
        .unwrap();

    assert_eq!(replay.records.len(), 1);
    assert_eq!(replay.records[0].record_id, "record-run-restart-transition");
    assert!(replay.diagnostics.is_empty());
    let _ = std::fs::remove_file(path);
}

#[test]
fn replay_skips_malformed_records_with_diagnostics() {
    let path = ledger_path("malformed");
    std::fs::write(&path, "{not-json}\n").unwrap();
    let ledger = LocalJsonlEvolutionGovernanceLedger::open(path.clone()).unwrap();
    ledger
        .append(append_command("run-malformed", "valid"))
        .unwrap();

    let replay = ledger
        .replay(EvolutionGovernanceReplayCommand {
            scope: EvolutionScope::default(),
            run_id: Some("run-malformed".into()),
            after_sequence: None,
            limit: 10,
        })
        .unwrap();

    assert_eq!(replay.records.len(), 1);
    assert!(replay
        .diagnostics
        .iter()
        .any(|item| item == "malformed_record_skipped"));
    let _ = std::fs::remove_file(path);
}

#[test]
fn replay_migrates_supported_previous_schema_version() {
    let path = ledger_path("migration");
    let previous_record = serde_json::json!({
        "schema_version": 0,
        "sequence": 7,
        "record_id": "previous-record",
        "run_id": "run-migration",
        "node_id": "node.previous",
        "kind": "transition",
        "target_type": "SkillPackage",
        "trace_id": "trace-previous",
        "scope": scope(),
        "record_ref": "previous://record",
        "evidence_refs": ["eventlog://previous"],
        "policy_decision_refs": ["policy://previous"],
        "audit_refs": ["audit://previous"],
        "rollback_refs": [],
        "captured_at": chrono::Utc::now(),
    });
    std::fs::write(&path, format!("{previous_record}\n")).unwrap();

    let ledger = LocalJsonlEvolutionGovernanceLedger::open(path.clone()).unwrap();
    let replay = ledger
        .replay(EvolutionGovernanceReplayCommand {
            scope: EvolutionScope::default(),
            run_id: Some("run-migration".into()),
            after_sequence: None,
            limit: 10,
        })
        .unwrap();

    assert_eq!(replay.records.len(), 1);
    assert_eq!(replay.records[0].schema_version, 1);
    assert!(replay
        .diagnostics
        .iter()
        .any(|item| item == "schema_v0_record_migrated"));
    let _ = std::fs::remove_file(path);
}

#[test]
fn concurrent_appends_replay_in_monotonic_sequence_order() {
    let path = ledger_path("concurrent");
    let ledger = Arc::new(LocalJsonlEvolutionGovernanceLedger::open(path.clone()).unwrap());
    let mut handles = Vec::new();
    for index in 0..8 {
        let ledger = Arc::clone(&ledger);
        handles.push(thread::spawn(move || {
            ledger
                .append(append_command("run-concurrent", &format!("record-{index}")))
                .unwrap()
                .sequence
        }));
    }
    for handle in handles {
        handle.join().unwrap();
    }

    let replay = ledger
        .replay(EvolutionGovernanceReplayCommand {
            scope: EvolutionScope::default(),
            run_id: Some("run-concurrent".into()),
            after_sequence: None,
            limit: 20,
        })
        .unwrap();
    let sequences = replay
        .records
        .iter()
        .map(|record| record.sequence)
        .collect::<Vec<_>>();

    assert_eq!(sequences.len(), 8);
    assert!(sequences.windows(2).all(|pair| pair[0] < pair[1]));
    let _ = std::fs::remove_file(path);
}

#[test]
fn compaction_retains_latest_records_without_renumbering() {
    let path = ledger_path("compact");
    let ledger = LocalJsonlEvolutionGovernanceLedger::open(path.clone()).unwrap();
    for index in 0..5 {
        ledger
            .append(append_command("run-compact", &format!("record-{index}")))
            .unwrap();
    }

    let compacted = ledger.compact(2).unwrap();
    let replay = ledger
        .replay(EvolutionGovernanceReplayCommand {
            scope: EvolutionScope::default(),
            run_id: Some("run-compact".into()),
            after_sequence: None,
            limit: 10,
        })
        .unwrap();

    assert_eq!(compacted.retained_records, 2);
    assert_eq!(replay.records.len(), 2);
    assert_eq!(
        replay
            .records
            .iter()
            .map(|record| record.sequence)
            .collect::<Vec<_>>(),
        vec![4, 5]
    );
    let _ = std::fs::remove_file(path);
}

#[test]
fn snapshots_and_replay_are_sanitized_and_bounded() {
    let path = ledger_path("sanitized");
    let ledger = LocalJsonlEvolutionGovernanceLedger::open(path.clone()).unwrap();
    let mut command = append_command("run-sanitize", "raw-prompt");
    command.record_ref = "raw_prompt:secret prompt body with credential=abc".repeat(8);
    command.evidence_refs = vec!["raw_provider_payload:secret".repeat(12)];
    ledger.append(command).unwrap();

    let snapshot = ledger.snapshot().unwrap();
    let replay = ledger
        .replay(EvolutionGovernanceReplayCommand {
            scope: EvolutionScope::default(),
            run_id: Some("run-sanitize".into()),
            after_sequence: None,
            limit: 10,
        })
        .unwrap();
    let rendered = serde_json::to_string(&replay.records).unwrap();

    assert_eq!(snapshot.record_count, 1);
    assert!(!rendered.contains("raw_prompt"));
    assert!(!rendered.contains("raw_provider_payload"));
    assert!(replay.records[0].record_ref.len() <= 160);
    assert!(replay.records[0].evidence_refs[0].len() <= 160);
    let _ = std::fs::remove_file(path);
}
