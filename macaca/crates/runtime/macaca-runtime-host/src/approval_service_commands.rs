//! Command handlers for `service.approval`.
//!
//! Keeping command logic outside the provider shell keeps the runtime-host file
//! small and makes the service state machine easier to audit.  The handlers are
//! still provider-owned and do not leak approval policy into Web, CLI,
//! frontend, SDK, kernel, or application layers.

use macaca_proto::workbench::approval::*;
use macaca_proto::{
    ServiceCallResult, ServiceError, ServiceResult, TraceContext, WorkbenchCommandStatus,
};
use tracing::info;
use uuid::Uuid;

use crate::approval_service_provider::{
    ApprovalSystemServiceProvider, LIST_LIMIT_DEFAULT, LIST_LIMIT_MAX,
};

impl ApprovalSystemServiceProvider {
    pub(crate) async fn unavailable_result(
        &self,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        Self::result(
            ApprovalServiceResponse::Listed {
                records: Vec::new(),
                truncated: false,
            },
            trace,
            WorkbenchCommandStatus::Unavailable {
                reason: self
                    .unavailable_reason
                    .clone()
                    .unwrap_or_else(|| "approval provider is not configured".into()),
            },
            None,
        )
    }

    pub(crate) async fn create(
        &self,
        request: ApprovalRequestCreate,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        if request.action_summary.trim().is_empty() {
            return Err(ServiceError::InvalidArgument(
                "approval request requires sanitized action_summary".into(),
            ));
        }
        let policy = self
            .reviewer_policy()?
            .explain(ApprovalPolicyExplain {
                side_effect_class: request.side_effect_class.clone(),
                approval_profile_ref: request.approval_profile_ref.clone(),
                metadata: request.metadata.clone(),
            })
            .await?;
        let now = chrono::Utc::now();
        let approval_ref = format!("approval://{}", Uuid::new_v4());
        let audit_ref = format!("audit://approval/{}", Uuid::new_v4());
        let mut trace_refs = request.trace_refs;
        if !trace_refs.iter().any(|value| value == &trace.trace_id) {
            trace_refs.push(trace.trace_id.clone());
        }
        let record = ApprovalRecord {
            approval_ref: approval_ref.clone(),
            status: ApprovalRequestStatus::Pending,
            action_summary: request.action_summary,
            side_effect_class: request.side_effect_class,
            approval_profile_ref: policy.approval_profile_ref,
            reviewer_class: policy.reviewer_class,
            reviewer_ref: None,
            reason_code: request.reason_code,
            decision_summary: None,
            target_refs: request.target_refs,
            trace_refs,
            audit_refs: vec![audit_ref.clone()],
            created_at: now,
            expires_at: request.expires_at,
            resolved_at: None,
            updated_at: now,
        };
        let audit = audit_from_record(&audit_ref, &record, ApprovalRequestStatus::Pending);
        self.records
            .write()
            .await
            .insert(approval_ref.clone(), record.clone());
        self.audits.write().await.insert(audit_ref, audit);
        self.emit_event(&record, &trace).await;
        info!(
            approval_ref = %approval_ref,
            reviewer_class = %record.reviewer_class,
            trace_id = %trace.trace_id,
            "approval request persisted before side effect"
        );
        Self::result(
            ApprovalServiceResponse::Created(record),
            trace,
            WorkbenchCommandStatus::PendingApproval { approval_ref },
            None,
        )
    }

    pub(crate) async fn list(
        &self,
        request: ApprovalRequestList,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        let limit = request
            .limit
            .unwrap_or(LIST_LIMIT_DEFAULT)
            .clamp(1, LIST_LIMIT_MAX);
        let mut records: Vec<_> = self
            .records
            .read()
            .await
            .values()
            .filter(|record| {
                request
                    .status
                    .as_ref()
                    .map_or(true, |status| &record.status == status)
                    && request
                        .side_effect_class
                        .as_ref()
                        .map_or(true, |class| &record.side_effect_class == class)
            })
            .cloned()
            .collect();
        records.sort_by(|left, right| right.created_at.cmp(&left.created_at));
        let truncated = records.len() > limit;
        records.truncate(limit);
        Self::result(
            ApprovalServiceResponse::Listed { records, truncated },
            trace,
            WorkbenchCommandStatus::Completed,
            None,
        )
    }

    pub(crate) async fn read(
        &self,
        request: ApprovalRequestRead,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        let record = self.record(&request.approval_ref).await?;
        Self::result(
            ApprovalServiceResponse::Read(record),
            trace,
            WorkbenchCommandStatus::Completed,
            None,
        )
    }

    pub(crate) async fn resolve(
        &self,
        request: ApprovalRequestResolve,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        let mut records = self.records.write().await;
        let record = records
            .get_mut(&request.approval_ref)
            .ok_or_else(|| ServiceError::InvalidArgument("unknown approval_ref".into()))?;
        ensure_pending(record)?;
        let status = match request.decision {
            ApprovalDecision::Approve => ApprovalRequestStatus::Approved,
            ApprovalDecision::Deny => ApprovalRequestStatus::Denied,
        };
        record.status = status.clone();
        record.reviewer_ref = Some(request.reviewer_ref);
        record.reason_code = request.reason_code;
        record.decision_summary = request.decision_summary;
        append_unique_trace_refs(&mut record.trace_refs, request.trace_refs);
        append_unique_trace_refs(&mut record.trace_refs, vec![trace.trace_id.clone()]);
        record.resolved_at = Some(chrono::Utc::now());
        record.updated_at = chrono::Utc::now();
        let audit_ref = format!("audit://approval/{}", Uuid::new_v4());
        record.audit_refs.push(audit_ref.clone());
        let cloned = record.clone();
        drop(records);
        let audit = audit_from_record(&audit_ref, &cloned, status);
        self.audits.write().await.insert(audit_ref, audit);
        self.emit_event(&cloned, &trace).await;
        info!(
            approval_ref = %cloned.approval_ref,
            decision = ?cloned.status,
            trace_id = %trace.trace_id,
            "approval request resolved by service policy"
        );
        Self::result(
            ApprovalServiceResponse::Resolved(cloned),
            trace,
            WorkbenchCommandStatus::Completed,
            None,
        )
    }

    pub(crate) async fn cancel(
        &self,
        request: ApprovalRequestCancel,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        let record = self
            .transition_terminal(
                &request.approval_ref,
                ApprovalRequestStatus::Cancelled,
                request.reason_code,
                trace.clone(),
            )
            .await?;
        Self::result(
            ApprovalServiceResponse::Cancelled(record),
            trace,
            WorkbenchCommandStatus::Completed,
            None,
        )
    }

    pub(crate) async fn expire(
        &self,
        request: ApprovalRequestExpire,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        let now = request.now.unwrap_or_else(chrono::Utc::now);
        let refs: Vec<String> = {
            let records = self.records.read().await;
            records
                .values()
                .filter(|record| {
                    record.status == ApprovalRequestStatus::Pending
                        && request
                            .approval_ref
                            .as_ref()
                            .map_or(true, |value| value == &record.approval_ref)
                        && record
                            .expires_at
                            .map_or(true, |expires_at| expires_at <= now)
                })
                .map(|record| record.approval_ref.clone())
                .collect()
        };
        let mut expired = Vec::new();
        for approval_ref in refs {
            expired.push(
                self.transition_terminal(
                    &approval_ref,
                    ApprovalRequestStatus::Expired,
                    "approval_expired".into(),
                    trace.clone(),
                )
                .await?,
            );
        }
        Self::result(
            ApprovalServiceResponse::Expired { records: expired },
            trace,
            WorkbenchCommandStatus::Completed,
            None,
        )
    }

    pub(crate) async fn policy_explain(
        &self,
        request: ApprovalPolicyExplain,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        let explanation = self.reviewer_policy()?.explain(request).await?;
        Self::result(
            ApprovalServiceResponse::PolicyExplanation(explanation),
            trace,
            WorkbenchCommandStatus::Completed,
            None,
        )
    }

    pub(crate) async fn decision_audit(
        &self,
        request: ApprovalDecisionAuditRead,
        trace: TraceContext,
    ) -> ServiceResult<ServiceCallResult> {
        let record = self.record(&request.approval_ref).await?;
        let audit_ref = record
            .audit_refs
            .last()
            .ok_or_else(|| ServiceError::InvalidArgument("approval audit is missing".into()))?;
        let audit = self
            .audits
            .read()
            .await
            .get(audit_ref)
            .cloned()
            .ok_or_else(|| ServiceError::InvalidArgument("approval audit ref is missing".into()))?;
        Self::result(
            ApprovalServiceResponse::DecisionAudit(audit),
            trace,
            WorkbenchCommandStatus::Completed,
            None,
        )
    }

    pub(crate) async fn record(&self, approval_ref: &str) -> ServiceResult<ApprovalRecord> {
        self.records
            .read()
            .await
            .get(approval_ref)
            .cloned()
            .ok_or_else(|| ServiceError::InvalidArgument("unknown approval_ref".into()))
    }

    async fn transition_terminal(
        &self,
        approval_ref: &str,
        status: ApprovalRequestStatus,
        reason_code: String,
        trace: TraceContext,
    ) -> ServiceResult<ApprovalRecord> {
        let mut records = self.records.write().await;
        let record = records
            .get_mut(approval_ref)
            .ok_or_else(|| ServiceError::InvalidArgument("unknown approval_ref".into()))?;
        ensure_pending(record)?;
        record.status = status.clone();
        record.reason_code = reason_code;
        append_unique_trace_refs(&mut record.trace_refs, vec![trace.trace_id.clone()]);
        record.resolved_at = Some(chrono::Utc::now());
        record.updated_at = chrono::Utc::now();
        let audit_ref = format!("audit://approval/{}", Uuid::new_v4());
        record.audit_refs.push(audit_ref.clone());
        let cloned = record.clone();
        drop(records);
        let audit = audit_from_record(&audit_ref, &cloned, status);
        self.audits.write().await.insert(audit_ref, audit);
        self.emit_event(&cloned, &trace).await;
        Ok(cloned)
    }

    async fn emit_event(&self, record: &ApprovalRecord, trace: &TraceContext) {
        let event = ApprovalEvent {
            event_ref: format!("event://approval/{}", Uuid::new_v4()),
            approval_ref: record.approval_ref.clone(),
            status: record.status.clone(),
            side_effect_class: record.side_effect_class.clone(),
            reviewer_class: record.reviewer_class.clone(),
            reason_code: record.reason_code.clone(),
            trace_id: trace.trace_id.clone(),
            emitted_at: chrono::Utc::now(),
        };
        let _ = self.notify_tx.send(event);
    }
}

fn audit_from_record(
    audit_ref: &str,
    record: &ApprovalRecord,
    decision: ApprovalRequestStatus,
) -> ApprovalAuditRecord {
    ApprovalAuditRecord {
        audit_ref: audit_ref.into(),
        approval_ref: record.approval_ref.clone(),
        action_summary: record.action_summary.clone(),
        side_effect_class: record.side_effect_class.clone(),
        reviewer_class: record.reviewer_class.clone(),
        decision,
        reason_code: record.reason_code.clone(),
        trace_refs: record.trace_refs.clone(),
        recorded_at: chrono::Utc::now(),
    }
}

fn ensure_pending(record: &ApprovalRecord) -> ServiceResult<()> {
    if record.status != ApprovalRequestStatus::Pending {
        return Err(ServiceError::DisabledByPolicy(
            "only pending approval requests can transition".into(),
        ));
    }
    Ok(())
}

fn append_unique_trace_refs(existing: &mut Vec<String>, refs: Vec<String>) {
    for trace_ref in refs {
        if !existing.iter().any(|value| value == &trace_ref) {
            existing.push(trace_ref);
        }
    }
}

pub(crate) fn decode<T: serde::de::DeserializeOwned>(value: serde_json::Value) -> ServiceResult<T> {
    serde_json::from_value(value).map_err(|error| ServiceError::InvalidArgument(error.to_string()))
}
