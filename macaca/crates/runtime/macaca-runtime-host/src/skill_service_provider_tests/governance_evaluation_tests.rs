//! Contract tests for Skill governance read model, status telemetry, and evaluation.

use macaca_kernel::SystemService;
use macaca_proto::TraceContext;
use macaca_skill::{
    SkillEvaluationReportCommand, SkillEvaluationReportResult, SkillEvaluationScoreCommand,
    SkillEvaluationScoreResult, SkillGovernanceRecordUsageCommand, SkillGovernanceRecordUsageResult,
    SkillGovernanceSnapshotCommand, SkillGovernanceSnapshotResult, SkillStatusCommand,
    SkillAuthorKind, SkillServiceScope, SkillStatusResult, SkillUsageEventKind,
    SKILL_EVALUATION_REPORT_COMMAND,
    SKILL_EVALUATION_SCORE_COMMAND, SKILL_GOVERNANCE_RECORD_USAGE_COMMAND,
    SKILL_GOVERNANCE_SNAPSHOT_COMMAND, SKILL_STATUS_COMMAND,
};

use crate::SkillSystemServiceProvider;

use super::fixtures::{complete_evaluation_record, observation, traced_command};

#[tokio::test]
async fn skill_governance_records_usage_and_snapshots_state() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-governance-record");
    let payload = SkillGovernanceRecordUsageCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        observation: observation(SkillUsageEventKind::Used, None),
    };

    let result = provider
        .call(traced_command(
            SKILL_GOVERNANCE_RECORD_USAGE_COMMAND,
            payload,
            trace.clone(),
        ))
        .await
        .expect("usage recording should succeed");
    let typed: SkillGovernanceRecordUsageResult =
        serde_json::from_value(result.output).expect("usage result should decode");

    assert_eq!(typed.record.provenance.name, "agent-example");
    assert_eq!(typed.record.telemetry.use_count, 1);
    assert_eq!(typed.record.provenance.author_kind, SkillAuthorKind::Agent);

    let snapshot_payload = SkillGovernanceSnapshotCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        include_archived: false,
        lifecycle_filters: Vec::new(),
    };
    let snapshot = provider
        .call(traced_command(
            SKILL_GOVERNANCE_SNAPSHOT_COMMAND,
            snapshot_payload,
            trace,
        ))
        .await
        .expect("snapshot should succeed");
    let snapshot: SkillGovernanceSnapshotResult =
        serde_json::from_value(snapshot.output).expect("snapshot result should decode");
    assert_eq!(snapshot.records.len(), 1);
    assert_eq!(snapshot.records[0].telemetry.use_count, 1);
    assert_eq!(snapshot.telemetry_aggregate.record_count, 1);
    assert_eq!(snapshot.telemetry_aggregate.use_count, 1);
}

#[tokio::test]
async fn skill_status_exposes_bounded_telemetry_aggregate() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-status-telemetry");

    for event in [
        SkillUsageEventKind::SuccessfulTask,
        SkillUsageEventKind::FailedTask,
    ] {
        let payload = SkillGovernanceRecordUsageCommand {
            trace: trace.clone(),
            scope: SkillServiceScope::default(),
            observation: observation(event, None),
        };
        provider
            .call(traced_command(
                SKILL_GOVERNANCE_RECORD_USAGE_COMMAND,
                payload,
                trace.clone(),
            ))
            .await
            .expect("status aggregate source event should record");
    }

    let status = SkillStatusCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
    };
    let result = provider
        .call(traced_command(SKILL_STATUS_COMMAND, status, trace))
        .await
        .expect("status command should include bounded telemetry");
    let status: SkillStatusResult =
        serde_json::from_value(result.output).expect("status result should decode");

    assert!(status.healthy);
    assert_eq!(status.telemetry_aggregate.record_count, 1);
    assert_eq!(status.telemetry_aggregate.successful_task_count, 1);
    assert_eq!(status.telemetry_aggregate.failed_task_count, 1);
}

#[tokio::test]
async fn skill_evaluation_score_and_report_are_runtime_host_owned() {
    let provider = SkillSystemServiceProvider::new();
    let trace = TraceContext::new("trace-skill-evaluation-provider");
    let record = complete_evaluation_record();
    let score_command = SkillEvaluationScoreCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        record: record.clone(),
    };

    let score_result = provider
        .call(traced_command(
            SKILL_EVALUATION_SCORE_COMMAND,
            score_command,
            trace.clone(),
        ))
        .await
        .expect("evaluation score should succeed");
    let score_result: SkillEvaluationScoreResult =
        serde_json::from_value(score_result.output).expect("score result should decode");
    assert!(score_result.score.passed);

    let report_command = SkillEvaluationReportCommand {
        trace: trace.clone(),
        scope: SkillServiceScope::default(),
        record,
        score: score_result.score,
        include_markdown: true,
    };
    let report_result = provider
        .call(traced_command(
            SKILL_EVALUATION_REPORT_COMMAND,
            report_command,
            trace,
        ))
        .await
        .expect("evaluation report should succeed");
    let report_result: SkillEvaluationReportResult =
        serde_json::from_value(report_result.output).expect("report result should decode");

    assert!(report_result.score.passed);
    assert!(report_result
        .json_report
        .to_string()
        .contains("eval-runtime"));
    assert!(report_result
        .markdown_report
        .as_deref()
        .unwrap_or_default()
        .contains("Self-Evolution Evaluation"));
}
