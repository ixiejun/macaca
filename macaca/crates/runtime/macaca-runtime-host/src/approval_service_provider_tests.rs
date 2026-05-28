//! Focused tests for `service.approval`.
//!
//! These tests prove the service owns approval policy, lifecycle transitions,
//! events, unavailable behavior, and sanitized audit replay.  Shell behavior is
//! represented as decision submission only; no test embeds UI or application
//! workflow semantics in the service.

use std::collections::BTreeMap;

use macaca_kernel::SystemService;
use macaca_proto::workbench::approval::*;
use macaca_proto::{
    ServiceCommand, ServiceCommandName, TraceContext, WorkbenchCommandResult,
    WorkbenchCommandStatus,
};

use crate::ApprovalSystemServiceProvider;

fn command<T: serde::Serialize>(name: &str, payload: T) -> ServiceCommand {
    ServiceCommand::with_trace(
        ServiceCommandName::new(name),
        serde_json::to_value(payload).unwrap(),
        TraceContext::new(format!("trace-{name}")),
    )
}

fn create_request() -> ApprovalRequestCreate {
    ApprovalRequestCreate {
        action_summary: "write bounded workspace file".into(),
        side_effect_class: ApprovalSideEffectClass::FileWrite,
        approval_profile_ref: None,
        reviewer_hint: None,
        reason_code: "file_write_requires_guardian".into(),
        target_refs: vec!["file://workspace/example.txt".into()],
        trace_refs: Vec::new(),
        expires_at: None,
        metadata: BTreeMap::new(),
    }
}

fn decode(value: serde_json::Value) -> WorkbenchCommandResult<ApprovalServiceResponse> {
    serde_json::from_value(value).unwrap()
}

async fn create(provider: &ApprovalSystemServiceProvider) -> ApprovalRecord {
    let result = decode(
        provider
            .call(command(REQUEST_CREATE_COMMAND, create_request()))
            .await
            .unwrap()
            .output,
    );
    assert!(matches!(
        result.status,
        WorkbenchCommandStatus::PendingApproval { .. }
    ));
    let Some(ApprovalServiceResponse::Created(record)) = result.output else {
        panic!("expected created approval record");
    };
    record
}

#[tokio::test]
async fn create_list_read_and_resolve_approval_record() {
    let provider = ApprovalSystemServiceProvider::local();
    let mut events = provider.subscribe();

    let created = create(&provider).await;
    assert_eq!(created.status, ApprovalRequestStatus::Pending);
    let pending_event = events.recv().await.unwrap();
    assert_eq!(pending_event.status, ApprovalRequestStatus::Pending);

    let listed = decode(
        provider
            .call(command(
                REQUEST_LIST_COMMAND,
                ApprovalRequestList {
                    status: Some(ApprovalRequestStatus::Pending),
                    side_effect_class: Some(ApprovalSideEffectClass::FileWrite),
                    limit: Some(10),
                },
            ))
            .await
            .unwrap()
            .output,
    );
    assert!(matches!(
        listed.output,
        Some(ApprovalServiceResponse::Listed { records, truncated: false })
            if records.len() == 1
    ));

    let read = decode(
        provider
            .call(command(
                REQUEST_READ_COMMAND,
                ApprovalRequestRead {
                    approval_ref: created.approval_ref.clone(),
                },
            ))
            .await
            .unwrap()
            .output,
    );
    assert!(matches!(
        read.output,
        Some(ApprovalServiceResponse::Read(record))
            if record.approval_ref == created.approval_ref
    ));

    let resolved = decode(
        provider
            .call(command(
                REQUEST_RESOLVE_COMMAND,
                ApprovalRequestResolve {
                    approval_ref: created.approval_ref.clone(),
                    decision: ApprovalDecision::Approve,
                    reviewer_ref: "operator://shell-submission".into(),
                    reason_code: "approved_by_guardian".into(),
                    decision_summary: Some("approved sanitized side effect".into()),
                    trace_refs: vec!["trace://shell-submit".into()],
                },
            ))
            .await
            .unwrap()
            .output,
    );
    assert!(matches!(
        resolved.output,
        Some(ApprovalServiceResponse::Resolved(record))
            if record.status == ApprovalRequestStatus::Approved
                && record.reviewer_class == "manual_guardian"
    ));
    let resolved_event = events.recv().await.unwrap();
    assert_eq!(resolved_event.status, ApprovalRequestStatus::Approved);
}

#[tokio::test]
async fn cancel_and_expire_only_transition_pending_records() {
    let provider = ApprovalSystemServiceProvider::local();
    let cancel_target = create(&provider).await;
    let cancelled = decode(
        provider
            .call(command(
                REQUEST_CANCEL_COMMAND,
                ApprovalRequestCancel {
                    approval_ref: cancel_target.approval_ref,
                    reason_code: "caller_cancelled".into(),
                },
            ))
            .await
            .unwrap()
            .output,
    );
    assert!(matches!(
        cancelled.output,
        Some(ApprovalServiceResponse::Cancelled(record))
            if record.status == ApprovalRequestStatus::Cancelled
    ));

    let mut expiring = create_request();
    expiring.expires_at = Some(chrono::Utc::now() - chrono::Duration::seconds(1));
    let created = decode(
        provider
            .call(command(REQUEST_CREATE_COMMAND, expiring))
            .await
            .unwrap()
            .output,
    );
    let Some(ApprovalServiceResponse::Created(record)) = created.output else {
        panic!("expected expiring approval");
    };
    let expired = decode(
        provider
            .call(command(
                REQUEST_EXPIRE_COMMAND,
                ApprovalRequestExpire {
                    approval_ref: Some(record.approval_ref),
                    now: Some(chrono::Utc::now()),
                },
            ))
            .await
            .unwrap()
            .output,
    );
    assert!(matches!(
        expired.output,
        Some(ApprovalServiceResponse::Expired { records }) if records.len() == 1
            && records[0].status == ApprovalRequestStatus::Expired
    ));
}

#[tokio::test]
async fn policy_explain_and_audit_are_service_owned() {
    let provider = ApprovalSystemServiceProvider::local();
    let explanation = decode(
        provider
            .call(command(
                POLICY_EXPLAIN_COMMAND,
                ApprovalPolicyExplain {
                    side_effect_class: ApprovalSideEffectClass::ProcessSpawn,
                    approval_profile_ref: Some("approval.managed".into()),
                    metadata: BTreeMap::new(),
                },
            ))
            .await
            .unwrap()
            .output,
    );
    assert!(matches!(
        explanation.output,
        Some(ApprovalServiceResponse::PolicyExplanation(policy))
            if policy.required && policy.managed_by_service
                && policy.reviewer_class == "manual_guardian"
    ));

    let created = create(&provider).await;
    let _ = decode(
        provider
            .call(command(
                REQUEST_RESOLVE_COMMAND,
                ApprovalRequestResolve {
                    approval_ref: created.approval_ref.clone(),
                    decision: ApprovalDecision::Deny,
                    reviewer_ref: "operator://shell-submission".into(),
                    reason_code: "denied_by_guardian".into(),
                    decision_summary: None,
                    trace_refs: Vec::new(),
                },
            ))
            .await
            .unwrap()
            .output,
    );
    let audit = decode(
        provider
            .call(command(
                DECISION_AUDIT_COMMAND,
                ApprovalDecisionAuditRead {
                    approval_ref: created.approval_ref,
                },
            ))
            .await
            .unwrap()
            .output,
    );
    assert!(matches!(
        audit.output,
        Some(ApprovalServiceResponse::DecisionAudit(record))
            if record.decision == ApprovalRequestStatus::Denied
                && record.action_summary == "write bounded workspace file"
    ));
}

#[tokio::test]
async fn unavailable_provider_returns_structured_unavailable() {
    let provider = ApprovalSystemServiceProvider::unavailable("approval disabled");

    let result = decode(
        provider
            .call(command(
                REQUEST_LIST_COMMAND,
                ApprovalRequestList {
                    status: None,
                    side_effect_class: None,
                    limit: None,
                },
            ))
            .await
            .unwrap()
            .output,
    );

    assert!(matches!(
        result.status,
        WorkbenchCommandStatus::Unavailable { .. }
    ));
}
